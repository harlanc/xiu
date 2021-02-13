use super::errors::NetConnectionError;
use crate::amf0::define::Amf0ValueType;
use crate::amf0::{self, amf0_writer::Amf0Writer};
use std::collections::HashMap;

use liverust_lib::netio::writer::Writer;

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

pub struct NetConnection {
    // writer: Writer,
    amf0_writer: Amf0Writer,
}

impl NetConnection {
    pub fn new(writer: Writer) -> Self {
        Self {
            amf0_writer: Amf0Writer::new(writer),
        }
    }
    fn connect(
        &mut self,
        transaction_id: &f64,
        properties: &ConnectProperties,
    ) -> Result<(), NetConnectionError> {
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

        Ok(())
    }

    pub fn connect_reply(
        &mut self,
        transaction_id: &f64,
        fmsver: &String,
        capabilities: &f64,
        code: &String,
        level: &String,
        description: &String,
        encoding: &f64,
    ) -> Result<(), NetConnectionError> {
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

        Ok(())
    }

    pub fn create_stream(&mut self, transaction_id: &f64) -> Result<(), NetConnectionError> {
        self.amf0_writer
            .write_string(&String::from("createStream"))?;
        self.amf0_writer.write_number(transaction_id)?;
        self.amf0_writer.write_null()?;

        Ok(())
    }

    pub fn create_stream_reply(
        &mut self,
        transaction_id: &f64,
        stream_id: &f64,
    ) -> Result<(), NetConnectionError> {
        self.amf0_writer.write_string(&String::from("_result"))?;
        self.amf0_writer.write_number(transaction_id)?;
        self.amf0_writer.write_null()?;
        self.amf0_writer.write_number(stream_id)?;

        Ok(())
    }
}
