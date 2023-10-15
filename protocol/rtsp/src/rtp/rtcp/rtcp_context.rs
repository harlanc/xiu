use crate::rtp::utils;
use crate::rtp::RtpPacket;
use bytes::BytesMut;

use super::{
    rtcp_app::RtcpApp,
    rtcp_bye::RtcpBye,
    rtcp_header::RtcpHeader,
    rtcp_rr::{ReportBlock, RtcpReceiverReport},
    rtcp_sr::RtcpSenderReport,
    RTCP_RR,
};

//For example: sequence numbers inserted are 65533, 65534, the new coming one is 2,
//the new is 2 and old is 65534, the distance between 2 and 65534 is 4 which is
//65535 - 65534 + 2 + 1.(65533,65534,65535,0,1,2)
pub fn distance(new: u16, old: u16) -> u16 {
    new.wrapping_sub(old)
}

const MIN_SEQUENTIAL: u32 = 2;
const RTP_SEQ_MOD: u32 = 1 << 16;
const MAX_DROPOUT: u32 = 3000;
const MAX_MISORDER: u32 = 100;

/*
 * Per-source state information
 */
#[derive(Debug, Clone, Default)]
struct RtcpSource {
    max_seq: u16,        /* highest seq. number seen */
    cycles: u32,         /* shifted count of seq. number cycles */
    base_seq: u32,       /* base seq number */
    bad_seq: u32,        /* last 'bad' seq number + 1 */
    probation: u32,      /* sequ. packets till source is valid */
    received: u32,       /* packets received */
    expected_prior: u32, /* packet expected at last interval */
    received_prior: u32, /* packet received at last interval */
    jitter: f64,         /* estimated jitter */
}

impl RtcpSource {
    //static int rtp_seq_update(struct rtp_member *sender, uint16_t seq)
    fn update_sequence(&mut self, seq: u16) -> usize {
        let delta = distance(seq, self.max_seq);

        if self.probation > 0 {
            /* packet is in sequence */
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
            /* in order, with permissible gap */
            if seq < self.max_seq {
                /*
                 * Sequence number wrapped - count another 64K cycle.
                 */
                self.cycles += RTP_SEQ_MOD;
            }
            self.max_seq = seq;
        } else if delta as u32 <= RTP_SEQ_MOD - MAX_MISORDER {
            if seq == self.bad_seq as u16 {
                /*
                 * Two sequential packets -- assume that the other side
                 * restarted without telling us so just re-sync
                 * (i.e., pretend this was the first packet).
                 */
                self.init_seq(seq);
            } else {
                self.bad_seq = (seq as u32 + 1) & (RTP_SEQ_MOD - 1);
                return 0;
            }
        } else {
            /* duplicate or reordered packet */
        }
        self.received += 1;

        1
    }

    fn init_seq(&mut self, seq: u16) {
        self.base_seq = seq as u32;
        self.max_seq = seq;
        self.bad_seq = RTP_SEQ_MOD + 1; /* so seq == bad_seq is false */
        self.cycles = 0;
        self.received = 0;
        self.received_prior = 0;
        self.expected_prior = 0;
        /* other initialization */

        self.base_seq = seq as u32;
        self.max_seq = seq;
        self.bad_seq = RTP_SEQ_MOD + 1;
    }
}

#[derive(Debug, Clone, Default)]
pub struct RtcpContext {
    ssrc: u32,
    sender_ssrc: u32,

    sr_ntp_lsr: u64,
    sr_clock_time: u64,
    last_rtp_clock: u64,
    last_rtp_timestamp: u32,
    sample_rate: u32,
    send_bytes: u64,
    send_packets: u64,

    source: RtcpSource,
}

impl RtcpContext {
    pub fn new(ssrc: u32, seq: u16, sample_rate: u32) -> Self {
        RtcpContext {
            ssrc,
            source: RtcpSource {
                max_seq: seq - 1,
                probation: MIN_SEQUENTIAL,
                ..Default::default()
            },

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
        let ssrss = vec![self.ssrc];
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
        let extend_max = self.source.cycles + self.source.max_seq as u32;
        let expected = extend_max - self.source.base_seq + 1;
        let lost = expected - self.source.received;
        let expected_interval = expected - self.source.expected_prior;
        self.source.expected_prior = expected;

        let received_interval = self.source.received - self.source.received_prior;
        self.source.received_prior = self.source.received;
        let lost_interval = expected_interval as i64 - received_interval as i64;

        let fraction = if expected_interval == 0 || lost_interval < 0 {
            0
        } else {
            ((lost_interval as u32) << 8) / expected_interval
        };

        let delay = utils::current_time() - self.sr_clock_time;
        let lsr = self.sr_ntp_lsr >> 8 & 0xFFFFFFFF;
        let dlsr = (delay as f64 / 1000000. * 65535.) as u32;

        ReportBlock {
            cumutlative_num_of_packets_lost: lost,
            fraction_lost: fraction as u8,
            extended_highest_seq_number: extend_max,
            lsr: lsr as u32,
            dlsr,
            ssrc: self.sender_ssrc,
            jitter: self.source.jitter as u32,
        }
    }

    pub fn generate_rr(&mut self) -> RtcpReceiverReport {
        let block = self.gen_report_block();

        RtcpReceiverReport {
            header: RtcpHeader {
                payload_type: RTCP_RR,
                report_count: 1,
                version: 2,
                length: (4 + 24) / 4,
                ..Default::default()
            },
            report_blocks: vec![block],
            ssrc: self.ssrc,
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
        if 0 == self.source.update_sequence(pkt.header.seq_number) {
            return;
        }

        let rtp_clock = utils::current_time();
        if self.last_rtp_clock == 0 {
            self.source.jitter = 0.;
        } else {
            let mut d = ((rtp_clock - self.last_rtp_clock) * self.sample_rate as u64 / 1000000)
                as i64
                - (pkt.header.timestamp - self.last_rtp_timestamp) as i64;

            if d < 0 {
                d = -d;
            }
            self.source.jitter += (d as f64 - self.source.jitter) / 16.;
        }

        self.last_rtp_clock = rtp_clock;
        self.last_rtp_timestamp = pkt.header.timestamp;
    }
}

#[cfg(test)]
mod tests {

    use super::distance;
    #[test]
    fn test_distance() {
        assert_eq!(distance(0, 0), 0);
        assert_eq!(distance(2, 0), 2);
        assert_eq!(distance(32767, 0), 32767);
        assert_eq!(distance(65535, 0), 65535);

        assert_eq!(distance(0, 65535), 1);
        assert_eq!(distance(0, 2), 65534);
    }
}
