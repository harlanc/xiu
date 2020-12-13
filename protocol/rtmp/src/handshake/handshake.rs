use byteorder::{BigEndian, ByteOrder, LittleEndian, WriteBytesExt};
use bytes::BytesMut;
use hmac::{Hmac, Mac};
use rand;
use rand::Rng;
use sha2::Sha256;
use std::io::{Cursor, Write};
use std::{collections::HashMap, ops::BitOr};

use liverust_lib::netio::{
    reader::{IOReadError, Reader},
    writer::{IOWriteError, Writer},
};

const RTMP_SERVER_VERSION: [u8; 4] = [0x0D, 0x0E, 0x0A, 0x0D];
const RTMP_CLIENT_VERSION: [u8; 4] = [0x0C, 0x00, 0x0D, 0x0E];

// 32
const RTMP_KEY_SECOND_HALF: [u8; 32] = [
    0xF0, 0xEE, 0xC2, 0x4A, 0x80, 0x68, 0xBE, 0xE8, 0x2E, 0x00, 0xD0, 0xD1, 0x02, 0x9E, 0x7E, 0x57,
    0x6E, 0xEC, 0x5D, 0x2D, 0x29, 0x80, 0x6F, 0xAB, 0x93, 0xB8, 0xE6, 0x36, 0xCF, 0xEB, 0x31, 0xAE,
];
//30
const RTMP_CLIENT_KEY_FIRST_HALF: &'static str = "Genuine Adobe Flash Media Server 001";
//36
const RTMP_SERVER_KEY_FIRST_HALF: &'static str = "Genuine Adobe Flash Player 001";
const RTMP_DIGEST_LENGTH: usize = 32;

enum ClientReadState {
    ReadS0S1,
    ReadS2,
}
enum ServerReadState {
    ReadC0,
    ReadC1,
    ReadC2,
}

use std::time::{SystemTime, SystemTimeError};
const RTMP_VERSION: usize = 3;
const RTMP_HANDSHAKE_SIZE: usize = 1536;

pub enum HandshakeErrorValue {
    IORead(IOReadError),
    IOWrite(IOWriteError),
    SysTimeError(SystemTimeError),
}

pub struct HandshakeError {
    pub value: HandshakeErrorValue,
}

impl From<HandshakeErrorValue> for HandshakeError {
    fn from(val: HandshakeErrorValue) -> Self {
        HandshakeError { value: val }
    }
}

impl From<IOReadError> for HandshakeError {
    fn from(error: IOReadError) -> Self {
        HandshakeError {
            value: HandshakeErrorValue::IORead(error),
        }
    }
}

impl From<IOWriteError> for HandshakeError {
    fn from(error: IOWriteError) -> Self {
        HandshakeError {
            value: HandshakeErrorValue::IOWrite(error),
        }
    }
}

impl From<SystemTimeError> for HandshakeError {
    fn from(error: SystemTimeError) -> Self {
        HandshakeError {
            value: HandshakeErrorValue::SysTimeError(error),
        }
    }
}
pub struct SimpleHandshakeClient {
    reader: Reader,
    writer: Writer,
    s1_bytes: BytesMut,
    state: ClientReadState,
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

impl SimpleHandshakeClient {
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

    pub fn init(&mut self) -> Result<(), HandshakeError> {
        self.write_c0()?;
        self.write_c1()?;
        Ok(())
    }

    pub fn process_bytes(&mut self) -> Result<(), HandshakeError> {
        match self.state {
            ClientReadState::ReadS0S1 => {
                self.read_s0()?;
                self.read_s1()?;
                self.write_c2()?;
            }
            ClientReadState::ReadS2 => {
                self.read_s2()?;
            }
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
}

fn make_digest(left_part: &[u8], right_part: &[u8], key: &[u8]) -> [u8; RTMP_DIGEST_LENGTH] {
    // let mut inputs = Vec::with_capacity(left_part.len() + right_part.len());
    // for index in 0..left_part.len() {
    //     inputs.push(left_part[index]);
    // }

    // for index in 0..right_part.len() {
    //     inputs.push(right_part[index]);
    // }

    let mut input = [left_part, right_part].concat();
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

fn find_digest_offset(data: &[u8], version: SchemaVersion) -> u32 {
    match version {
        SchemaVersion::Schema0 => {
            ((data[772] as u32) + (data[773] as u32) + (data[774] as u32) + (data[775] as u32))
                % 728
                + 776
        }
        SchemaVersion::Schema1 => {
            ((data[8] as u32) + (data[9] as u32) + (data[10] as u32) + (data[11] as u32)) % 728 + 12
        }
    }
}

pub struct ComplexHandshakeClient {
    reader: Reader,
    writer: Writer,
    // s1_random_bytes: BytesMut,
    // s1_timestamp: u32,
    // s1_version: u32,
    s1_bytes: BytesMut,

    state: ClientReadState,
}

impl ComplexHandshakeClient {
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

        // self.writer.write_u32::<BigEndian>(current_time())?;
        // self.writer.write(&RTMP_CLIENT_VERSION)?;

        // let mut buf: [u8; RTMP_HANDSHAKE_SIZE - 8];
        // generate_random_bytes(&mut buf);

        let offset: u32 = find_digest_offset(&c1_bytes[..], SchemaVersion::Schema1);

        let left_part = &c1_bytes[0..(offset as usize)];
        let right_part = &c1_bytes[(offset as usize + RTMP_HANDSHAKE_SIZE)..];
        let digest_bytes =
            make_digest(left_part, right_part, RTMP_CLIENT_KEY_FIRST_HALF.as_bytes());

        for idx in 0..RTMP_DIGEST_LENGTH {
            c1_bytes[(offset as usize) + idx] = digest_bytes[idx];
        }
        self.writer.write(&c1_bytes[..])?;
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
}
