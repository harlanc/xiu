use {
    super::errors::NetConnectionError,
    crate::amf0::{amf0_writer::Amf0Writer, define::Amf0ValueType},
    bytes::BytesMut,
    netio::bytes_writer::BytesWriter,
    std::collections::HashMap,
};

pub struct ConnectProperties {
    app: String,         // Server application name, e.g.: testapp
    flash_ver: String,   // Flash Player version, FMSc/1.0
    swf_url: String,     // URL of the source SWF file file://C:/FlvPlayer.swf
    tc_url: String,      // URL of the Server, rtmp://host:1935/testapp/instance1
    fpad: bool,          // True if proxy is being used.
    capabilities: f64,   // double default: 15
    audio_codecs: f64,   // double default: 4071
    video_codecs: f64,   // double default: 252
    video_function: f64, // double default: 1
    object_encoding: f64,
    page_url: String, // http://host/sample.html
}

impl ConnectProperties {
    pub fn new(app_name: String) -> Self {
        Self {
            app: app_name,
            flash_ver: "LNX 9,0,124,2".to_string(),
            swf_url: "".to_string(),
            tc_url: "".to_string(),
            fpad: false,
            capabilities: 15_f64,
            audio_codecs: 4071_f64,
            video_codecs: 252_f64,
            video_function: 1_f64,
            object_encoding: 0_f64,
            page_url: "".to_string(),
        }
    }
}

pub struct NetConnection {
    // writer: BytesWriter,
    amf0_writer: Amf0Writer,
}

impl NetConnection {
    pub fn new(writer: BytesWriter) -> Self {
        Self {
            amf0_writer: Amf0Writer::new(writer),
        }
    }
    pub fn connect(
        &mut self,
        transaction_id: &f64,
        properties: &ConnectProperties,
    ) -> Result<BytesMut, NetConnectionError> {
        self.amf0_writer.write_string(&String::from("connect"))?;
        self.amf0_writer.write_number(transaction_id)?;

        let mut properties_map = HashMap::new();
        properties_map.insert(
            String::from("app"),
            Amf0ValueType::UTF8String(properties.app.clone()),
        );
        properties_map.insert(
            String::from("flashVer"),
            Amf0ValueType::UTF8String(properties.flash_ver.clone()),
        );

        properties_map.insert(
            String::from("tcUrl"),
            Amf0ValueType::UTF8String(properties.tc_url.clone()),
        );
        properties_map.insert(
            String::from("swfUrl"),
            Amf0ValueType::UTF8String(properties.swf_url.clone()),
        );
        properties_map.insert(
            String::from("pageUrl"),
            Amf0ValueType::UTF8String(properties.page_url.clone()),
        );

        properties_map.insert(
            String::from("fpab"),
            Amf0ValueType::Boolean(properties.fpad),
        );
        properties_map.insert(
            String::from("capabilities"),
            Amf0ValueType::Number(properties.capabilities),
        );
        properties_map.insert(
            String::from("audioCodecs"),
            Amf0ValueType::Number(properties.audio_codecs),
        );
        properties_map.insert(
            String::from("videoCodecs"),
            Amf0ValueType::Number(properties.video_codecs),
        );
        properties_map.insert(
            String::from("videoFunction"),
            Amf0ValueType::Number(properties.video_function),
        );
        properties_map.insert(
            String::from("objectEncoding"),
            Amf0ValueType::Number(properties.object_encoding),
        );

        self.amf0_writer.write_object(&properties_map)?;

        return Ok(self.amf0_writer.extract_current_bytes());
    }

    pub fn connect_response(
        &mut self,
        transaction_id: &f64,
        fmsver: &String,
        capabilities: &f64,
        code: &String,
        level: &String,
        description: &String,
        encoding: &f64,
    ) -> Result<BytesMut, NetConnectionError> {
        self.amf0_writer.write_string(&String::from("_result"))?;
        self.amf0_writer.write_number(transaction_id)?;

        let mut properties_map_a = HashMap::new();

        properties_map_a.insert(
            String::from("fmsVer"),
            Amf0ValueType::UTF8String(fmsver.clone()),
        );
        properties_map_a.insert(
            String::from("capabilities"),
            Amf0ValueType::Number(capabilities.clone()),
        );

        self.amf0_writer.write_object(&properties_map_a)?;

        let mut properties_map_b = HashMap::new();

        properties_map_b.insert(
            String::from("level"),
            Amf0ValueType::UTF8String(level.clone()),
        );
        properties_map_b.insert(
            String::from("code"),
            Amf0ValueType::UTF8String(code.clone()),
        );
        properties_map_b.insert(
            String::from("description"),
            Amf0ValueType::UTF8String(description.clone()),
        );
        properties_map_b.insert(
            String::from("objectEncoding"),
            Amf0ValueType::Number(encoding.clone()),
        );

        self.amf0_writer.write_object(&properties_map_b)?;

        return Ok(self.amf0_writer.extract_current_bytes());
    }

    pub fn create_stream(&mut self, transaction_id: &f64) -> Result<BytesMut, NetConnectionError> {
        self.amf0_writer
            .write_string(&String::from("createStream"))?;
        self.amf0_writer.write_number(transaction_id)?;
        self.amf0_writer.write_null()?;

        return Ok(self.amf0_writer.extract_current_bytes());
    }

    pub fn create_stream_response(
        &mut self,
        transaction_id: &f64,
        stream_id: &f64,
    ) -> Result<BytesMut, NetConnectionError> {
        self.amf0_writer.write_string(&String::from("_result"))?;
        self.amf0_writer.write_number(transaction_id)?;
        self.amf0_writer.write_null()?;
        self.amf0_writer.write_number(stream_id)?;

        return Ok(self.amf0_writer.extract_current_bytes());
    }

    pub fn error(
        &mut self,
        transaction_id: &f64,
        code: &String,
        level: &String,
        description: &String,
    ) -> Result<BytesMut, NetConnectionError> {
        self.amf0_writer.write_string(&String::from("_error"))?;
        self.amf0_writer.write_number(transaction_id)?;
        self.amf0_writer.write_null()?;

        let mut properties_map = HashMap::new();

        properties_map.insert(
            String::from("level"),
            Amf0ValueType::UTF8String(level.clone()),
        );
        properties_map.insert(
            String::from("code"),
            Amf0ValueType::UTF8String(code.clone()),
        );
        properties_map.insert(
            String::from("description"),
            Amf0ValueType::UTF8String(description.clone()),
        );
        self.amf0_writer.write_object(&properties_map)?;

        return Ok(self.amf0_writer.extract_current_bytes());
    }
}
