use crate::rtp::utils;
use crate::rtp::RtpPacket;
use bytes::BytesMut;
use std::time::SystemTime;

use super::{
    rtcp_app::RtcpApp,
    rtcp_bye::RtcpBye,
    rtcp_header::RtcpHeader,
    rtcp_rr::{ReportBlock, RtcpReceiverReport},
    rtcp_sr::RtcpSenderReport,
    RTCP_RR,
};

#[derive(Debug, Clone, Default)]
pub struct RtcpContext {
    ssrc: u32,
    sender_ssrc: u32,
    max_seq: u16,
    cycles: u32,
    base_seq: u32,
    bad_seq: u32,
    probation: usize,
    received: u32,
    expect_prior: u32,
    received_prior: u32,
    transit: u32,
    jitter: f64,
    sr_ntp_lsr: u64,
    sr_clock_time: u64,
    last_rtp_clock: u64,
    last_rtp_timestamp: u32,
    sample_rate: u32,
    send_bytes: u64,
    send_packets: u64,
    bindwidth: usize,
}

const MIN_SEQUENTIAL: usize = 2;
const RTP_SEQ_MOD: usize = 1 << 16;
const MAX_DROPOUT: usize = 3000;
const MAX_MISORDER: usize = 100;

impl RtcpContext {
    pub fn new(ssrc: u32, seq: u16, sample_rate: u32) -> Self {
        RtcpContext {
            ssrc,
            max_seq: seq - 1,
            probation: MIN_SEQUENTIAL,
            sample_rate,
            ..Default::default()
        }
    }

    pub fn generate_app(&self, name: String, data: BytesMut) -> RtcpApp {
        let mut buf = BytesMut::with_capacity(name.len());
        buf.extend_from_slice(name.as_bytes());

        RtcpApp {
            ssrc: self.ssrc,
            name: buf,
            app_data: data,
            ..Default::default()
        }
    }

    pub fn generate_bye(&self) -> RtcpBye {
        let mut ssrss = Vec::new();
        ssrss.push(self.ssrc);
        RtcpBye {
            header: RtcpHeader {
                report_count: 1,
                ..Default::default()
            },
            ssrss,
            ..Default::default()
        }
    }

    //int rtcp_report_block(struct rtp_member* sender, uint8_t* ptr, int bytes)
    fn gen_report_block(&mut self) -> ReportBlock {
        let extend_max = self.cycles + self.max_seq as u32;
        let expected = extend_max - self.base_seq + 1;
        let lost = expected - self.received;
        let expected_interval = expected - self.expect_prior;
        self.expect_prior = expected;

        let received_interval = self.received - self.received_prior;
        self.received_prior = self.received;
        let lost_interval = expected_interval - received_interval;

        let fraction = if expected_interval == 0 || lost_interval < 0 {
            0
        } else {
            (lost_interval << 8) / expected_interval
        };

        let delay = utils::current_time() - self.sr_clock_time;
        let lsr = self.sr_ntp_lsr >> 8 & 0xFFFFFFFF;
        let dlsr = (delay as f64 / 1000000. * 65535.) as u32;

        let mut report_block = ReportBlock::default();
        report_block.cumutlative_num_of_packets_lost = lost;
        report_block.fraction_lost = fraction as u8;
        report_block.extended_highest_seq_number = extend_max;
        report_block.lsr = lsr as u32;
        report_block.dlsr = dlsr;
        report_block.ssrc = self.sender_ssrc;
        report_block.jitter = self.jitter as u32;

        report_block
    }

    pub fn generate_rr(&mut self) -> RtcpReceiverReport {
        let block = self.gen_report_block();
        let mut blocks = Vec::new();
        blocks.push(block);

        RtcpReceiverReport {
            header: RtcpHeader {
                payload_type: RTCP_RR,
                report_count: 1,
                ..Default::default()
            },
            report_blocks: blocks,
            ssrc: self.ssrc,
            ..Default::default()
        }
    }

    pub fn send_rtp(&mut self, pkt: RtpPacket) {
        self.send_bytes += pkt.payload.len() as u64;
        self.send_packets += 1;
        self.last_rtp_timestamp = pkt.header.timestamp;
    }

    pub fn received_sr(&mut self, sr: &RtcpSenderReport) {
        self.sr_clock_time = utils::current_time();

        self.sr_ntp_lsr = sr.ntp;
        self.sender_ssrc = sr.ssrc;
    }

    pub fn received_rtp(&mut self, pkt: RtpPacket) {

    }

    //static int rtp_seq_update(struct rtp_member *sender, uint16_t seq)
    fn update_sequence(&mut self, seq: u16) -> usize {
        let delta = seq - self.max_seq;
        if self.probation > 0 {
            if seq == self.max_seq + 1 {
                self.probation -= 1;
                self.max_seq = seq;
                if self.probation == 0 {
                    self.init_seq(seq);
                    self.received += 1;
                    return 1;
                }
            } else {
                self.probation = MIN_SEQUENTIAL - 1;
                self.max_seq = seq;
            }
            return 0;
        } else if delta < MAX_DROPOUT as u16 {
            if seq < self.max_seq {
                self.cycles += RTP_SEQ_MOD as u32;
            }
            self.max_seq = seq;
        } else if delta <= RTP_SEQ_MOD as u16 - MAX_MISORDER as u16 {
            if seq == self.bad_seq as u16 {
                self.init_seq(seq);
            } else {
                self.bad_seq = ((seq + 1) & (RTP_SEQ_MOD as u16 - 1)) as u32;
                return 0;
            }
        } else {
        }
        self.received += 1;

        1
    }

    fn init_seq(&mut self, seq: u16) {
        self.base_seq = seq as u32;
        self.max_seq = seq;
        self.bad_seq = RTP_SEQ_MOD as u32 + 1;
    }
}
