use super::errors::NetStreamError;
use crate::amf0::amf0_writer::Amf0Writer;
use crate::amf0::define::Amf0ValueType;
use liverust_lib::netio::writer::Writer;
use std::collections::HashMap;
pub struct NetStream {
    writer: Writer,
    amf0_writer: Amf0Writer,
}

impl NetStream {
    fn play(
        &mut self,
        transaction_id: &f64,
        stream_name: &String,
        start: &f64,
        duration: &f64,
        reset: &bool,
    ) -> Result<(), NetStreamError> {
        self.amf0_writer.write_string(&String::from("play"))?;
        self.amf0_writer.write_number(transaction_id)?;
        self.amf0_writer.write_null()?;
        self.amf0_writer.write_string(stream_name)?;
        self.amf0_writer.write_number(start)?;
        self.amf0_writer.write_number(duration)?;
        self.amf0_writer.write_bool(reset)?;

        Ok(())
    }
    fn delete_stream(
        &mut self,
        transaction_id: &f64,
        stream_id: &f64,
    ) -> Result<(), NetStreamError> {
        self.amf0_writer
            .write_string(&String::from("deleteStream"))?;
        self.amf0_writer.write_number(transaction_id)?;
        self.amf0_writer.write_null()?;
        self.amf0_writer.write_number(stream_id)?;

        Ok(())
    }

    fn close_stream(
        &mut self,
        transaction_id: &f64,
        stream_id: &f64,
    ) -> Result<(), NetStreamError> {
        self.amf0_writer
            .write_string(&String::from("closeStream"))?;
        self.amf0_writer.write_number(transaction_id)?;
        self.amf0_writer.write_null()?;
        self.amf0_writer.write_number(stream_id)?;

        Ok(())
    }

    fn receive_audio(&mut self, transaction_id: &f64, enable: &bool) -> Result<(), NetStreamError> {
        self.amf0_writer
            .write_string(&String::from("receiveAudio"))?;
        self.amf0_writer.write_number(transaction_id)?;
        self.amf0_writer.write_null()?;
        self.amf0_writer.write_bool(enable)?;

        Ok(())
    }

    fn receive_video(&mut self, transaction_id: &f64, enable: &bool) -> Result<(), NetStreamError> {
        self.amf0_writer
            .write_string(&String::from("receiveVideo"))?;
        self.amf0_writer.write_number(transaction_id)?;
        self.amf0_writer.write_null()?;
        self.amf0_writer.write_bool(enable)?;

        Ok(())
    }
    fn publish(
        &mut self,
        transaction_id: &f64,
        stream_name: &String,
        stream_type: &String,
    ) -> Result<(), NetStreamError> {
        self.amf0_writer.write_string(&String::from("publish"))?;
        self.amf0_writer.write_number(transaction_id)?;
        self.amf0_writer.write_null()?;
        self.amf0_writer.write_string(stream_name)?;
        self.amf0_writer.write_string(stream_type)?;

        Ok(())
    }
    fn seek(&mut self, transaction_id: &f64, ms: &f64) -> Result<(), NetStreamError> {
        self.amf0_writer.write_string(&String::from("seek"))?;
        self.amf0_writer.write_number(transaction_id)?;
        self.amf0_writer.write_null()?;
        self.amf0_writer.write_number(ms)?;

        Ok(())
    }

    fn pause(
        &mut self,
        transaction_id: &f64,
        pause: &bool,
        ms: &f64,
    ) -> Result<(), NetStreamError> {
        self.amf0_writer.write_string(&String::from("pause"))?;
        self.amf0_writer.write_number(transaction_id)?;
        self.amf0_writer.write_null()?;
        self.amf0_writer.write_bool(pause)?;
        self.amf0_writer.write_number(ms)?;

        Ok(())
    }

    fn on_bw_done(&mut self, transaction_id: &f64, bandwidth: &f64) -> Result<(), NetStreamError> {
        self.amf0_writer.write_string(&String::from("onBWDone"))?;
        self.amf0_writer.write_number(transaction_id)?;
        self.amf0_writer.write_null()?;
        self.amf0_writer.write_number(bandwidth)?;
        Ok(())
    }

    fn on_status(
        &mut self,
        transaction_id: &f64,
        level: &String,
        code: &String,
        description: &String,
    ) -> Result<(), NetStreamError> {
        self.amf0_writer.write_string(&String::from("onStatus"))?;
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

        Ok(())
    }
}
