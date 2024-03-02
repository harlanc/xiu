use super::define;
use super::errors::PackerError;
use super::errors::UnPackerError;
use super::RtpPacket;
use async_trait::async_trait;
use bytes::BytesMut;
use bytesio::bytes_reader::BytesReader;
use bytesio::bytesio::TNetIO;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::SystemTime;
use streamhub::define::FrameData;
use tokio::sync::Mutex;

pub trait Unmarshal<T1, T2> {
    fn unmarshal(data: T1) -> T2
    where
        Self: Sized;
}

pub trait Marshal<T> {
    fn marshal(&self) -> T;
}

pub type OnFrameFn = Box<dyn Fn(FrameData) -> Result<(), UnPackerError> + Send + Sync>;

//Arc<Mutex<Box<dyn TNetIO + Send + Sync>>> : The network connection used by packer to send a/v data
//BytesMut: The Rtp packet data that will be sent using the TNetIO
pub type OnRtpPacketFn = Box<
    dyn Fn(
            Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>,
            RtpPacket,
        ) -> Pin<Box<dyn Future<Output = Result<(), PackerError>> + Send + 'static>>
        + Send
        + Sync,
>;

pub type OnRtpPacketFn2 =
    Box<dyn Fn(RtpPacket) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>> + Send + Sync>;
// pub type OnPacketFn2 = Box<dyn Fn(&RtpPacket) + Send + Sync>;

pub trait TRtpReceiverForRtcp {
    fn on_packet_for_rtcp_handler(&mut self, f: OnRtpPacketFn2);
}

#[async_trait]
pub trait TPacker: TRtpReceiverForRtcp + Send + Sync {
    /*Split frame to rtp packets and send out*/
    async fn pack(&mut self, nalus: &mut BytesMut, timestamp: u32) -> Result<(), PackerError>;
    /*Call back function used for processing a rtp packet.*/
    fn on_packet_handler(&mut self, f: OnRtpPacketFn);
}

#[async_trait]
pub trait TVideoPacker: TPacker {
    /*pack one nalu to rtp packets*/
    async fn pack_nalu(&mut self, nalu: BytesMut) -> Result<(), PackerError>;
}

#[async_trait]
pub trait TUnPacker: TRtpReceiverForRtcp + Send + Sync {
    /*Assemble rtp fragments into complete frame and send to stream hub*/
    async fn unpack(&mut self, reader: &mut BytesReader) -> Result<(), UnPackerError>;
    /*Call back function used for processing a frame.*/
    fn on_frame_handler(&mut self, f: OnFrameFn);
}

pub(super) fn is_fu_start(fu_header: u8) -> bool {
    fu_header & define::FU_START > 0
}

pub(super) fn is_fu_end(fu_header: u8) -> bool {
    fu_header & define::FU_END > 0
}

pub fn find_start_code(nalus: &[u8]) -> Option<usize> {
    let pattern = [0x00, 0x00, 0x01];
    nalus.windows(pattern.len()).position(|w| w == pattern)
}

pub async fn split_annexb_and_process<T: TVideoPacker>(
    nalus: &mut BytesMut,
    packer: &mut T,
) -> Result<(), PackerError> {
    while !nalus.is_empty() {
        /* 0x02,...,0x00,0x00,0x01,0x02..,0x00,0x00,0x01  */
        /*  |         |              |      |             */
        /*  -----------              --------             */
        /*   first_pos         distance_to_first_pos      */
        if let Some(first_pos) = find_start_code(&nalus[..]) {
            let mut nalu_with_start_code =
                if let Some(distance_to_first_pos) = find_start_code(&nalus[first_pos + 3..]) {
                    let mut second_pos = first_pos + 3 + distance_to_first_pos;
                    while second_pos > 0 && nalus[second_pos - 1] == 0 {
                        second_pos -= 1;
                    }
                    nalus.split_to(second_pos)
                } else {
                    nalus.split_to(nalus.len())
                };

            let nalu = nalu_with_start_code.split_off(first_pos + 3);
            packer.pack_nalu(nalu).await?;
        } else {
            break;
        }
    }
    Ok(())
}

pub fn current_time() -> u64 {
    let duration = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH);

    match duration {
        Ok(result) => (result.as_nanos() / 1000) as u64,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {

    use bytes::BytesMut;

    fn find_start_code(nalus: &[u8]) -> Option<usize> {
        let pattern = [0x00, 0x00, 0x01];
        nalus.windows(pattern.len()).position(|w| w == pattern)
    }

    #[test]
    pub fn test_annexb_split() {
        let mut nalus = BytesMut::new();
        nalus.extend_from_slice(&[
            0x00, 0x00, 0x01, 0x02, 0x03, 0x05, 0x06, 0x00, 0x00, 0x00, 0x01, 0x02, 0x03, 0x04,
            0x00, 0x00, 0x01, 0x02, 0x03,
        ]);

        while !nalus.is_empty() {
            /* 0x02,...,0x00,0x00,0x01,0x02..,0x00,0x00,0x01  */
            /*  |         |              |      |             */
            /*  -----------              --------             */
            /*   first_pos              second_pos            */
            if let Some(first_pos) = find_start_code(&nalus[..]) {
                let mut nalu_with_start_code =
                    if let Some(distance_to_first_pos) = find_start_code(&nalus[first_pos + 3..]) {
                        let mut second_pos = first_pos + 3 + distance_to_first_pos;
                        println!("left: {first_pos} right: {distance_to_first_pos}");
                        while second_pos > 0 && nalus[second_pos - 1] == 0 {
                            second_pos -= 1;
                        }
                        // while nalus[pos_right ]
                        nalus.split_to(second_pos)
                    } else {
                        nalus.split_to(nalus.len())
                    };

                println!("nalu_with_start_code: {:?}", nalu_with_start_code.to_vec());

                let nalu = nalu_with_start_code.split_off(first_pos + 3);
                println!("nalu: {:?}", nalu.to_vec());
            } else {
                break;
            }
        }
    }
}
