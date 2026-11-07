use super::*;
use crate::network::{ProxyConfig, ProxyType};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

fn config(port: u16, kind: ProxyType) -> ProxyConfig {
    ProxyConfig {
        id: "fixture".into(),
        proxy_type: kind,
        host: "127.0.0.1".into(),
        port,
        username: Some("user".into()),
        password: Some("pass".into()),
    }
}

async fn http_header(stream: &mut TcpStream) -> String {
    let mut bytes = Vec::new();
    while !bytes.ends_with(b"\r\n\r\n") {
        assert!(bytes.len() < 8192);
        bytes.push(stream.read_u8().await.unwrap());
    }
    String::from_utf8(bytes).unwrap()
}

#[tokio::test]
// Tungstenite's callback fixes its unboxed HTTP error response type.
#[allow(clippy::result_large_err)]
async fn websocket_frames_follow_authenticated_http_and_remote_dns_socks_proxy_tunnels() {
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        for kind in [ProxyType::Http,ProxyType::Socks5] {
            let listener=TcpListener::bind("127.0.0.1:0").await.unwrap();let proxy=config(listener.local_addr().unwrap().port(),kind.clone());
            let server=tokio::spawn(async move {
                let(mut stream,_)=listener.accept().await.unwrap();
                if kind==ProxyType::Http {
                    let header=http_header(&mut stream).await;
                    assert!(header.starts_with("CONNECT unreachable.invalid:8765 HTTP/1.1\r\n"));
                    assert!(header.contains("Proxy-Authorization: Basic dXNlcjpwYXNz\r\n"));
                    stream.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n").await.unwrap();
                } else {
                    let mut greeting=[0;4];stream.read_exact(&mut greeting).await.unwrap();assert_eq!(greeting,[5,2,0,2]);
                    stream.write_all(&[5,2]).await.unwrap();
                    let mut auth=[0;11];stream.read_exact(&mut auth).await.unwrap();assert_eq!(&auth,b"\x01\x04user\x04pass");
                    stream.write_all(&[1,0]).await.unwrap();
                    let mut connect=[0;5];stream.read_exact(&mut connect).await.unwrap();assert_eq!(&connect[..4],&[5,1,0,3]);
                    let mut domain=vec![0;connect[4] as usize];stream.read_exact(&mut domain).await.unwrap();assert_eq!(domain,b"unreachable.invalid");
                    assert_eq!(stream.read_u16().await.unwrap(),8765);
                    stream.write_all(&[5,0,0,1,127,0,0,1,0,1]).await.unwrap();
                }
                let mut socket=tokio_tungstenite::accept_hdr_async(stream, |_: &tokio_tungstenite::tungstenite::handshake::server::Request, mut response: tokio_tungstenite::tungstenite::handshake::server::Response| {
                    response.headers_mut().insert("Sec-WebSocket-Protocol", "Mixin-Blaze-1".parse().unwrap()); Ok(response)
                }).await.unwrap();
                assert_eq!(socket.next().await.unwrap().unwrap().into_data(),b"through selected proxy".as_slice());
                socket.send(Message::Text("received".into())).await.unwrap();
            });
            let(mut sink,mut incoming)=connect(&Handshake {url:"ws://unreachable.invalid:8765/socket".into(),authorization:"Bearer fixture".into(),protocol:"Mixin-Blaze-1".into(),proxy:Some(proxy)}).await.unwrap();
            sink.send(Message::Text("through selected proxy".into())).await.unwrap();
            assert_eq!(incoming.next().await.unwrap().unwrap().into_text().unwrap(),"received");
            server.await.unwrap();
        }
    }).await.unwrap();
}

#[tokio::test]
async fn proxy_rejection_never_revokes_the_mixin_session_and_headers_remain_bounded() {
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        for status in [401, 407, 502, 200] {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let proxy = config(listener.local_addr().unwrap().port(), ProxyType::Http);
            let server = tokio::spawn(async move {
                let (mut stream, _) = listener.accept().await.unwrap();
                http_header(&mut stream).await;
                if status == 200 {
                    let _ = stream
                        .write_all(
                            format!("HTTP/1.1 200 OK\r\nX-Large: {}\r\n\r\n", "x".repeat(8192))
                                .as_bytes(),
                        )
                        .await;
                } else {
                    stream
                        .write_all(
                            format!("HTTP/1.1 {status} Rejected\r\nContent-Length: 0\r\n\r\n")
                                .as_bytes(),
                        )
                        .await
                        .unwrap();
                }
            });
            let error = match connect(&Handshake {
                url: "ws://unreachable.invalid/socket".into(),
                authorization: "Bearer fixture".into(),
                protocol: "Mixin-Blaze-1".into(),
                proxy: Some(proxy),
            })
            .await
            {
                Err(error) => error,
                Ok(_) => panic!("rejected proxy must not connect"),
            };
            assert!(!error.unauthorized);
            assert!(if status == 200 {
                error.message.contains("too large")
            } else {
                error.message.contains(&format!("HTTP {status}"))
            });
            assert!(!error.message.contains("user:pass"));
            server.await.unwrap();
        }
    })
    .await
    .unwrap();
}
