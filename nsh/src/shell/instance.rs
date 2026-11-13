//! Per-data-directory process ownership, established before any database opens.
use std::{
    fs::{File, OpenOptions, TryLockError},
    io,
    path::Path,
};
#[cfg(unix)]
use std::{
    io::{Read, Write},
    path::PathBuf,
    time::Duration,
};
#[cfg(all(unix, any(not(target_os = "macos"), test)))]
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::JoinHandle,
};

use tokio::sync::mpsc;

use super::Event;

#[cfg(target_os = "macos")]
mod macos;
mod protocol;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(all(unix, any(not(target_os = "macos"), test)))]
use protocol::MAX_PAYLOAD;

pub struct Instance {
    _lock: File,
    #[cfg(target_os = "windows")]
    transport: windows::Server,
    #[cfg(all(unix, not(target_os = "macos")))]
    transport: UnixServer,
    #[cfg(target_os = "macos")]
    transport: macos::Server,
    #[cfg(target_os = "macos")]
    _urls: super::native::UrlCapture,
}

pub enum Launch {
    Primary(Instance),
    Forwarded,
}

pub(super) fn acquire(
    directory: &Path,
    intents: Vec<Event>,
    events: mpsc::Sender<Event>,
) -> io::Result<Launch> {
    std::fs::create_dir_all(directory)?;
    let directory = std::fs::canonicalize(directory)?;
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    let lock = options.open(directory.join("nsh-instance.lock"))?;
    match lock.try_lock() {
        Ok(()) => {}
        Err(TryLockError::WouldBlock) => {
            #[cfg(target_os = "macos")]
            {
                match macos::forward(&directory, &intents) {
                    Err(error) if error.kind() == io::ErrorKind::NotFound => {
                        // Older app versions retain the same directory lock but
                        // publish a private Unix socket instead of an endpoint.
                        forward(&socket_path(&directory)?, &intents)?;
                    }
                    result => result?,
                }
                return Ok(Launch::Forwarded);
            }
            #[cfg(all(unix, not(target_os = "macos")))]
            {
                forward(&socket_path(&directory)?, &intents)?;
                return Ok(Launch::Forwarded);
            }
            #[cfg(target_os = "windows")]
            {
                windows::forward(&directory, &intents)?;
                return Ok(Launch::Forwarded);
            }
            #[cfg(not(any(unix, target_os = "windows")))]
            {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "Mixin NSH is already using this data directory",
                ));
            }
        }
        Err(TryLockError::Error(error)) => return Err(error),
    }
    #[cfg(target_os = "macos")]
    let capture = super::native::UrlCapture::new(events.clone())?;
    #[cfg(target_os = "macos")]
    let transport = macos::Server::start(&directory, events.clone())?;
    #[cfg(all(unix, not(target_os = "macos")))]
    let transport = UnixServer::start(socket_path(&directory)?, events.clone())?;
    #[cfg(target_os = "windows")]
    let transport = windows::Server::start(&directory, events.clone())?;
    let instance = Instance {
        _lock: lock,
        #[cfg(any(unix, target_os = "windows"))]
        transport,
        #[cfg(target_os = "macos")]
        _urls: capture,
    };
    let bytes = protocol::encode(&intents)?;
    protocol::enqueue(protocol::decode(&bytes)?, &events)?;
    Ok(Launch::Primary(instance))
}

#[cfg(all(unix, any(not(target_os = "macos"), test)))]
struct UnixServer {
    socket: PathBuf,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

#[cfg(all(unix, any(not(target_os = "macos"), test)))]
impl UnixServer {
    fn start(socket: PathBuf, events: mpsc::Sender<Event>) -> io::Result<Self> {
        Self::start_observed(socket, events, || {})
    }

    fn start_observed(
        socket: PathBuf,
        events: mpsc::Sender<Event>,
        accepted: impl Fn() + Send + 'static,
    ) -> io::Result<Self> {
        use std::os::unix::{fs::PermissionsExt as _, net::UnixListener};
        match std::fs::remove_file(&socket) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        let listener = UnixListener::bind(&socket)?;
        std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(0o600))?;
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = stop.clone();
        let worker = std::thread::Builder::new()
            .name("nsh-instance".into())
            .spawn(move || {
                for stream in listener.incoming() {
                    if stopped.load(Ordering::Acquire) {
                        break;
                    }
                    let Ok(mut stream) = stream else { break };
                    accepted();
                    let deadline = std::time::Instant::now() + Duration::from_secs(2);
                    let reply = receive(
                        &mut DeadlineReader {
                            stream: &mut stream,
                            deadline,
                            stop: &stopped,
                        },
                        &events,
                    )
                    .map_or(1_u8, |()| 0);
                    if stopped.load(Ordering::Acquire) {
                        break;
                    }
                    if let Some(remaining) =
                        deadline.checked_duration_since(std::time::Instant::now())
                    {
                        let _ = stream.set_write_timeout(Some(remaining));
                        let _ = stream.write_all(&[reply]);
                    }
                }
            })?;
        Ok(Self {
            socket,
            stop,
            worker: Some(worker),
        })
    }

    fn shutdown(&mut self) {
        if self.worker.is_none() {
            return;
        }
        self.stop.store(true, Ordering::Release);
        let _ = std::os::unix::net::UnixStream::connect(&self.socket);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        let _ = std::fs::remove_file(&self.socket);
    }
}

#[cfg(all(unix, any(not(target_os = "macos"), test)))]
impl Drop for UnixServer {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(all(unix, any(not(target_os = "macos"), test)))]
struct DeadlineReader<'a> {
    stream: &'a mut std::os::unix::net::UnixStream,
    deadline: std::time::Instant,
    stop: &'a AtomicBool,
}

#[cfg(all(unix, any(not(target_os = "macos"), test)))]
impl Read for DeadlineReader<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        loop {
            if self.stop.load(Ordering::Acquire) {
                return Err(io::Error::new(
                    io::ErrorKind::ConnectionAborted,
                    "The IPC owner is stopping",
                ));
            }
            let remaining = self
                .deadline
                .checked_duration_since(std::time::Instant::now())
                .filter(|remaining| !remaining.is_zero())
                .ok_or_else(|| {
                    io::Error::new(io::ErrorKind::TimedOut, "The launch request timed out")
                })?;
            self.stream
                .set_read_timeout(Some(remaining.min(Duration::from_millis(100))))?;
            match self.stream.read(buffer) {
                Err(error)
                    if matches!(
                        error.kind(),
                        io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
                    ) => {}
                result => return result,
            }
        }
    }
}

#[cfg(unix)]
fn socket_path(directory: &Path) -> io::Result<PathBuf> {
    use std::os::unix::fs::{DirBuilderExt as _, MetadataExt as _, PermissionsExt as _};
    unsafe extern "C" {
        fn geteuid() -> u32;
    }
    // SAFETY: geteuid has no preconditions and is available on supported Unix hosts.
    let user = unsafe { geteuid() };
    let parent = std::env::temp_dir().join(format!("nsh-{user}"));
    match std::fs::DirBuilder::new().mode(0o700).create(&parent) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error),
    }
    let metadata = std::fs::symlink_metadata(&parent)?;
    if !metadata.file_type().is_dir()
        || metadata.uid() != user
        || metadata.permissions().mode() & 0o077 != 0
    {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "The application IPC directory is not private",
        ));
    }
    // FNV-1a is only a stable compact filename, not an authentication primitive.
    // The per-user directory and socket permissions restrict access.
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in directory.as_os_str().as_encoded_bytes() {
        hash = (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3);
    }
    Ok(parent.join(format!("{hash:016x}.sock")))
}

#[cfg(unix)]
fn forward(socket: &Path, intents: &[Event]) -> io::Result<()> {
    use std::os::unix::net::UnixStream;
    let bytes = protocol::encode(intents)?;
    // The owner holds its lock before publishing the socket. A second launch
    // in that short setup interval waits for bounded readiness, without opening DBs.
    let deadline = std::time::Instant::now() + Duration::from_secs(1);
    let mut stream = loop {
        match UnixStream::connect(socket) {
            Ok(stream) => break stream,
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused
                ) && std::time::Instant::now() < deadline =>
            {
                std::thread::sleep(Duration::from_millis(10))
            }
            Err(error) => return Err(error),
        }
    };
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    stream.write_all(&(bytes.len() as u32).to_be_bytes())?;
    stream.write_all(&bytes)?;
    let mut reply = [1];
    stream.read_exact(&mut reply)?;
    if reply[0] != 0 {
        return Err(io::Error::other(
            "The running application could not receive the link",
        ));
    }
    Ok(())
}

#[cfg(all(unix, any(not(target_os = "macos"), test)))]
fn receive(stream: &mut impl Read, events: &mpsc::Sender<Event>) -> io::Result<()> {
    let mut size = [0; 4];
    stream.read_exact(&mut size)?;
    let size = u32::from_be_bytes(size) as usize;
    if size > MAX_PAYLOAD {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "The link request is too large",
        ));
    }
    let mut bytes = vec![0; size];
    stream.read_exact(&mut bytes)?;
    protocol::enqueue(protocol::decode(&bytes)?, events)
}

impl Drop for Instance {
    fn drop(&mut self) {
        #[cfg(any(unix, target_os = "windows"))]
        self.transport.shutdown();
    }
}

#[cfg(all(test, unix))]
mod tests;
