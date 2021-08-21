use {
    super::errors::{HandshakeError, HandshakeErrorValue},
    //crate::utils::print,
    byteorder::{BigEndian, WriteBytesExt},
    bytes::BytesMut,
    bytesio::{
        bytes_reader::BytesReader, bytes_writer::AsyncBytesWriter, bytes_writer::BytesWriter,
        bytesio::BytesIO,
    },
    hmac::{Hmac, Mac, NewMac},
    rand,
    rand::Rng,
    sha2::Sha256,
    std::{convert::TryInto, io::Write, sync::Arc, time::SystemTime},
    tokio::sync::Mutex,
};

const RTMP_SERVER_VERSION: [u8; 4] = [0x0D, 0x0E, 0x0A, 0x0D];
const RTMP_CLIENT_VERSION: [u8; 4] = [0x0C, 0x00, 0x0D, 0x0E];

// 32
// const RTMP_KEY_SECOND_HALF: [u8; 32] = [
//     0xF0, 0xEE, 0xC2, 0x4A, 0x80, 0x68, 0xBE, 0xE8, 0x2E, 0x00, 0xD0, 0xD1, 0x02, 0x9E, 0x7E, 0x57,
//     0x6E, 0xEC, 0x5D, 0x2D, 0x29, 0x80, 0x6F, 0xAB, 0x93, 0xB8, 0xE6, 0x36, 0xCF, 0xEB, 0x31, 0xAE,
// ];
// //30
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
#[derive(Copy, Clone)]
pub enum ServerHandshakeState {
    ReadC0C1,
    WriteS0S1S2,
    ReadC2,
    Finish,
}

const RTMP_VERSION: usize = 3;
const RTMP_HANDSHAKE_SIZE: usize = 1536;

pub struct SimpleHandshakeClient {
    reader: BytesReader,
    writer: AsyncBytesWriter,
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

impl SimpleHandshakeClient {
    pub fn new(io: Arc<Mutex<BytesIO>>) -> Self {
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
        //self.writer.write_u32::<BigEndian>(current_time())?;
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
        let _ = self.reader.read_bytes(RTMP_HANDSHAKE_SIZE)?;
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
        loop {
            match self.state {
                ClientHandshakeState::WriteC0C1 => {
                    self.write_c0()?;
                    self.write_c1()?;
                    self.flush().await?;
                    self.state = ClientHandshakeState::ReadS0S1S2;
                    break;
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

                ClientHandshakeState::Finish => {
                    break;
                }
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

#[allow(dead_code)]
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
    let mut mac = Hmac::<Sha256>::new_from_slice(key).unwrap();

    mac.update(input);

    let result = mac.finalize();
    let array = result.into_bytes();

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
#[allow(dead_code)]
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

    for version in schemas {
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

pub struct ComplexHandshakeClient {
    reader: BytesReader,
    writer: AsyncBytesWriter,
    // s1_random_bytes: BytesMut,
    s1_timestamp: u32,
    // s1_version: u32,
    s1_bytes: BytesMut,

    state: ClientHandshakeState,
}

//// 1536bytes C2S2
//random-data: 1504bytes
//digest-data: 32bytes
impl ComplexHandshakeClient {
    fn write_c0(&mut self) -> Result<(), HandshakeError> {
        self.writer.write_u8(RTMP_VERSION as u8)?;
        Ok(())
    }
    fn write_c1(&mut self) -> Result<(), HandshakeError> {
        //let mut c1_bytes : vec<u8> vec[u8;RTMP_HANDSHAKE_SIZE];

        let mut c1_bytes = vec![];
        c1_bytes.write_u32::<BigEndian>(current_time())?;
        c1_bytes.write(&RTMP_CLIENT_VERSION)?;

        generate_random_bytes(&mut c1_bytes[8..RTMP_HANDSHAKE_SIZE]);

        let c1_array: [u8; RTMP_HANDSHAKE_SIZE] =
            c1_bytes.clone().try_into().unwrap_or_else(|v: Vec<u8>| {
                panic!(
                    "Expected a Vec of length {} but it was {}\n",
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
        c2_bytes.write_u32::<BigEndian>(current_time())?;
        c2_bytes.write_u32::<BigEndian>(self.s1_timestamp)?;
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
        let _ = self.reader.read_bytes(RTMP_HANDSHAKE_SIZE)?;
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

pub struct SimpleHandshakeServer {
    reader: BytesReader,
    writer: AsyncBytesWriter,
    c1_bytes: BytesMut,
    c1_timestamp: u32,
    pub state: ServerHandshakeState,
}

impl SimpleHandshakeServer {
    pub fn new(io: Arc<Mutex<BytesIO>>) -> Self {
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
        let c1_bytes = self.reader.read_bytes(RTMP_HANDSHAKE_SIZE)?;
        self.c1_bytes = c1_bytes.clone();
        let mut reader = BytesReader::new(c1_bytes);
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
        self.writer.write_u32::<BigEndian>(current_time())?;
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
                    log::info!("[ S<-C ] [simple handshake] read C0C1");
                    self.read_c0()?;
                    log::debug!("[ S<-C ] [simple handshake] read C0");
                    self.read_c1()?;
                    log::debug!("[ S<-C ] [simple handshake] read C1");
                    self.state = ServerHandshakeState::WriteS0S1S2;
                }
                ServerHandshakeState::WriteS0S1S2 => {
                    log::info!("[ S->C ] [simple handshake] write S0S1S2");
                    self.write_s0()?;
                    self.write_s1()?;
                    self.write_s2()?;
                    self.flush().await?;
                    self.state = ServerHandshakeState::ReadC2;
                    break;
                }
                ServerHandshakeState::ReadC2 => {
                    log::info!("[ S<-C ] [simple handshake] read C2");
                    self.read_c2()?;
                    self.state = ServerHandshakeState::Finish;
                }
                ServerHandshakeState::Finish => {
                    log::info!("simple handshake successfully..");
                    break;
                }
            }
        }

        Ok(())
    }

    pub fn get_remaining_bytes(&mut self) -> BytesMut {
        return self.reader.get_remaining_bytes();
    }
}

pub struct ComplexHandshakeServer {
    reader: BytesReader,
    writer: AsyncBytesWriter,

    c1_bytes: BytesMut,
    c1_schema_version: SchemaVersion,
    c1_digest: [u8; RTMP_DIGEST_LENGTH],
    c1_timestamp: u32,

    pub state: ServerHandshakeState,
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

impl ComplexHandshakeServer {
    pub fn new(io: Arc<Mutex<BytesIO>>) -> Self {
        Self {
            reader: BytesReader::new(BytesMut::new()),
            writer: AsyncBytesWriter::new(io),
            c1_bytes: BytesMut::new(),
            c1_digest: [0; RTMP_DIGEST_LENGTH],
            c1_timestamp: 0,
            state: ServerHandshakeState::ReadC0C1,
            c1_schema_version: SchemaVersion::Schema0,
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
        let mut writer = BytesWriter::new();
        writer.write_u32::<BigEndian>(current_time())?;
        writer.write(&RTMP_SERVER_VERSION)?;
        writer.write_random_bytes(RTMP_HANDSHAKE_SIZE as u32 - 8)?;

        let mut s1_array: [u8; RTMP_HANDSHAKE_SIZE] = writer.extract_current_bytes()[..]
            .try_into()
            .expect("slice with incorrect length");

        let offset = find_digest_offset(&s1_array, &self.c1_schema_version);

        let left_part = &s1_array[0..(offset as usize)];
        let right_part = &s1_array[(offset as usize + RTMP_DIGEST_LENGTH)..];

        let input = [left_part, right_part].concat();
        let digest_bytes = make_digest(&input, RTMP_SERVER_KEY_FIRST_HALF.as_bytes());

        for idx in 0..RTMP_DIGEST_LENGTH {
            s1_array[(offset as usize) + idx] = digest_bytes[idx];
        }

        self.writer.write(&s1_array)?;
        Ok(())
    }

    fn write_s2(&mut self) -> Result<(), HandshakeError> {
        let mut writer = BytesWriter::new();
        writer.write_u32::<BigEndian>(current_time())?;
        writer.write_u32::<BigEndian>(self.c1_timestamp)?;
        writer.write_random_bytes(RTMP_HANDSHAKE_SIZE as u32 - 8)?;

        let mut s2_array: [u8; RTMP_HANDSHAKE_SIZE] = writer.extract_current_bytes()[..]
            .try_into()
            .expect("slice with incorrect length");

        let tmp_key = make_digest(&self.c1_digest, &RTMP_SERVER_KEY);
        let digest = make_digest(&s2_array[..1504], &tmp_key);

        for idx in 0..RTMP_DIGEST_LENGTH {
            s2_array[RTMP_HANDSHAKE_SIZE - 32 + idx] = digest[idx];
        }

        self.writer.write(&s2_array[..])?;

        Ok(())
    }

    pub fn extend_data(&mut self, data: &[u8]) {
        self.reader.extend_from_slice(data);
    }

    pub async fn flush(&mut self) -> Result<(), HandshakeError> {
        self.writer.flush().await?;
        Ok(())
    }

    pub fn get_remaining_bytes(&mut self) -> BytesMut {
        return self.reader.get_remaining_bytes();
    }

    pub async fn handshake(&mut self) -> Result<(), HandshakeError> {
        loop {
            match self.state {
                ServerHandshakeState::ReadC0C1 => {
                    log::info!("[ S<-C ] [complex handshake] read C0C1");
                    self.read_c0()?;
                    self.read_c1()?;
                    self.state = ServerHandshakeState::WriteS0S1S2;
                }

                ServerHandshakeState::WriteS0S1S2 => {
                    log::info!("[ S->C ] [complex handshake] write S0S1S2");
                    self.write_s0()?;
                    self.write_s1()?;
                    self.write_s2()?;
                    self.flush().await?;
                    self.state = ServerHandshakeState::ReadC2;
                    break;
                }

                ServerHandshakeState::ReadC2 => {
                    log::info!("[ S<-C ] [complex handshake] read C2");
                    self.read_c2()?;
                    self.state = ServerHandshakeState::Finish;
                }

                ServerHandshakeState::Finish => {
                    log::info!("complex handshake successfully..");
                    break;
                }
            }
        }

        Ok(())
    }
}

pub struct HandshakeServer {
    simple_handshaker: SimpleHandshakeServer,
    complex_handshaker: ComplexHandshakeServer,
    is_complex: bool,
    saved_data: BytesMut,
}

impl HandshakeServer {
    pub fn new(io: Arc<Mutex<BytesIO>>) -> Self {
        Self {
            simple_handshaker: SimpleHandshakeServer::new(io.clone()),
            complex_handshaker: ComplexHandshakeServer::new(io),
            is_complex: true,
            saved_data: BytesMut::new(),
        }
    }

    pub fn extend_data(&mut self, data: &[u8]) {
        if self.is_complex {
            self.complex_handshaker.extend_data(data);
            self.saved_data.extend_from_slice(data);
        } else {
            self.simple_handshaker.extend_data(data);
        }
    }

    pub fn state(&mut self) -> ServerHandshakeState {
        if self.is_complex {
            return self.complex_handshaker.state;
        } else {
            return self.simple_handshaker.state;
        }
    }

    pub fn get_remaining_bytes(&mut self) -> BytesMut {
        match self.is_complex {
            true => self.complex_handshaker.get_remaining_bytes(),
            false => self.simple_handshaker.get_remaining_bytes(),
        }
    }

    pub async fn handshake(&mut self) -> Result<(), HandshakeError> {
        match self.is_complex {
            true => {
                let result = self.complex_handshaker.handshake().await;
                match result {
                    Ok(_) => {
                        //println!("Complex handshake is successfully!!")
                    }
                    Err(_) => {
                        log::error!("complex handshake failed..");
                        self.is_complex = false;
                        let data = self.saved_data.clone();
                        self.extend_data(&data[..]);
                        self.simple_handshaker.handshake().await?;
                    }
                }
            }
            false => {
                self.simple_handshaker.handshake().await?;
            }
        }

        // match self.state() {
        //     ServerHandshakeState::Finish => match self.is_complex {
        //         true => {
        //             log::info!("Complex handshake is successfully!!")
        //         }
        //         false => {
        //             log::info!("Simple handshake is successfully!!")
        //         }
        //     },
        //     _ => {}
        // }

        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use super::find_digest;

    #[test]
    fn test_find_digest() {
        let data: [u8; 1536] = [
            120, 70, 39, 240, //time
            0, 0, 0, 0, //version
            167, 65, 0, 0, 241, 58, 214, 16, 217, 172, 183, 96, 42, 12, 181, 58, 130, 183, 49, 68,
            200, 218, 6, 28, 216, 142, 5, 6, 254, 9, 229, 86, 67, 47, 243, 86, 77, 4, 164, 119,
            152, 152, 22, 49, 85, 60, 124, 66, 140, 18, 93, 106, 226, 219, 108, 4, 179, 212, 215,
            6, 71, 55, 205, 67, 23, 57, 232, 85, 17, 65, 252, 0, 152, 195, 19, 49, 84, 73, 142, 8,
            47, 46, 112, 53, 17, 43, 232, 87, 45, 150, 99, 75, 5, 139, 24, 119, 88, 50, 79, 108,
            245, 102, 114, 67, 107, 129, 197, 11, 214, 157, 179, 83, 136, 119, 63, 53, 7, 157, 192,
            89, 153, 100, 133, 116, 146, 180, 143, 97, 72, 15, 152, 33, 51, 81, 19, 6, 98, 238, 55,
            94, 65, 148, 3, 42, 243, 76, 250, 80, 13, 15, 210, 96, 35, 80, 218, 126, 229, 106, 195,
            46, 95, 248, 158, 31, 48, 53, 196, 125, 209, 34, 128, 92, 200, 246, 109, 96, 237, 245,
            100, 83, 97, 197, 77, 9, 12, 104, 217, 80, 75, 21, 62, 113, 2, 41, 232, 34, 53, 84,
            220, 55, 57, 128, 28, 98, 129, 80, 38, 61, 132, 100, 111, 30, 184, 37, 48, 35, 20, 101,
            252, 45, 162, 27, 80, 21, 156, 51, 70, 62, 180, 102, 230, 114, 90, 238, 96, 116, 103,
            146, 192, 7, 42, 172, 203, 115, 202, 52, 148, 64, 229, 218, 21, 66, 72, 18, 214, 40,
            233, 73, 74, 126, 197, 160, 58, 63, 241, 5, 17, 34, 176, 34, 157, 11, 196, 86, 80, 121,
            21, 156, 112, 9, 138, 43, 23, 64, 229, 145, 43, 49, 155, 106, 119, 35, 77, 248, 247,
            116, 57, 195, 206, 67, 246, 245, 114, 59, 247, 7, 114, 120, 232, 38, 105, 6, 161, 76,
            121, 97, 5, 16, 150, 98, 211, 234, 245, 105, 254, 248, 11, 16, 237, 243, 7, 2, 165, 77,
            31, 88, 213, 184, 34, 111, 243, 224, 148, 80, 217, 154, 60, 94, 228, 76, 218, 92, 91,
            60, 238, 127, 250, 187, 188, 113, 108, 84, 217, 29, 195, 141, 53, 39, 81, 17, 238, 44,
            226, 243, 178, 65, 32, 156, 169, 75, 174, 31, 73, 108, 12, 16, 191, 52, 225, 158, 182,
            107, 6, 253, 120, 34, 152, 166, 43, 53, 109, 96, 198, 68, 97, 183, 228, 59, 255, 95,
            195, 36, 52, 113, 211, 22, 161, 24, 173, 19, 30, 252, 43, 73, 25, 59, 181, 102, 253,
            26, 59, 4, 54, 218, 104, 68, 80, 50, 206, 63, 233, 66, 73, 122, 183, 13, 200, 95, 129,
            154, 252, 70, 143, 175, 3, 114, 195, 15, 251, 79, 58, 244, 199, 107, 30, 67, 115, 20,
            15, 113, 59, 27, 192, 157, 123, 91, 44, 215, 177, 16, 68, 146, 165, 11, 85, 180, 37,
            35, 122, 74, 98, 0, 200, 143, 7, 53, 171, 174, 112, 3, 80, 94, 219, 90, 201, 4, 13,
            120, 178, 101, 181, 30, 222, 152, 39, 23, 178, 34, 165, 40, 246, 232, 136, 113, 181,
            168, 118, 83, 226, 45, 62, 19, 76, 90, 54, 87, 79, 96, 98, 49, 221, 251, 160, 49, 159,
            131, 247, 63, 136, 92, 226, 18, 103, 238, 40, 75, 189, 210, 59, 108, 206, 173, 136, 75,
            31, 214, 74, 122, 242, 203, 47, 71, 97, 161, 246, 18, 0, 241, 220, 125, 142, 119, 82,
            52, 120, 38, 39, 18, 151, 153, 78, 72, 151, 173, 69, 32, 14, 165, 135, 62, 52, 90, 101,
            59, 98, 42, 5, 123, 7, 199, 33, 16, 215, 164, 145, 25, 94, 44, 221, 40, 71, 228, 139,
            80, 161, 26, 56, 16, 88, 72, 92, 83, 41, 180, 145, 79, 142, 24, 203, 102, 91, 71, 185,
            31, 162, 183, 251, 60, 245, 11, 213, 49, 98, 27, 4, 26, 70, 204, 161, 3, 134, 9, 106,
            115, 156, 119, 151, 54, 196, 190, 45, 18, 46, 63, 71, 121, 54, 35, 133, 52, 42, 201,
            202, 16, 2, 237, 92, 113, 115, 88, 204, 11, 18, 236, 194, 20, 100, 156, 24, 9, 230,
            109, 195, 46, 6, 47, 100, 32, 135, 67, 75, 17, 239, 92, 62, 100, 83, 130, 71, 58, 9,
            53, 189, 44, 209, 248, 228, 57, 8, 127, 139, 103, 83, 27, 65, 119, 79, 35, 105, 83, 81,
            66, 141, 27, 248, 225, 252, 86, 101, 141, 87, 116, 143, 29, 4, 32, 180, 167, 48, 110,
            240, 82, 82, 60, 128, 41, 195, 64, 183, 179, 217, 76, 203, 218, 165, 104, 25, 123, 191,
            96, 238, 215, 42, 59, 154, 104, 198, 121, 235, 148, 253, 80, 215, 249, 59, 56, 24, 175,
            143, 105, 204, 134, 48, 88, 79, 232, 225, 89, 162, 200, 90, 126, 124, 68, 42, 122, 140,
            103, 246, 108, 55, 80, 8, 37, 223, 93, 202, 68, 193, 254, 216, 65, 173, 82, 61, 13,
            165, 226, 254, 52, 209, 217, 209, 73, 51, 78, 51, 112, 209, 47, 83, 56, 58, 95, 104,
            93, 190, 8, 68, 111, 3, 43, 154, 91, 240, 0, 126, 102, 33, 193, 111, 90, 233, 139, 240,
            94, 177, 158, 113, 2, 183, 119, 105, 113, 140, 201, 50, 67, 203, 40, 78, 59, 216, 69,
            88, 8, 47, 108, 81, 91, 127, 176, 149, 62, 242, 129, 118, 85, 179, 98, 77, 87, 140, 2,
            139, 25, 109, 66, 84, 117, 72, 62, 213, 110, 208, 33, 236, 113, 30, 25, 160, 69, 72,
            36, 209, 17, 27, 252, 164, 57, 45, 110, 163, 126, 79, 141, 150, 27, 175, 75, 23, 57,
            113, 235, 105, 40, 113, 92, 224, 65, 128, 32, 213, 112, 95, 237, 72, 56, 215, 20, 216,
            59, 242, 77, 64, 103, 211, 127, 189, 46, 158, 13, 19, 27, 244, 13, 227, 127, 196, 79,
            169, 19, 241, 214, 179, 80, 155, 145, 228, 78, 148, 127, 19, 0, 150, 205, 28, 0, 232,
            50, 253, 98, 29, 80, 25, 93, 171, 207, 218, 40, 129, 248, 119, 53, 147, 248, 91, 87,
            179, 167, 28, 87, 115, 5, 70, 26, 126, 206, 15, 107, 27, 233, 189, 90, 39, 26, 19, 106,
            217, 44, 26, 12, 196, 96, 118, 10, 57, 233, 202, 97, 87, 207, 149, 80, 22, 130, 95, 27,
            100, 128, 85, 27, 65, 51, 96, 10, 185, 245, 196, 49, 53, 94, 224, 121, 21, 40, 73, 121,
            232, 184, 230, 49, 240, 160, 117, 34, 60, 254, 152, 88, 149, 66, 91, 36, 216, 86, 112,
            99, 232, 177, 85, 100, 206, 41, 19, 54, 30, 175, 29, 36, 24, 237, 211, 24, 100, 207,
            114, 1, 250, 174, 141, 24, 173, 171, 218, 125, 104, 43, 61, 31, 221, 200, 236, 104,
            252, 90, 119, 17, 89, 95, 246, 50, 50, 230, 237, 76, 19, 4, 164, 22, 1, 135, 7, 106, 9,
            137, 48, 20, 57, 184, 116, 126, 11, 231, 234, 32, 15, 148, 230, 29, 31, 116, 2, 13,
            229, 165, 21, 28, 202, 107, 62, 81, 113, 196, 22, 89, 104, 4, 187, 101, 5, 124, 30, 89,
            248, 89, 96, 96, 54, 218, 178, 85, 46, 59, 20, 81, 152, 114, 57, 7, 220, 92, 146, 76,
            202, 150, 14, 29, 173, 182, 205, 40, 200, 44, 148, 89, 106, 44, 8, 15, 219, 234, 155,
            97, 237, 1, 88, 67, 37, 145, 230, 70, 128, 56, 67, 75, 26, 130, 50, 42, 154, 142, 251,
            89, 157, 78, 79, 18, 207, 46, 178, 20, 166, 34, 31, 62, 38, 224, 27, 110, 67, 23, 25,
            104, 25, 101, 70, 71, 221, 112, 151, 94, 175, 237, 114, 40, 232, 144, 82, 17, 58, 120,
            167, 67, 137, 74, 198, 39, 197, 124, 71, 75, 31, 144, 80, 66, 60, 5, 38, 55, 109, 194,
            33, 33, 25, 147, 99, 47, 157, 102, 78, 50, 56, 231, 50, 57, 222, 42, 234, 60, 16, 116,
            164, 49, 230, 223, 191, 35, 96, 135, 4, 10, 195, 180, 83, 47, 123, 134, 128, 32, 232,
            5, 253, 85, 114, 238, 142, 86, 195, 170, 200, 65, 242, 9, 67, 93, 179, 30, 66, 92, 22,
            169, 237, 120, 96, 33, 248, 58, 222, 67, 87, 120, 139, 225, 40, 41, 201, 129, 239, 58,
            89, 209, 49, 66, 2, 76, 167, 84, 185, 66, 111, 51, 16, 154, 133, 77, 50, 184, 69, 126,
            98, 27, 64, 19, 205, 200, 197, 85, 185, 54, 2, 44, 65, 194, 86, 69, 247, 92, 235, 66,
            115, 126, 36, 106, 118, 230, 233, 1, 245, 77, 25, 35, 211, 33, 77, 82, 219, 205, 231,
            69, 183, 8, 124, 117, 163, 98, 32, 34, 213, 204, 43, 124, 163, 243, 148, 31, 135, 98,
            7, 108, 121, 196, 213, 88, 127, 17, 86, 58, 196, 198, 150, 104, 129, 163, 211, 2, 154,
            107, 147, 20, 3, 86, 125, 90, 94, 13, 154, 90, 202, 196, 227, 55, 112, 198, 140, 74,
            76, 9, 56, 96, 237, 141, 234, 126, 179, 3, 12, 89,
        ];

        let _ = find_digest(&data, super::RTMP_CLIENT_KEY_FIRST_HALF.as_bytes()).unwrap();
    }

    #[test]

    fn test_array() {
        let vec1: [u8; 3] = [1, 2, 3];
        let vec2: [u8; 3] = [4, 5, 6];

        // left_part: Vec::from(left_part),
        // right_part: Vec::from(right_part),

        let v1 = Vec::from(vec1);
        let v2 = Vec::from(vec2);

        let v3 = [v1, v2].concat();

        for i in v3 {
            print!("{} ", i);
        }
        print!("\n");

        let mut inputs = Vec::with_capacity(vec1.len() + vec2.len());
        for index in 0..vec1.len() {
            inputs.push(vec1[index]);
        }

        for index in 0..vec2.len() {
            inputs.push(vec2[index]);
        }

        for i in inputs {
            print!("{} ", i);
        }
        print!("\n");
    }
}
