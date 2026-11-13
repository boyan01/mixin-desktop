//! Authenticated loopback IPC avoids Darwin's short sockaddr_un path limit.
//! The directory lock still owns the process. Only its holder publishes an
//! endpoint; the random credential is readable only by the directory's user.
use super::{Event, protocol};
use hmac::{Hmac, KeyInit as _, Mac as _};
use sha2::Sha256;
use std::os::unix::fs::OpenOptionsExt as _;
use std::{
    fs::{self, OpenOptions},
    io::{self, Read, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};
use tokio::sync::mpsc;

const TOKEN_SIZE: usize = 32;
const ENDPOINT: &str = "nsh-instance.endpoint";

fn proof(token: &[u8], nonce: &[u8; TOKEN_SIZE], direction: &[u8]) -> Hmac<Sha256> {
    let mut mac = Hmac::<Sha256>::new_from_slice(token).expect("HMAC accepts any key length");
    mac.update(b"mixin-instance-v1");
    mac.update(direction);
    mac.update(nonce);
    mac
}

pub(super) struct Server {
    endpoint: PathBuf,
    address: std::net::SocketAddr,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl Server {
    pub(super) fn start(directory: &Path, events: mpsc::Sender<Event>) -> io::Result<Self> {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))?;
        let address = listener.local_addr()?;
        let mut token = [0; TOKEN_SIZE];
        std::fs::File::open("/dev/urandom")?.read_exact(&mut token)?;
        let endpoint = directory.join(ENDPOINT);
        let temporary = directory.join(format!(".nsh-instance-{}.tmp", std::process::id()));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temporary)?;
        let published = (|| {
            file.write_all(&address.port().to_be_bytes())?;
            file.write_all(&token)?;
            file.sync_all()?;
            fs::rename(&temporary, &endpoint)
        })();
        if published.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        published?;
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = stop.clone();
        let worker = std::thread::Builder::new()
            .name("mixin-instance".into())
            .spawn(move || {
                for stream in listener.incoming() {
                    if stopped.load(Ordering::Acquire) {
                        break;
                    }
                    let Ok(mut stream) = stream else { break };
                    let deadline = Instant::now() + Duration::from_secs(2);
                    let result = receive(&mut stream, &token, &events, &stopped, deadline);
                    if !stopped.load(Ordering::Acquire) {
                        let _ = stream.set_write_timeout(Some(Duration::from_millis(100)));
                        let _ = stream.write_all(&[u8::from(result.is_err())]);
                    }
                }
            });
        match worker {
            Ok(worker) => Ok(Self {
                endpoint,
                address,
                stop,
                worker: Some(worker),
            }),
            Err(error) => {
                let _ = fs::remove_file(endpoint);
                Err(error)
            }
        }
    }

    pub(super) fn shutdown(&mut self) {
        if self.worker.is_none() {
            return;
        }
        self.stop.store(true, Ordering::Release);
        let _ = TcpStream::connect(self.address);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        let _ = fs::remove_file(&self.endpoint);
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn receive(
    stream: &mut TcpStream,
    expected: &[u8; TOKEN_SIZE],
    events: &mpsc::Sender<Event>,
    stop: &AtomicBool,
    deadline: Instant,
) -> io::Result<()> {
    struct Reader<'a> {
        stream: &'a mut TcpStream,
        stop: &'a AtomicBool,
        deadline: Instant,
    }
    impl Read for Reader<'_> {
        fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
            loop {
                if self.stop.load(Ordering::Acquire) || Instant::now() >= self.deadline {
                    return Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        "IPC request expired",
                    ));
                }
                self.stream
                    .set_read_timeout(Some(Duration::from_millis(100)))?;
                match self.stream.read(bytes) {
                    Err(error)
                        if matches!(
                            error.kind(),
                            io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                        ) => {}
                    result => return result,
                }
            }
        }
    }
    let mut reader = Reader {
        stream,
        stop,
        deadline,
    };
    let mut nonce = [0; TOKEN_SIZE];
    reader.read_exact(&mut nonce)?;
    reader
        .stream
        .set_write_timeout(Some(Duration::from_millis(100)))?;
    reader
        .stream
        .write_all(&proof(expected, &nonce, b"server").finalize().into_bytes())?;
    let mut authentication = [0; TOKEN_SIZE];
    reader.read_exact(&mut authentication)?;
    proof(expected, &nonce, b"client")
        .verify_slice(&authentication)
        .map_err(|_| {
            io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Invalid instance credential",
            )
        })?;
    let mut length = [0; 4];
    reader.read_exact(&mut length)?;
    let length = u32::from_be_bytes(length) as usize;
    if length > protocol::MAX_PAYLOAD {
        return Err(io::Error::other("IPC payload too large"));
    }
    let mut payload = vec![0; length];
    reader.read_exact(&mut payload)?;
    protocol::enqueue(protocol::decode(&payload)?, events)
}

pub(super) fn forward(directory: &Path, intents: &[Event]) -> io::Result<()> {
    use std::os::unix::fs::MetadataExt as _;
    let endpoint = directory.join(ENDPOINT);
    let deadline = Instant::now() + Duration::from_secs(1);
    let mut connection = loop {
        let result = (|| {
            let meta = fs::symlink_metadata(&endpoint)?;
            let owner = fs::metadata(directory)?.uid();
            if !meta.is_file() || meta.uid() != owner || meta.mode() & 0o077 != 0 {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "Instance endpoint is not private",
                ));
            }
            if meta.len() != (TOKEN_SIZE + 2) as u64 {
                return Err(io::Error::other("Invalid instance endpoint size"));
            }
            let bytes = fs::read(&endpoint)?;
            if bytes.len() != TOKEN_SIZE + 2 {
                return Err(io::Error::other("Invalid instance endpoint"));
            }
            let port = u16::from_be_bytes([bytes[0], bytes[1]]);
            let stream = TcpStream::connect_timeout(
                &std::net::SocketAddr::from((std::net::Ipv4Addr::LOCALHOST, port)),
                Duration::from_millis(100),
            )?;
            Ok((stream, bytes))
        })();
        match result {
            Ok(connection) => break connection,
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused
                ) && Instant::now() < deadline =>
            {
                std::thread::sleep(Duration::from_millis(10))
            }
            Err(error) => return Err(error),
        }
    };
    let (stream, bytes) = &mut connection;
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    let payload = protocol::encode(intents)?;
    // Authenticate the server before sending any URL or notification target.
    // A process reusing a stale loopback port must not receive launch intents.
    let mut nonce = [0; TOKEN_SIZE];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut nonce)?;
    stream.write_all(&nonce)?;
    let mut authentication = [0; TOKEN_SIZE];
    stream.read_exact(&mut authentication)?;
    proof(&bytes[2..], &nonce, b"server")
        .verify_slice(&authentication)
        .map_err(|_| io::Error::new(io::ErrorKind::PermissionDenied, "Invalid instance server"))?;
    stream.write_all(
        &proof(&bytes[2..], &nonce, b"client")
            .finalize()
            .into_bytes(),
    )?;
    stream.write_all(&(payload.len() as u32).to_be_bytes())?;
    stream.write_all(&payload)?;
    let mut reply = [1];
    stream.read_exact(&mut reply)?;
    if reply[0] != 0 {
        return Err(io::Error::other(
            "The running instance rejected the request",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt as _;
    struct Directory(PathBuf);
    impl Directory {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir()
                .join(format!("mixin-ipc-{name}-{}", std::process::id()))
                .join("long-directory-name-".repeat(12));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Directory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(self.0.parent().unwrap());
        }
    }

    #[test]
    fn long_container_paths_forward_authenticated_intents_and_remove_the_endpoint() {
        let directory = Directory::new("long");
        let (sender, mut receiver) = mpsc::channel(8);
        let server = Server::start(&directory.0, sender).unwrap();
        forward(&directory.0, &[Event::Activate]).unwrap();
        assert_eq!(receiver.try_recv().unwrap(), Event::Activate);
        drop(server);
        assert!(!directory.0.join(ENDPOINT).exists());
    }

    #[test]
    fn an_unauthenticated_peer_cannot_enqueue_an_activation() {
        let directory = Directory::new("auth");
        let (sender, mut receiver) = mpsc::channel(8);
        let server = Server::start(&directory.0, sender).unwrap();
        let mut peer = TcpStream::connect(server.address).unwrap();
        peer.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        peer.write_all(&[0; TOKEN_SIZE]).unwrap();
        let mut server_proof = [0; TOKEN_SIZE];
        peer.read_exact(&mut server_proof).unwrap();
        peer.write_all(&[0; TOKEN_SIZE]).unwrap();
        let mut reply = [0];
        peer.read_exact(&mut reply).unwrap();
        assert_eq!(reply, [1]);
        assert!(receiver.try_recv().is_err());
        forward(&directory.0, &[Event::Activate]).unwrap();
        assert_eq!(receiver.try_recv().unwrap(), Event::Activate);
    }

    #[test]
    fn an_impostor_server_receives_no_launch_payload() {
        let directory = Directory::new("impostor");
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let mut endpoint = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(directory.0.join(ENDPOINT))
            .unwrap();
        endpoint.write_all(&port.to_be_bytes()).unwrap();
        endpoint.write_all(&[1; TOKEN_SIZE]).unwrap();
        drop(endpoint);
        let peer = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut nonce = [0; TOKEN_SIZE];
            stream.read_exact(&mut nonce).unwrap();
            stream.write_all(&[0; TOKEN_SIZE]).unwrap();
            let mut trailing = Vec::new();
            stream.read_to_end(&mut trailing).unwrap();
            trailing
        });
        assert_eq!(
            forward(
                &directory.0,
                &[Event::Open("mixin://codes/private-target".into())]
            )
            .unwrap_err()
            .kind(),
            io::ErrorKind::PermissionDenied
        );
        assert!(peer.join().unwrap().is_empty());
    }

    #[test]
    fn a_public_endpoint_is_rejected_before_connecting() {
        let directory = Directory::new("permissions");
        let (sender, mut receiver) = mpsc::channel(8);
        let _server = Server::start(&directory.0, sender).unwrap();
        fs::set_permissions(
            directory.0.join(ENDPOINT),
            fs::Permissions::from_mode(0o644),
        )
        .unwrap();
        assert_eq!(
            forward(&directory.0, &[Event::Activate])
                .unwrap_err()
                .kind(),
            io::ErrorKind::PermissionDenied
        );
        assert!(receiver.try_recv().is_err());
    }
}
