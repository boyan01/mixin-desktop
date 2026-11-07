//! Same-user Windows launch handoff, independent of the native UI and database.
use std::{
    io, os::windows::io::AsRawHandle, path::Path, sync::mpsc as sync_mpsc, thread::JoinHandle,
    time::Duration,
};

use sha2::{Digest, Sha256};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::windows::named_pipe::{ClientOptions, NamedPipeServer, ServerOptions},
    sync::{mpsc, oneshot},
};
use windows::Win32::{
    Foundation::{ERROR_FILE_NOT_FOUND, ERROR_PIPE_BUSY, HANDLE},
    System::Pipes::{GetNamedPipeClientProcessId, GetNamedPipeServerProcessId},
};

use super::{Event, protocol};

#[path = "windows/security.rs"]
mod security;

const DEADLINE: Duration = Duration::from_secs(2);

pub(super) struct Server {
    stop: Option<oneshot::Sender<()>>,
    worker: Option<JoinHandle<()>>,
}

impl Server {
    pub(super) fn start(directory: &Path, events: mpsc::Sender<Event>) -> io::Result<Self> {
        let name = pipe_name(directory)?;
        let (stop, shutdown) = oneshot::channel();
        let (ready, started) = sync_mpsc::sync_channel(1);
        let worker = std::thread::Builder::new()
            .name("nsh-instance".into())
            .spawn(move || {
                let runtime = match runtime() {
                    Ok(runtime) => runtime,
                    Err(error) => {
                        let _ = ready.send(Err(error));
                        return;
                    }
                };
                runtime.block_on(async move {
                    let prepared = (|| {
                        let mut user = security::User::current()?;
                        let descriptor = security::Descriptor::for_user(&mut user)?;
                        let pipe = create(&name, &descriptor, true)?;
                        Ok::<_, io::Error>((pipe, descriptor, user))
                    })();
                    match prepared {
                        Ok((pipe, descriptor, user)) => {
                            if ready.send(Ok(())).is_ok() {
                                serve(name, pipe, descriptor, user, shutdown, events).await;
                            }
                        }
                        Err(error) => {
                            let _ = ready.send(Err(error));
                        }
                    }
                });
            })?;
        match started.recv_timeout(DEADLINE) {
            Ok(Ok(())) => Ok(Self {
                stop: Some(stop),
                worker: Some(worker),
            }),
            result => {
                let _ = stop.send(());
                let _ = worker.join();
                Err(match result {
                    Ok(Err(error)) => error,
                    _ => io::Error::new(io::ErrorKind::TimedOut, "The IPC server could not start"),
                })
            }
        }
    }

    pub(super) fn shutdown(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn runtime() -> io::Result<tokio::runtime::Runtime> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
}

fn pipe_name(directory: &Path) -> io::Result<String> {
    let mut user = security::User::current()?;
    let mut digest = Sha256::new();
    digest.update(user.name()?.as_bytes());
    digest.update([0]);
    digest.update(directory.as_os_str().as_encoded_bytes());
    let key: String = digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    Ok(format!(r"\\.\pipe\LOCAL\mixin-nsh-{key}"))
}

fn create(
    name: &str,
    descriptor: &security::Descriptor,
    first: bool,
) -> io::Result<NamedPipeServer> {
    let mut attributes = descriptor.attributes();
    // SAFETY: the descriptor and attributes remain valid for CreateNamedPipe.
    unsafe {
        ServerOptions::new()
            .first_pipe_instance(first)
            .reject_remote_clients(true)
            .create_with_security_attributes_raw(
                name,
                (&mut attributes as *mut windows::Win32::Security::SECURITY_ATTRIBUTES).cast(),
            )
    }
}

async fn serve(
    name: String,
    mut pipe: NamedPipeServer,
    descriptor: security::Descriptor,
    mut user: security::User,
    mut stop: oneshot::Receiver<()>,
    events: mpsc::Sender<Event>,
) {
    loop {
        let connected = tokio::select! {
            biased;
            _ = &mut stop => break,
            result = pipe.connect() => result,
        };
        if connected.is_err() {
            break;
        }
        // Keep one owned listening instance alive while serving the current
        // connection; a gap must not allow a different pipe owner to replace it.
        let next = match create(&name, &descriptor, false) {
            Ok(next) => next,
            Err(_) => break,
        };
        let handle = HANDLE(pipe.as_raw_handle());
        let mut process = 0;
        // SAFETY: this is a connected pipe owned by this worker.
        let peer = unsafe { GetNamedPipeClientProcessId(handle, &mut process) }
            .map_err(io::Error::other)
            .and_then(|()| user.check_process(process));
        if peer.is_ok() {
            let request = async {
                let length = pipe.read_u32().await? as usize;
                if length > protocol::MAX_PAYLOAD {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "The launch request is too large",
                    ));
                }
                let mut bytes = vec![0; length];
                pipe.read_exact(&mut bytes).await?;
                let result = protocol::decode(&bytes)
                    .and_then(|intents| protocol::enqueue(intents, &events));
                pipe.write_all(&[u8::from(result.is_err())]).await?;
                Ok::<_, io::Error>(())
            };
            tokio::select! {
                biased;
                _ = &mut stop => break,
                _ = tokio::time::timeout(DEADLINE, request) => {}
            }
        }
        drop(pipe);
        pipe = next;
    }
}

pub(super) fn forward(directory: &Path, intents: &[Event]) -> io::Result<()> {
    let name = pipe_name(directory)?;
    let bytes = protocol::encode(intents)?;
    runtime()?.block_on(async move {
        tokio::time::timeout(DEADLINE, async move {
            let mut pipe = loop {
                match ClientOptions::new().open(&name) {
                    Ok(pipe) => break pipe,
                    Err(error) if matches!(error.raw_os_error(), Some(code) if code == ERROR_PIPE_BUSY.0 as i32 || code == ERROR_FILE_NOT_FOUND.0 as i32) => {
                        tokio::time::sleep(Duration::from_millis(10)).await;
                    }
                    Err(error) => return Err(error),
                }
            };
            let handle = HANDLE(pipe.as_raw_handle());
            let mut user = security::User::current()?;
            user.check_pipe_owner(handle)?;
            let mut process = 0;
            // SAFETY: this is the connected client endpoint.
            unsafe { GetNamedPipeServerProcessId(handle, &mut process) }.map_err(io::Error::other)?;
            user.check_process(process)?;
            pipe.write_u32(bytes.len() as u32).await?;
            pipe.write_all(&bytes).await?;
            if pipe.read_u8().await? != 0 {
                return Err(io::Error::other("The running application rejected the launch request"));
            }
            Ok(())
        }).await.map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "The running application did not respond"))?
    })
}
