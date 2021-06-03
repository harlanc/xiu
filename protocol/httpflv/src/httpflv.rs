use core::time;

use crate::define::HttpResponseDataConsumer;

use super::define::tag_type;
use super::define::HttpResponseDataProducer;
use super::errors::HttpFLvError;
use super::errors::HttpFLvErrorValue;
use byteorder::BigEndian;
use networkio::bytes_writer::BytesWriter;
use rtmp::amf0::amf0_writer::Amf0Writer;
use rtmp::cache::metadata::MetaData;
use rtmp::session::common::SessionInfo;
use rtmp::session::define::SessionSubType;
use rtmp::session::errors::SessionError;
use rtmp::session::errors::SessionErrorValue;
use rtmp::utils::print;
use {
    crate::rtmp::channels::define::{
        ChannelData, ChannelDataConsumer, ChannelDataProducer, ChannelEvent, ChannelEventProducer,
    },
    bytes::BytesMut,
    std::time::Duration,
    tokio::{
        sync::{mpsc, oneshot, Mutex},
        time::sleep,
    },
};

const FLV_HEADER: [u8; 9] = [
    0x46, // 'F'
    0x4c, //'L'
    0x56, //'V'
    0x01, //version
    0x05, //00000101  audio tag  and video tag
    0x00, 0x00, 0x00, 0x09, //flv header size
]; // 9
const HEADER_LENGTH: u32 = 11;
pub struct HttpFlv {
    app_name: String,
    stream_name: String,
    writer: BytesWriter,
    event_producer: ChannelEventProducer,
    data_consumer: ChannelDataConsumer,

    http_response_data_producer: HttpResponseDataProducer,
}

impl HttpFlv {
    pub fn new(
        app_name: String,
        stream_name: String,
        event_producer: ChannelEventProducer,
        http_response_data_producer: HttpResponseDataProducer,
    ) -> Self {
        let (_, data_consumer) = mpsc::unbounded_channel();

        Self {
            app_name,
            stream_name,
            writer: BytesWriter::new(),
            data_consumer,
            event_producer,
            http_response_data_producer,
        }
    }

    pub async fn run(&mut self) -> Result<(), HttpFLvError> {
        self.subscribe_from_rtmp_channels(self.app_name.clone(), self.stream_name.clone(), 50)
            .await?;

        self.send_http_response_data().await?;

        Ok(())
    }

    pub async fn send_http_response_data(&mut self) -> Result<(), HttpFLvError> {
        self.write_flv_header()?;
        self.write_previous_tag_size(0)?;
        self.flush_response_data()?;
        //write flv body
        loop {
            if let Some(data) = self.data_consumer.recv().await {
                self.write_flv_tag(data)?;
            }
        }
    }

    pub fn write_flv_header(&mut self) -> Result<(), HttpFLvError> {
        self.writer.write(&FLV_HEADER)?;
        Ok(())
    }

    pub fn write_flv_tag(&mut self, channel_data: ChannelData) -> Result<(), HttpFLvError> {
        match channel_data {
            ChannelData::Audio { timestamp, data } => {
                let len = data.len() as u32;
                self.write_flv_tag_header(tag_type::AUDIO, len, timestamp)?;
                self.write_flv_tag_body(data)?;
                self.write_previous_tag_size(len + HEADER_LENGTH)?;
            }
            ChannelData::Video { timestamp, data } => {
                let len = data.len() as u32;
                self.write_flv_tag_header(tag_type::VIDEO, len, timestamp)?;
                self.write_flv_tag_body(data)?;
                self.write_previous_tag_size(len + HEADER_LENGTH)?;
            }
            ChannelData::MetaData { timestamp, data } => {
                let mut metadata = MetaData::default();
                metadata.save(data);
                let data = metadata.remove_set_data_frame()?;
                let len = data.len() as u32;
                self.write_flv_tag_header(tag_type::SCRIPT_DATA_AMF, len, timestamp)?;
                self.write_flv_tag_body(data)?;
                self.write_previous_tag_size(len + HEADER_LENGTH)?;
            }
        }

        self.flush_response_data()?;
        Ok(())
    }

    pub fn write_flv_tag_header(
        &mut self,
        tag_type: u8,
        data_size: u32,
        timestamp: u32,
    ) -> Result<(), SessionError> {
        //tag type
        self.writer.write_u8(tag_type)?;
        //data size
        self.writer.write_u24::<BigEndian>(data_size)?;
        //timestamp
        self.writer.write_u24::<BigEndian>(timestamp & 0xffffff)?;
        //timestamp extended.
        let timestamp_ext = (timestamp >> 24 & 0xff) as u8;
        self.writer.write_u8(timestamp_ext)?;
        //stream id
        self.writer.write_u24::<BigEndian>(0)?;

        Ok(())
    }

    pub fn write_flv_tag_body(&mut self, body: BytesMut) -> Result<(), SessionError> {
        self.writer.write(&body[..])?;
        Ok(())
    }

    pub fn write_previous_tag_size(&mut self, size: u32) -> Result<(), HttpFLvError> {
        self.writer.write_u32::<BigEndian>(size)?;
        Ok(())
    }

    pub fn flush_response_data(&mut self) -> Result<(), HttpFLvError> {
        let data = self.writer.extract_current_bytes();
        print::print(data.clone());
        self.http_response_data_producer.start_send(Ok(data))?;
        Ok(())
    }

    pub async fn subscribe_from_rtmp_channels(
        &mut self,
        app_name: String,
        stream_name: String,
        session_id: u64,
    ) -> Result<(), HttpFLvError> {
        let mut retry_count: u8 = 0;

        loop {
            let (sender, receiver) = oneshot::channel();

            let session_info = SessionInfo {
                session_id: session_id,
                session_sub_type: SessionSubType::Player,
            };

            let subscribe_event = ChannelEvent::Subscribe {
                app_name: app_name.clone(),
                stream_name: stream_name.clone(),
                session_info: session_info,
                responder: sender,
            };
            println!("httpflv begin send subscribe");
            let rv = self.event_producer.send(subscribe_event);
            match rv {
                Err(_) => {
                    let session_error = SessionError {
                        value: SessionErrorValue::SendChannelDataErr,
                    };
                    println!("httpflv send subscribe error");
                    return Err(HttpFLvError {
                        value: HttpFLvErrorValue::SessionError(session_error),
                    });
                }
                _ => {}
            }

            match receiver.await {
                Ok(consumer) => {
                    self.data_consumer = consumer;
                    break;
                }
                Err(_) => {
                    if retry_count > 10 {
                        let session_error = SessionError {
                            value: SessionErrorValue::SubscribeCountLimitReach,
                        };
                        return Err(HttpFLvError {
                            value: HttpFLvErrorValue::SessionError(session_error),
                        });
                    }
                }
            }

            sleep(Duration::from_millis(800)).await;
            retry_count = retry_count + 1;
        }

        Ok(())
    }
}
