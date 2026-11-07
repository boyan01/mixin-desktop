use super::*;
use std::{
    pin::Pin,
    task::{Context as TaskContext, Poll},
};
use tokio::{io::ReadBuf, sync::oneshot};

const ID: &str = "10000000-0000-4000-8000-000000000004";
const KEY: [u8; KEY_SIZE] = [0x42; KEY_SIZE];

fn staging_is_empty(account: &Path) {
    let directory = account.join(".device-transfer");
    if directory.exists() {
        assert_eq!(std::fs::read_dir(directory).unwrap().count(), 0);
    }
}

async fn legacy_wire(bytes: &[u8]) -> Vec<u8> {
    let iv = [0x21_u8; IV_SIZE];
    // Existing whole-packet codec is independent of the streaming block loops.
    let encrypted = encrypt_cbc(&KEY[..32], &iv, bytes).unwrap();
    let mut body = Uuid::parse_str(ID).unwrap().as_bytes().to_vec();
    body.extend_from_slice(&iv);
    body.extend_from_slice(&encrypted);
    let mut wire = Vec::new();
    write_packet_body(&mut wire, &KEY, PACKET_FILE, &body)
        .await
        .unwrap();
    wire
}

#[test]
fn streaming_cbc_blocks_match_nist_aes256_cbc_vector() {
    // NIST SP 800-38A, F.2.5 (two blocks, without the wire's PKCS#7 block).
    // https://nvlpubs.nist.gov/nistpubs/legacy/sp/nistspecialpublication800-38a.pdf
    let key =
        hex::decode("603deb1015ca71be2b73aef0857d77811f352c073b6108d72d9810a30914dff4").unwrap();
    let iv: [u8; 16] = hex::decode("000102030405060708090a0b0c0d0e0f")
        .unwrap()
        .try_into()
        .unwrap();
    let mut data =
        hex::decode("6bc1bee22e409f96e93d7e117393172aae2d8a571e03ac9c9eb76fac45af8e51").unwrap();
    let plain = data.clone();
    let cipher = Aes256::new_from_slice(&key).unwrap();
    let mut encrypt_iv = iv;
    encrypt_blocks(&cipher, &mut encrypt_iv, &mut data);
    assert_eq!(
        hex::encode(&data),
        "f58c4c04d6e5f1ba779eabfb5f7bfbd69cfc4e967edb808d679f777bc6702c7d"
    );
    let mut decrypt_iv = iv;
    decrypt_blocks(&cipher, &mut decrypt_iv, &mut data);
    assert_eq!(data, plain);
}

#[tokio::test]
async fn streaming_files_preserve_legacy_wire_across_block_and_chunk_boundaries() {
    let account = tempfile::tempdir().unwrap();
    for length in [
        1,
        15,
        16,
        17,
        CHUNK_SIZE - 1,
        CHUNK_SIZE,
        CHUNK_SIZE + 1,
        CHUNK_SIZE * 3 + 7,
    ] {
        let bytes: Vec<u8> = (0..length).map(|i| (i % 251) as u8).collect();
        let source = account.path().join("source");
        std::fs::write(&source, &bytes).unwrap();
        let mut wire = Vec::new();
        let written = write_file_packet(&mut wire, &KEY, ID, &source)
            .await
            .unwrap();
        assert_eq!(written, wire.len());
        assert_eq!(wire[0], PACKET_FILE);
        assert_eq!(
            i32::from_be_bytes(wire[1..5].try_into().unwrap()) as usize + 5 + HMAC_SIZE,
            wire.len()
        );
        let body_end = wire.len() - HMAC_SIZE;
        let body = &wire[5..body_end];
        assert_eq!(&body[..16], Uuid::parse_str(ID).unwrap().as_bytes());
        let mut mac = <Hmac<Sha256> as HmacKeyInit>::new_from_slice(&KEY[32..]).unwrap();
        mac.update(body);
        mac.verify_slice(&wire[body_end..]).unwrap();
        assert_eq!(
            decrypt_cbc(&KEY[..32], &body[16..32], &body[32..]).unwrap(),
            bytes
        );

        // Conversely, accept a packet produced by the original whole-body codec.
        let wire = legacy_wire(&bytes).await;
        let packet = read_packet(&mut wire.as_slice(), &KEY, account.path())
            .await
            .unwrap();
        let Packet::File {
            message_id,
            file,
            size,
        } = packet
        else {
            panic!("expected authenticated file");
        };
        assert_eq!(message_id, ID);
        assert_eq!(size, wire.len());
        assert_eq!(std::fs::read(file.path()).unwrap(), bytes);
        drop(file);
        staging_is_empty(account.path());
    }
}

#[tokio::test]
async fn corrupted_or_truncated_file_packets_release_their_unpublished_staging_file() {
    for corruption in ["hmac", "truncated-body", "truncated-hmac", "padding"] {
        let account = tempfile::tempdir().unwrap();
        let mut wire = legacy_wire(&vec![7_u8; CHUNK_SIZE * 2 + 3]).await;
        match corruption {
            "hmac" => *wire.last_mut().unwrap() ^= 1,
            "truncated-body" => wire.truncate(wire.len() / 2),
            "truncated-hmac" => wire.truncate(wire.len() - 10),
            "padding" => {
                // A valid MAC does not make invalid PKCS#7 padding acceptable.
                let mut body = Uuid::parse_str(ID).unwrap().as_bytes().to_vec();
                let iv = [0x21_u8; 16];
                body.extend_from_slice(&iv);
                let mut invalid = [0_u8; 16];
                let mut previous = iv;
                encrypt_blocks(
                    &Aes256::new_from_slice(&KEY[..32]).unwrap(),
                    &mut previous,
                    &mut invalid,
                );
                body.extend_from_slice(&invalid);
                wire.clear();
                write_packet_body(&mut wire, &KEY, PACKET_FILE, &body)
                    .await
                    .unwrap();
            }
            _ => unreachable!(),
        }
        assert!(
            read_packet(&mut wire.as_slice(), &KEY, account.path())
                .await
                .is_err(),
            "{corruption}"
        );
        staging_is_empty(account.path());
        assert!(!account.path().join("Media").exists());
    }
}

/// Stops after the encrypted body, so the test can cancel while authentication
/// is still pending without relying on time or native I/O scheduling.
struct AwaitingMac {
    wire: Vec<u8>,
    position: usize,
    stop_at: usize,
    reached: Option<oneshot::Sender<()>>,
}
impl AsyncRead for AwaitingMac {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _: &mut TaskContext<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        if self.position == self.stop_at {
            if let Some(reached) = self.reached.take() {
                let _ = reached.send(());
            }
            return Poll::Pending;
        }
        let count = buffer.remaining().min(self.stop_at - self.position);
        buffer.put_slice(&self.wire[self.position..self.position + count]);
        self.position += count;
        Poll::Ready(Ok(()))
    }
}

#[tokio::test]
async fn cancelling_a_file_before_its_mac_arrives_removes_partial_plaintext() {
    let account = tempfile::tempdir().unwrap();
    let path = account.path().to_owned();
    let wire = legacy_wire(&vec![9_u8; CHUNK_SIZE * 2 + 1]).await;
    let stop_at = wire.len() - HMAC_SIZE;
    let (send, receive) = oneshot::channel();
    let task = tokio::spawn(async move {
        let mut reader = AwaitingMac {
            wire,
            position: 0,
            stop_at,
            reached: Some(send),
        };
        read_packet(&mut reader, &KEY, &path).await
    });
    tokio::time::timeout(std::time::Duration::from_secs(3), receive)
        .await
        .unwrap()
        .unwrap();
    task.abort();
    match task.await {
        Err(error) => assert!(error.is_cancelled()),
        Ok(_) => panic!("cancelled receiver completed unexpectedly"),
    }
    staging_is_empty(account.path());
    assert!(!account.path().join("Media").exists());
}

#[tokio::test]
async fn invalid_control_headers_fail_before_reading_an_advertised_large_body() {
    let account = tempfile::tempdir().unwrap();
    for (kind, length) in [
        (PACKET_JSON, JSON_PACKET_LIMIT as i32 + 64),
        (PACKET_COMMAND, -1),
        (255, 1),
        (PACKET_FILE, 47),
    ] {
        let mut wire = vec![kind];
        wire.extend_from_slice(&length.to_be_bytes());
        let error = read_packet(&mut wire.as_slice(), &KEY, account.path())
            .await
            .err()
            .unwrap();
        assert!(!error.to_string().contains("early eof"));
        staging_is_empty(account.path());
    }
}
