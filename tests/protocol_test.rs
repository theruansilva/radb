use bytes::Bytes;
use radb::constants::*;
use radb::keypair::AdbKeyPair;
use radb::message::AdbMessage;
use radb::message_queue::MessageQueue;
use radb::reader::AdbReader;
use radb::server::AdbDeviceDescriptor;
use radb::writer::AdbWriter;
use tokio::io::duplex;

#[test]
fn test_message_checksum_and_header() {
    let payload = Bytes::from_static(b"hello world");
    let msg = AdbMessage::new(CMD_CNXN, 0x01000000, 1048576, payload.clone());

    assert_eq!(msg.command, CMD_CNXN);
    assert_eq!(msg.arg0, 0x01000000);
    assert_eq!(msg.arg1, 1048576);
    assert_eq!(msg.payload_length, 11);
    assert_eq!(msg.checksum, 1116); // sum of "hello world" unsigned bytes
    assert_eq!(msg.magic, CMD_CNXN ^ 0xFFFF_FFFF);
}

#[tokio::test]
async fn test_reader_writer_roundtrip() {
    let (client, server) = duplex(1024);
    let mut writer = AdbWriter::new(client);
    let mut reader = AdbReader::new(server);

    let original = AdbMessage::new(CMD_OPEN, 1, 0, Bytes::from_static(b"shell,v2,raw:echo hello\0"));
    writer.write_message(&original).await.unwrap();

    let read_msg = reader.read_message().await.unwrap();
    assert_eq!(read_msg, original);
}

#[test]
fn test_keypair_generation_and_adb_pubkey_format() {
    let keypair = AdbKeyPair::generate().unwrap();
    let pubkey_bytes = &keypair.public_key_bytes;

    // Must be 524 bytes (struct) + null + " host@radb\0" = 536 bytes
    assert!(pubkey_bytes.len() >= 524);

    // First 4 bytes must be 64 (KEY_LENGTH_WORDS in u32 LE)
    let word_count = u32::from_le_bytes(pubkey_bytes[0..4].try_into().unwrap());
    assert_eq!(word_count, 64);
}

#[tokio::test]
async fn test_message_queue_routing() {
    let (client, server) = duplex(4096);
    let mut writer = AdbWriter::new(client);
    let reader = AdbReader::new(server);

    let queue = MessageQueue::new();
    queue.spawn_reader(reader);

    let mut rx0 = queue.start_listening(10).await.unwrap();
    let mut rx1 = queue.start_listening(20).await.unwrap();

    let msg0 = AdbMessage::new(CMD_WRTE, 0, 10, Bytes::from_static(b"stream 10 payload"));
    let msg1 = AdbMessage::new(CMD_WRTE, 0, 20, Bytes::from_static(b"stream 20 payload"));

    writer.write_message(&msg0).await.unwrap();
    writer.write_message(&msg1).await.unwrap();

    let r0 = rx0.recv().await.unwrap().unwrap();
    let r1 = rx1.recv().await.unwrap().unwrap();

    assert_eq!(r0.payload, Bytes::from_static(b"stream 10 payload"));
    assert_eq!(r1.payload, Bytes::from_static(b"stream 20 payload"));
}

#[test]
fn test_device_descriptor_parsing() {
    let output = "emulator-5554\tdevice\n192.168.1.50:5555\tunauthorized\n";
    let mut devices = Vec::new();
    for line in output.lines() {
        if let Some((serial, state)) = line.split_once('\t') {
            devices.push(AdbDeviceDescriptor {
                serial: serial.to_string(),
                state: state.to_string(),
            });
        }
    }

    assert_eq!(devices.len(), 2);
    assert_eq!(devices[0].serial, "emulator-5554");
    assert_eq!(devices[0].state, "device");
    assert_eq!(devices[1].serial, "192.168.1.50:5555");
    assert_eq!(devices[1].state, "unauthorized");
}
