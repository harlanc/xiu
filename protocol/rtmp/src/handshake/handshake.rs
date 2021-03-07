use super::errors::{HandshakeError, HandshakeErrorValue};
use byteorder::{BigEndian, ByteOrder, LittleEndian, WriteBytesExt};
use bytes::BytesMut;
use hmac::{Hmac, Mac};
use rand;
use rand::Rng;
use sha2::Sha256;
use std::convert::TryInto;
use std::io::{Cursor, Write};
use std::{collections::HashMap, ops::BitOr};
use tokio_util::codec::{BytesCodec, Framed};

use netio::{
    bytes_errors::{BytesReadError, BytesWriteError},
    //bytes_reader::NetworkReader,
    bytes_reader::BytesReader,
    bytes_writer::AsyncBytesWriter,
    netio::NetworkIO,
};

use tokio::prelude::*;

use std::cell::RefCell;
use std::rc::Rc;
use std::time::{SystemTime, SystemTimeError};

const RTMP_SERVER_VERSION: [u8; 4] = [0x0D, 0x0E, 0x0A, 0x0D];
const RTMP_CLIENT_VERSION: [u8; 4] = [0x0C, 0x00, 0x0D, 0x0E];

// 32
const RTMP_KEY_SECOND_HALF: [u8; 32] = [
    0xF0, 0xEE, 0xC2, 0x4A, 0x80, 0x68, 0xBE, 0xE8, 0x2E, 0x00, 0xD0, 0xD1, 0x02, 0x9E, 0x7E, 0x57,
    0x6E, 0xEC, 0x5D, 0x2D, 0x29, 0x80, 0x6F, 0xAB, 0x93, 0xB8, 0xE6, 0x36, 0xCF, 0xEB, 0x31, 0xAE,
];
//30
const RTMP_SERVER_KEY_FIRST_HALF: &'static str = "Genuine Adobe Flash Media Server 001";
//36
const RTMP_CLIENT_KEY_FIRST_HALF: &'static str = "Genuine Adobe Flash Player 001";
const RTMP_DIGEST_LENGTH: usize = 32;

const RTMP_SERVER_KEY: [u8; 68] = [
    0x47, 0x65, 0x6e, 0x75, 0x69, 0x6e, 0x65, 0x20, 0x41, 0x64, 0x6f, 0x62, 0x65, 0x20, 0x46, 0x6c,
    0x61, 0x73, 0x68, 0x20, 0x4d, 0x65, 0x64, 0x69, 0x61, 0x20, 0x53, 0x65, 0x72, 0x76, 0x65, 0x72,
    0x20, 0x30, 0x30, 0x31, // Genuine Adobe Flash Media Server 001
    0xf0, 0xee, 0xc2, 0x4a, 0x80, 0x68, 0xbe, 0xe8, 0x2e, 0x00, 0xd0, 0xd1, 0x02, 0x9e, 0x7e, 0x57,
    0x6e, 0xec, 0x5d, 0x2d, 0x29, 0x80, 0x6f, 0xab, 0x93, 0xb8, 0xe6, 0x36, 0xcf, 0xeb, 0x31, 0xae,
]; // 68

// 62bytes FP key which is used to sign the client packet.
const RTMP_CLIENT_KEY: [u8; 62] = [
    0x47, 0x65, 0x6E, 0x75, 0x69, 0x6E, 0x65, 0x20, 0x41, 0x64, 0x6F, 0x62, 0x65, 0x20, 0x46, 0x6C,
    0x61, 0x73, 0x68, 0x20, 0x50, 0x6C, 0x61, 0x79, 0x65, 0x72, 0x20, 0x30, 0x30,
    0x31, // Genuine Adobe Flash Player 001
    0xF0, 0xEE, 0xC2, 0x4A, 0x80, 0x68, 0xBE, 0xE8, 0x2E, 0x00, 0xD0, 0xD1, 0x02, 0x9E, 0x7E, 0x57,
    0x6E, 0xEC, 0x5D, 0x2D, 0x29, 0x80, 0x6F, 0xAB, 0x93, 0xB8, 0xE6, 0x36, 0xCF, 0xEB, 0x31, 0xAE,
];

#[derive(PartialEq)]
pub enum ClientHandshakeState {
    WriteC0C1,
    ReadS0S1S2,
    WriteC2,
    Finish,
}

enum ServerHandshakeState {
    ReadC0C1,
    WriteS0S1S2,
    ReadC2,
    Finish,
}

const RTMP_VERSION: usize = 3;
const RTMP_HANDSHAKE_SIZE: usize = 1536;

pub struct SimpleHandshakeClient<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    reader: BytesReader,
    writer: AsyncBytesWriter<S>,
    s1_bytes: BytesMut,
    pub state: ClientHandshakeState,
}

fn current_time() -> u32 {
    let duration = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH);

    match duration {
        Ok(result) => result.as_nanos() as u32,
        _ => 0,
    }
}

fn generate_random_bytes(buffer: &mut [u8]) {
    let mut rng = rand::thread_rng();
    for x in 0..buffer.len() {
        let value = rng.gen();
        buffer[x] = value;
    }
}

// pub fn new(io: NetworkIO<S>) -> Self {
//     Self {
//         reader: BytesReader::new(BytesMut::new()),
//         writer: BytesWriter::new(io),
//         c1_bytes: BytesMut::new(),
//         c1_timestamp: 0,
//         state: ServerHandshakeState::ReadC0C1,
//     }
// }

impl<S> SimpleHandshakeClient<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    pub fn new(io: Rc<RefCell<NetworkIO<S>>>) -> Self {
        Self {
            reader: BytesReader::new(BytesMut::new()),
            writer: AsyncBytesWriter::new(io),
            s1_bytes: BytesMut::new(),
            state: ClientHandshakeState::WriteC0C1,
        }
    }

    fn write_c0(&mut self) -> Result<(), HandshakeError> {
        self.writer.write_u8(RTMP_VERSION as u8)?;
        Ok(())
    }
    fn write_c1(&mut self) -> Result<(), HandshakeError> {
        self.writer.write_u32::<BigEndian>(current_time())?;
        self.writer.write_u32::<BigEndian>(0)?;

        self.writer
            .write_random_bytes((RTMP_HANDSHAKE_SIZE - 8) as u32)?;
        Ok(())
    }
    fn write_c2(&mut self) -> Result<(), HandshakeError> {
        //let time = self.s1_bytes.split_to(4);
        self.writer.write(&self.s1_bytes[0..])?;
        self.writer.write_u32::<BigEndian>(current_time())?;
        Ok(())
    }

    fn read_s0(&mut self) -> Result<(), HandshakeError> {
        self.reader.read_u8()?;
        Ok(())
    }
    fn read_s1(&mut self) -> Result<(), HandshakeError> {
        self.s1_bytes = self.reader.read_bytes(RTMP_HANDSHAKE_SIZE)?;
        Ok(())
    }
    fn read_s2(&mut self) -> Result<(), HandshakeError> {
        let s2_bytes = self.reader.read_bytes(RTMP_HANDSHAKE_SIZE)?;
        Ok(())
    }

    // async fn flush_data(&mut self)-> Result<(), HandshakeError> {
    //     self.stream.send().;
    // }

    pub fn extend_data(&mut self, data: &[u8]) {
        self.reader.extend_from_slice(data);
    }
    pub async fn flush(&mut self) -> Result<(), HandshakeError> {
        self.writer.flush().await?;
        Ok(())
    }

    pub async fn handshake(&mut self) -> Result<(), HandshakeError> {
        match self.state {
            ClientHandshakeState::WriteC0C1 => {
                self.write_c0()?;
                self.write_c1()?;
                self.flush().await?;
                self.state = ClientHandshakeState::ReadS0S1S2;
            }

            ClientHandshakeState::ReadS0S1S2 => {
                self.read_s0()?;
                self.read_s1()?;
                self.read_s2()?;
                self.state = ClientHandshakeState::WriteC2;
            }

            ClientHandshakeState::WriteC2 => {
                self.write_c2()?;
                self.flush().await?;
                self.state = ClientHandshakeState::Finish;
            }

            ClientHandshakeState::Finish => {}
        }

        Ok(())
    }
}

/**************************************
// c1s1 schema0
time:4 bytes
version:4 bytes
key:764 bytes
digest:764 bytes

//c1s1 schema1
time: 4bytes
version: 4bytes
digest: 764bytes
key: 764bytes

// 764 bytes key
random-data:(offset)bytes
key-data:128bytes
random-data:(764-offset-128-4)bytes
offset:4bytes

// 764 bytes digest
offset:4bytes
random-data:(offset)bytes
digest-data:32bytes
random-data:(764-4-offset-32)bytes
****************************************/

enum SchemaVersion {
    Schema0,
    Schema1,
    Unknown,
}

struct DigestMsg {
    left_part: Vec<u8>,
    right_part: Vec<u8>,
    digest: [u8; RTMP_DIGEST_LENGTH],
}

fn make_digest(input: &[u8], key: &[u8]) -> [u8; RTMP_DIGEST_LENGTH] {
    let mut mac = Hmac::<Sha256>::new_varkey(key).unwrap();
    mac.input(&input[..]);

    let result = mac.result();
    let array = result.code();

    if array.len() != RTMP_DIGEST_LENGTH {
        panic!(
            "Expected hmac signature to be 32 byte array, instead it was a {} byte array",
            array.len()
        );
    }

    let mut output = [0_u8; 32];
    for index in 0..32 {
        output[index] = array[index];
    }

    output
}

fn find_digest_offset(data: &[u8; RTMP_HANDSHAKE_SIZE], version: &SchemaVersion) -> u32 {
    match version {
        SchemaVersion::Schema0 => {
            ((data[772] as u32) + (data[773] as u32) + (data[774] as u32) + (data[775] as u32))
                % 728
                + 776
        }
        SchemaVersion::Schema1 => {
            ((data[8] as u32) + (data[9] as u32) + (data[10] as u32) + (data[11] as u32)) % 728 + 12
        }
        SchemaVersion::Unknown => 0,
    }
}

struct DigestResult {
    digest_content: [u8; RTMP_DIGEST_LENGTH],
    version: SchemaVersion,
}

impl DigestResult {
    pub fn new(content: [u8; RTMP_DIGEST_LENGTH], ver: SchemaVersion) -> DigestResult {
        DigestResult {
            digest_content: content,
            version: ver,
        }
    }
}

// clone a slice https://stackoverflow.com/questions/28219231/how-to-idiomatically-copy-a-slice
fn find_digest(
    data: &[u8; RTMP_HANDSHAKE_SIZE],
    key: &[u8],
) -> Result<DigestResult, HandshakeError> {
    let mut schemas = Vec::new();
    schemas.push(SchemaVersion::Schema0);
    schemas.push(SchemaVersion::Schema1);

    for mut version in schemas {
        let digest_offset = find_digest_offset(&data, &version);
        let msg = cook_handshake_msg(data, digest_offset)?;
        let input = [msg.left_part, msg.right_part].concat();
        let digest = make_digest(&input, key);
        if digest == msg.digest {
            return Ok(DigestResult::new(msg.digest, version));
        }
    }

    Err(HandshakeError {
        value: HandshakeErrorValue::DigestNotFound,
    })
}

fn cook_handshake_msg(
    handshake: &[u8; RTMP_HANDSHAKE_SIZE],
    digest_offset: u32,
) -> Result<DigestMsg, HandshakeError> {
    let (left_part, rest) = handshake.split_at(digest_offset as usize);
    let (raw_digest, right_part) = rest.split_at(RTMP_DIGEST_LENGTH);

    Ok(DigestMsg {
        left_part: Vec::from(left_part),
        right_part: Vec::from(right_part),
        digest: raw_digest.try_into().expect("slice with incorrect length"),
    })
}

pub struct ComplexHandshakeClient<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    reader: BytesReader,
    writer: AsyncBytesWriter<S>,
    // s1_random_bytes: BytesMut,
    s1_timestamp: u32,
    // s1_version: u32,
    s1_bytes: BytesMut,

    state: ClientHandshakeState,
}

//// 1536bytes C2S2
//random-data: 1504bytes
//digest-data: 32bytes
impl<S> ComplexHandshakeClient<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    fn write_c0(&mut self) -> Result<(), HandshakeError> {
        self.writer.write_u8(RTMP_VERSION as u8)?;
        Ok(())
    }
    fn write_c1(&mut self) -> Result<(), HandshakeError> {
        //let mut c1_bytes : vec<u8> vec[u8;RTMP_HANDSHAKE_SIZE];

        let mut c1_bytes = vec![];
        c1_bytes.write_u32::<BigEndian>(current_time());
        c1_bytes.write(&RTMP_CLIENT_VERSION);

        generate_random_bytes(&mut c1_bytes[8..RTMP_HANDSHAKE_SIZE]);

        let c1_array: [u8; RTMP_HANDSHAKE_SIZE] =
            c1_bytes.clone().try_into().unwrap_or_else(|v: Vec<u8>| {
                panic!(
                    "Expected a Vec of length {} but it was {}",
                    RTMP_HANDSHAKE_SIZE,
                    v.len()
                )
            });

        let offset: u32 = find_digest_offset(&c1_array, &SchemaVersion::Schema1);

        let left_part = &c1_bytes[0..(offset as usize)];
        let right_part = &c1_bytes[(offset as usize + RTMP_HANDSHAKE_SIZE)..];

        let input = [left_part, right_part].concat();
        let digest_bytes = make_digest(&input, RTMP_CLIENT_KEY_FIRST_HALF.as_bytes());

        for idx in 0..RTMP_DIGEST_LENGTH {
            c1_bytes[(offset as usize) + idx] = digest_bytes[idx];
        }
        self.writer.write(&c1_bytes[..])?;
        Ok(())
    }
    fn write_c2(&mut self) -> Result<(), HandshakeError> {
        //let time = self.s1_bytes.split_to(4);

        let mut c2_bytes = vec![];
        c2_bytes.write_u32::<BigEndian>(current_time());
        c2_bytes.write_u32::<BigEndian>(self.s1_timestamp);
        generate_random_bytes(&mut c2_bytes[8..RTMP_HANDSHAKE_SIZE - 24]);

        let s1_array: [u8; RTMP_HANDSHAKE_SIZE] = self.s1_bytes[..]
            .try_into()
            .expect("slice with incorrect length");

        let result = find_digest(&s1_array, RTMP_CLIENT_KEY_FIRST_HALF.as_bytes())?;

        let tmp_key = make_digest(&result.digest_content, &RTMP_CLIENT_KEY);
        let digest = make_digest(&c2_bytes[..1504], &tmp_key);

        c2_bytes.append(&mut digest.to_vec());
        self.writer.write(&c2_bytes[..])?;

        Ok(())
    }

    fn read_s0(&mut self) -> Result<(), HandshakeError> {
        let version = self.reader.read_u8()?;
        if version != RTMP_VERSION as u8 {
            return Err(HandshakeError {
                value: HandshakeErrorValue::S0VersionNotCorrect,
            });
        }
        Ok(())
    }
    fn read_s1(&mut self) -> Result<(), HandshakeError> {
        self.s1_bytes = self.reader.read_bytes(RTMP_HANDSHAKE_SIZE)?;

        let buffer = self.s1_bytes.clone();
        let mut reader = BytesReader::new(buffer);

        //time
        self.s1_timestamp = reader.read_u32::<BigEndian>()?;
        //version
        reader.read_bytes(4)?;

        Ok(())
    }
    fn read_s2(&mut self) -> Result<(), HandshakeError> {
        let s2_bytes = self.reader.read_bytes(RTMP_HANDSHAKE_SIZE)?;
        Ok(())
    }

    pub fn handshake(&mut self) -> Result<(), HandshakeError> {
        match self.state {
            ClientHandshakeState::WriteC0C1 => {
                self.write_c0()?;
                self.write_c1()?;
                self.state = ClientHandshakeState::ReadS0S1S2;
            }

            ClientHandshakeState::ReadS0S1S2 => {
                self.read_s0()?;
                self.read_s1()?;
                self.read_s2()?;
                self.state = ClientHandshakeState::WriteC2;
            }

            ClientHandshakeState::WriteC2 => {
                self.write_c2()?;
                self.state = ClientHandshakeState::Finish;
            }

            ClientHandshakeState::Finish => {}
        }

        Ok(())
    }
}

pub struct SimpleHandshakeServer<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    reader: BytesReader,
    writer: AsyncBytesWriter<S>,
    c1_bytes: BytesMut,
    c1_timestamp: u32,
    state: ServerHandshakeState,
}

impl<S> SimpleHandshakeServer<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    pub fn new(io: Rc<RefCell<NetworkIO<S>>>) -> Self {
        Self {
            reader: BytesReader::new(BytesMut::new()),
            writer: AsyncBytesWriter::new(io),
            c1_bytes: BytesMut::new(),
            c1_timestamp: 0,
            state: ServerHandshakeState::ReadC0C1,
        }
    }
    fn read_c0(&mut self) -> Result<(), HandshakeError> {
        self.reader.read_u8()?;
        Ok(())
    }

    fn read_c1(&mut self) -> Result<(), HandshakeError> {
        self.c1_bytes = self.reader.read_bytes(RTMP_HANDSHAKE_SIZE)?;
        let buffer = self.c1_bytes.clone();
        let mut reader = BytesReader::new(buffer);
        self.c1_timestamp = reader.read_u32::<BigEndian>()?;

        Ok(())
    }

    fn read_c2(&mut self) -> Result<(), HandshakeError> {
        self.reader.read_bytes(RTMP_HANDSHAKE_SIZE)?;
        Ok(())
    }

    fn write_s0(&mut self) -> Result<(), HandshakeError> {
        self.writer.write_u8(RTMP_VERSION as u8)?;
        Ok(())
    }

    fn write_s1(&mut self) -> Result<(), HandshakeError> {
        self.writer.write_u32::<BigEndian>(current_time());
        self.writer.write_u32::<BigEndian>(self.c1_timestamp)?;
        self.writer
            .write_random_bytes(RTMP_HANDSHAKE_SIZE as u32 - 8)?;
        Ok(())
    }

    fn write_s2(&mut self) -> Result<(), HandshakeError> {
        self.writer.write(&self.c1_bytes)?;
        Ok(())
    }

    pub fn extend_data(&mut self, data: &[u8]) {
        self.reader.extend_from_slice(data);
    }

    pub async fn flush(&mut self) -> Result<(), HandshakeError> {
        self.writer.flush().await?;
        Ok(())
    }

    pub async fn handshake(&mut self) -> Result<(), HandshakeError> {
        loop {
            match self.state {
                ServerHandshakeState::ReadC0C1 => {
                    self.read_c0()?;
                    self.read_c1()?;
                    self.state = ServerHandshakeState::WriteS0S1S2;
                }

                ServerHandshakeState::WriteS0S1S2 => {
                    self.write_s0()?;
                    self.write_s1()?;
                    self.write_s2()?;
                    self.flush().await?;

                    self.state = ServerHandshakeState::ReadC2;
                }

                ServerHandshakeState::ReadC2 => {
                    self.read_c2()?;
                    self.state = ServerHandshakeState::Finish;
                }

                ServerHandshakeState::Finish => {
                    break;
                }
            }
        }

        Ok(())
    }
}

pub struct ComplexHandshakeServer<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    reader: BytesReader,
    writer: AsyncBytesWriter<S>,

    c1_bytes: BytesMut,
    c1_schema_version: SchemaVersion,
    c1_digest: [u8; RTMP_DIGEST_LENGTH],
    c1_timestamp: u32,

    state: ServerHandshakeState,
}

/**************************************
// c1s1 schema0  (1536 bytes)
time:4 bytes
version:4 bytes
key:764 bytes
digest:764 bytes

//c1s1 schema1  (1536 bytes)
time: 4bytes
version: 4bytes
digest: 764bytes
key: 764bytes

// 764 bytes key
random-data:(offset)bytes
key-data:128bytes
random-data:(764-offset-128-4)bytes
offset:4bytes

// 764 bytes digest
offset:4bytes
random-data:(offset)bytes
digest-data:32bytes
random-data:(764-4-offset-32)bytes
****************************************/

impl<S> ComplexHandshakeServer<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    fn read_c0(&mut self) -> Result<(), HandshakeError> {
        self.reader.read_u8()?;
        Ok(())
    }

    fn read_c1(&mut self) -> Result<(), HandshakeError> {
        self.c1_bytes = self.reader.read_bytes(RTMP_HANDSHAKE_SIZE)?;

        let buffer = self.c1_bytes.clone();
        let mut reader = BytesReader::new(buffer);
        self.c1_timestamp = reader.read_u32::<BigEndian>()?;

        let s1_array: [u8; RTMP_HANDSHAKE_SIZE] = self.c1_bytes[..]
            .try_into()
            .expect("slice with incorrect length");

        let result = find_digest(&s1_array, RTMP_CLIENT_KEY_FIRST_HALF.as_bytes())?;

        self.c1_digest = result.digest_content;
        Ok(())
    }

    fn read_c2(&mut self) -> Result<(), HandshakeError> {
        self.reader.read_bytes(RTMP_HANDSHAKE_SIZE)?;
        Ok(())
    }

    fn write_s0(&mut self) -> Result<(), HandshakeError> {
        self.writer.write_u8(RTMP_VERSION as u8)?;
        Ok(())
    }

    fn write_s1(&mut self) -> Result<(), HandshakeError> {
        let mut s1_bytes = vec![];
        s1_bytes.write_u32::<BigEndian>(current_time());
        s1_bytes.write(&RTMP_SERVER_VERSION);
        generate_random_bytes(&mut s1_bytes[8..RTMP_HANDSHAKE_SIZE - 24]);

        let s1_array: [u8; RTMP_HANDSHAKE_SIZE] =
            s1_bytes.clone().try_into().unwrap_or_else(|v: Vec<u8>| {
                panic!(
                    "Expected a Vec of length {} but it was {}",
                    RTMP_HANDSHAKE_SIZE,
                    v.len()
                )
            });

        let offset = find_digest_offset(&s1_array, &self.c1_schema_version);

        let left_part = &s1_bytes[0..(offset as usize)];
        let right_part = &s1_bytes[(offset as usize + RTMP_HANDSHAKE_SIZE)..];

        let input = [left_part, right_part].concat();
        let digest_bytes = make_digest(&input, RTMP_CLIENT_KEY_FIRST_HALF.as_bytes());

        for idx in 0..RTMP_DIGEST_LENGTH {
            s1_bytes[(offset as usize) + idx] = digest_bytes[idx];
        }

        self.writer.write(&s1_bytes)?;

        Ok(())
    }

    fn write_s2(&mut self) -> Result<(), HandshakeError> {
        let mut s2_bytes = vec![];
        s2_bytes.write_u32::<BigEndian>(current_time());

        s2_bytes.write_u32::<BigEndian>(self.c1_timestamp);
        generate_random_bytes(&mut s2_bytes[8..RTMP_HANDSHAKE_SIZE - 24]);

        let c1_array: [u8; RTMP_HANDSHAKE_SIZE] = self.c1_bytes[..]
            .try_into()
            .expect("slice with incorrect length");

        let result = find_digest(&c1_array, RTMP_CLIENT_KEY_FIRST_HALF.as_bytes())?;

        let tmp_key = make_digest(&result.digest_content, &RTMP_SERVER_KEY);
        let digest = make_digest(&s2_bytes[..1504], &tmp_key);

        s2_bytes.append(&mut digest.to_vec());
        self.writer.write(&s2_bytes[..])?;

        Ok(())
    }

    pub fn handshake(&mut self) -> Result<(), HandshakeError> {
        match self.state {
            ServerHandshakeState::ReadC0C1 => {
                self.read_c0()?;
                self.read_c1()?;
                self.state = ServerHandshakeState::WriteS0S1S2;
            }

            ServerHandshakeState::WriteS0S1S2 => {
                self.write_s0()?;
                self.write_s1()?;
                self.write_s2()?;
                self.state = ServerHandshakeState::ReadC2;
            }

            ServerHandshakeState::ReadC2 => {
                self.read_c2()?;
                self.state = ServerHandshakeState::Finish;
            }

            ServerHandshakeState::Finish => {}
        }

        Ok(())
    }
}
