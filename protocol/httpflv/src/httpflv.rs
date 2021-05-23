use super::define::tag_type;
use super::errors::HttpFLvError;
use super::errors::HttpFLvErrorValue;
use byteorder::BigEndian;
use networkio::bytes_writer::BytesWriter;
use rtmp::amf0::amf0_writer::Amf0Writer;
use rtmp::session::common::SessionInfo;
use rtmp::session::define::SessionSubType;
use rtmp::session::errors::SessionError;
use rtmp::session::errors::SessionErrorValue;
use {
    crate::rtmp::channels::define::{
        ChannelData, ChannelDataConsumer, ChannelDataProducer, ChannelEvent, ChannelEventProducer,
    },
    bytes::BytesMut,
    networkio::networkio::NetworkIO,
    std::{sync::Arc, time::Duration},
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

pub struct HttpFlv {
    //writer: BytesWriter,
    data_consumer: ChannelDataConsumer,
    event_producer: ChannelEventProducer,
}

impl HttpFlv {
    fn new(event_producer: ChannelEventProducer) -> Self {
        let (_, data_consumer) = mpsc::unbounded_channel();
        Self {
            data_consumer,
            event_producer,
        }
    }

    fn get_set_data_frame_bytes_len() -> Result<u32, HttpFLvError> {
        let bytes_writer: BytesWriter = BytesWriter::new();
        let amf_writer: Amf0Writer = Amf0Writer::new(bytes_writer);

        amf_writer.write_string(String::from("@setDataFrame"))?;
        Ok(bytes_writer.bytes.len() as u32)
    }

    pub fn write_flv_header() -> Result<(), SessionError> {
        let writer: BytesWriter = BytesWriter::new();
        writer.write(FLV_HEADER)?;
        Ok(())
    }

    pub fn write_previous_tag_size(size: u32) -> Result<u32, HttpFLvError> {
        let writer: BytesWriter = BytesWriter::new();
        writer.write_u32::<BigEndian>(size)?;
    }

    pub fn write_flv_tag_header(
        t: tag_type,
        data_size: u32,
        timestamp: u32,
    ) -> Result<(), SessionError> {
        let writer: BytesWriter = BytesWriter::new();

        //tag type
        writer.write_u8(t)?;
        //data size
        writer.write_u24::<BigEndian>(data_size)?;
        //timestamp
        writer.write_u24::<BigEndian>(timestamp & 0xffffff)?;
        //timestamp extended.
        writer.write_u8(timestamp >> 24 & 0xff)?;

        Ok(())
    }

    pub async fn send_rtmp_channel_data(&mut self) -> Result<(), SessionError> {
        loop {
            if let Some(data) = self.data_consumer.recv().await {
                match data {
                    ChannelData::Audio { timestamp, data } => {}
                    ChannelData::Video { timestamp, data } => {}
                    ChannelData::MetaData { body } => {}
                }
            }
        }
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
                session_sub_type: SessionSubType::Publisher,
            };

            let subscribe_event = ChannelEvent::Subscribe {
                app_name: app_name.clone(),
                stream_name: stream_name.clone(),
                session_info: session_info,
                responder: sender,
            };
            let rv = self.event_producer.send(subscribe_event);
            match rv {
                Err(_) => {
                    let session_error = SessionError {
                        value: SessionErrorValue::SendChannelDataErr,
                    };
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
