use {
    super::{
        define, define::ServerHandshakeState, digest::DigestProcessor, errors::HandshakeError,
        handshake_trait::THandshakeServer, utils,
    },
    byteorder::BigEndian,
    bytes::BytesMut,
    bytesio::{
        bytes_reader::BytesReader, bytes_writer::AsyncBytesWriter, bytes_writer::BytesWriter,
        bytesio::TNetIO,
    },
    std::sync::Arc,
    tokio::sync::Mutex,
};

pub struct SimpleHandshakeServer {
    pub reader: BytesReader,
    pub writer: AsyncBytesWriter,
    pub state: ServerHandshakeState,

    c1_bytes: BytesMut,
    c1_timestamp: u32,
}

pub struct ComplexHandshakeServer {
    pub reader: BytesReader,
    pub writer: AsyncBytesWriter,
    pub state: ServerHandshakeState,

    c1_digest: BytesMut,
    c1_timestamp: u32,
}

impl SimpleHandshakeServer {
    pub fn new(io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>) -> Self {
        Self {
            reader: BytesReader::new(BytesMut::new()),
            writer: AsyncBytesWriter::new(io),
            state: ServerHandshakeState::ReadC0C1,

            c1_bytes: BytesMut::new(),
            c1_timestamp: 0,
        }
    }
    pub fn extend_data(&mut self, data: &[u8]) {
        self.reader.extend_from_slice(data);
    }

    pub async fn handshake(&mut self) -> Result<(), HandshakeError> {
        loop {
            match self.state {
                ServerHandshakeState::ReadC0C1 => {
                    log::info!("[ S<-C ] [simple handshake] read C0C1");
                    self.read_c0()?;
                    self.read_c1()?;
                    self.state = ServerHandshakeState::WriteS0S1S2;
                }

                ServerHandshakeState::WriteS0S1S2 => {
                    log::info!("[ S->C ] [simple handshake] write S0S1S2");
                    self.write_s0()?;
                    self.write_s1()?;
                    self.write_s2()?;
                    self.writer.flush().await?;
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
}

impl ComplexHandshakeServer {
    pub fn new(io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>) -> Self {
        Self {
            reader: BytesReader::new(BytesMut::new()),
            writer: AsyncBytesWriter::new(io),
            state: ServerHandshakeState::ReadC0C1,

            c1_digest: BytesMut::new(),
            c1_timestamp: 0,
        }
    }

    pub fn extend_data(&mut self, data: &[u8]) {
        self.reader.extend_from_slice(data);
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
                    self.writer.flush().await?;
                    log::info!("[ S->C ] [complex handshake] write S0S1S2 finish");
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

impl THandshakeServer for SimpleHandshakeServer {
    fn read_c0(&mut self) -> Result<(), HandshakeError> {
        self.reader.read_u8()?;
        Ok(())
    }

    fn read_c1(&mut self) -> Result<(), HandshakeError> {
        let c1_bytes = self.reader.read_bytes(define::RTMP_HANDSHAKE_SIZE)?;
        self.c1_bytes = c1_bytes.clone();

        let mut reader = BytesReader::new(c1_bytes);
        self.c1_timestamp = reader.read_u32::<BigEndian>()?;

        Ok(())
    }

    fn read_c2(&mut self) -> Result<(), HandshakeError> {
        self.reader.read_bytes(define::RTMP_HANDSHAKE_SIZE)?;
        Ok(())
    }

    fn write_s0(&mut self) -> Result<(), HandshakeError> {
        self.writer.write_u8(define::RTMP_VERSION as u8)?;
        Ok(())
    }

    fn write_s1(&mut self) -> Result<(), HandshakeError> {
        self.writer.write_u32::<BigEndian>(utils::current_time())?;

        let timestamp = self.c1_timestamp;
        self.writer.write_u32::<BigEndian>(timestamp)?;

        self.writer
            .write_random_bytes(define::RTMP_HANDSHAKE_SIZE as u32 - 8)?;
        Ok(())
    }

    fn write_s2(&mut self) -> Result<(), HandshakeError> {
        let data = self.c1_bytes.clone();
        self.writer.write(&data[..])?;
        Ok(())
    }
}

impl THandshakeServer for ComplexHandshakeServer {
    fn read_c0(&mut self) -> Result<(), HandshakeError> {
        self.reader.read_u8()?;
        Ok(())
    }

    fn read_c1(&mut self) -> Result<(), HandshakeError> {
        let c1_bytes = self.reader.read_bytes(define::RTMP_HANDSHAKE_SIZE)?;

        /*read the timestamp*/
        self.c1_timestamp = BytesReader::new(c1_bytes.clone()).read_u32::<BigEndian>()?;

        /*read the digest and save*/
        let mut key = BytesMut::new();
        key.extend_from_slice(define::RTMP_CLIENT_KEY_FIRST_HALF.as_bytes());

        let mut digest_processor = DigestProcessor::new(c1_bytes, key);
        let (digest_content, _) = digest_processor.read_digest()?;

        self.c1_digest = digest_content;

        Ok(())
    }

    fn read_c2(&mut self) -> Result<(), HandshakeError> {
        self.reader.read_bytes(define::RTMP_HANDSHAKE_SIZE)?;
        Ok(())
    }

    fn write_s0(&mut self) -> Result<(), HandshakeError> {
        self.writer.write_u8(define::RTMP_VERSION as u8)?;
        Ok(())
    }

    fn write_s1(&mut self) -> Result<(), HandshakeError> {
        /*write the s1 data*/
        let mut writer = BytesWriter::new();

        writer.write_u32::<BigEndian>(utils::current_time())?;
        writer.write(&define::RTMP_SERVER_VERSION)?;
        writer.write_random_bytes(define::RTMP_HANDSHAKE_SIZE as u32 - 8)?;

        /*generate the digest*/
        let mut key = BytesMut::new();
        key.extend_from_slice(define::RTMP_SERVER_KEY_FIRST_HALF.as_bytes());

        let mut digest_processor = DigestProcessor::new(writer.extract_current_bytes(), key);
        let content = digest_processor.generate_and_fill_digest()?;

        /*write*/
        self.writer.write(&content[..])?;
        Ok(())
    }

    fn write_s2(&mut self) -> Result<(), HandshakeError> {
        /*write the s2 data*/
        let mut writer = BytesWriter::new();

        writer.write_u32::<BigEndian>(utils::current_time())?;
        writer.write_u32::<BigEndian>(self.c1_timestamp)?;
        writer.write_random_bytes(define::RTMP_HANDSHAKE_SIZE as u32 - 8)?;

        /*generate the key for s2*/
        let mut key = BytesMut::new();
        key.extend_from_slice(&define::RTMP_SERVER_KEY);

        let mut digest_processor = DigestProcessor::new(BytesMut::new(), key);
        let tmp_key = digest_processor.make_digest(Vec::from(&self.c1_digest[..]))?;

        /*generate the digest for s2 data*/
        let mut data: BytesMut = BytesMut::new();
        data.extend_from_slice(&writer.get_current_bytes()[..1504]);

        let mut digest_processor_2 = DigestProcessor::new(BytesMut::new(), tmp_key);
        let digtest = digest_processor_2.make_digest(Vec::from(&data[..]))?;

        let content = [data, digtest].concat();

        /*write*/
        self.writer.write(&content[..])?;

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
    pub fn new(io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>) -> Self {
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
            self.complex_handshaker.state
        } else {
            self.simple_handshaker.state
        }
    }

    pub fn get_remaining_bytes(&mut self) -> BytesMut {
        match self.is_complex {
            true => self.complex_handshaker.reader.get_remaining_bytes(),
            false => self.simple_handshaker.reader.get_remaining_bytes(),
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
                    Err(err) => {
                        log::warn!("complex handshake failed.. err:{}", err);
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

        Ok(())
    }
}
