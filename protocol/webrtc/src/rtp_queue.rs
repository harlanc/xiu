use std::collections::VecDeque;
use webrtc::rtp::packet::Packet as RtpPacket;

const MIN_SEQUENTIAL: usize = 2;
const RTP_SEQ_MOD: u32 = 1 << 16;
const MAX_DROPOUT: u16 = 3000;
const MAX_MISORDER: u16 = 100;

pub struct RtpQueue {
    cycles: u32,      /* shifted count of seq. number cycles */
    bad_seq: u16,     /* last 'bad' seq number + 1 */
    probation: usize, /* sequ. packets till source is valid */

    first_ordered_seq: u16, /* the first ordered sequence number needs to be sent out */
    max_cache_size: usize,
    cache: VecDeque<RtpPacket>,
    bad_cache: Vec<RtpPacket>,
}

impl RtpQueue {
    pub fn new(max_cache_size: usize) -> Self {
        RtpQueue {
            cycles: 0,
            bad_seq: 0,
            probation: MIN_SEQUENTIAL,
            first_ordered_seq: 0,
            max_cache_size,
            cache: VecDeque::new(),
            bad_cache: Vec::new(),
        }
    }
    fn front_seq(&self) -> u16 {
        if let Some(pkt) = self.cache.front() {
            pkt.header.sequence_number
        } else {
            0
        }
    }

    fn back_seq(&self) -> u16 {
        if let Some(pkt) = self.cache.back() {
            pkt.header.sequence_number
        } else {
            0
        }
    }

    fn clear_cache(&mut self) {
        self.cache.clear();
        self.probation = MIN_SEQUENTIAL;
    }

    fn clear_bad_cache(&mut self) {
        self.bad_cache.clear();
        self.bad_seq = 0;
    }
    fn insert(&mut self, packet: RtpPacket) {
        let cur_seq_number = packet.header.sequence_number;

        let cur_cache_size = self.cache.len();

        for (index, item) in self.cache.iter_mut().rev().enumerate() {
            // let delta = cur_seq_number.wrapping_sub(item.header.seq_number) as i16;
            // if delta == 0 {
            //     break;
            // } else if delta > 0 {
            //     self.cache.insert(cur_cache_size - index, packet);
            //     break;
            // }

            match cur_seq_number.wrapping_sub(item.header.sequence_number) as i16 {
                0 => {
                    break;
                }
                1.. => {
                    self.cache.insert(cur_cache_size - index, packet);
                    break;
                }

                _ => {}
            }
        }
    }

    fn get_seqs(&self) -> String {
        let mut res: String = String::from("");
        for ele in &self.cache {
            res += ele.header.sequence_number.to_string().as_str();
            res += ",";
        }

        res
    }
    pub fn write_queue(&mut self, packet: RtpPacket) {
        let cur_seq_number = packet.header.sequence_number;

        log::debug!(
            "write queue: {}, cache size:{}, queue: {}",
            cur_seq_number,
            self.cache.len(),
            self.get_seqs()
        );

        if self.probation > 0 {
            if self.cache.is_empty() {
                self.cache.push_back(packet);
                return;
            }

            if packet.header.sequence_number == self.back_seq().wrapping_add(1) {
                self.probation -= 1;
                if self.probation == 0 {
                    if let Some(pkt) = self.cache.front() {
                        self.first_ordered_seq = pkt.header.sequence_number;
                    }
                }
            } else {
                self.clear_cache();
            }

            self.cache.push_back(packet);
        } else {
            let delta = cur_seq_number.wrapping_sub(self.back_seq());

            if delta == 0 {
                log::debug!("duplicate");
                //duplicate
                return;
            } else if delta < MAX_DROPOUT {
                log::debug!("with permissible gap");
                /* in order, with permissible gap */
                if cur_seq_number < self.back_seq() {
                    /*
                     * Sequence number wrapped - count another 64K cycle.
                     */
                    self.cycles += RTP_SEQ_MOD;
                }
                self.cache.push_back(packet);
            } else if self.back_seq().wrapping_sub(cur_seq_number)
                < self.back_seq().wrapping_sub(self.front_seq())
            {
                log::debug!("reordered packet");
                //reordered packet
                self.insert(packet);
            } else if self.front_seq().wrapping_sub(cur_seq_number) < MAX_MISORDER {
                log::debug!("mis order");
                //too late
                return;
            } else {
                log::debug!("bad");
                if self.bad_cache.is_empty() || cur_seq_number == self.bad_seq {
                    self.bad_cache.push(packet);
                    self.bad_seq = cur_seq_number.wrapping_add(1);
                } else {
                    self.clear_bad_cache();
                }

                // Two sequential packets -- assume that the other side
                // restarted without telling us so just re-sync
                // (i.e., pretend this was the first packet).
                if self.bad_cache.len() >= MIN_SEQUENTIAL {
                    self.cache.extend(self.bad_cache.to_owned());
                    self.clear_bad_cache();
                }
                return;
            }
            self.clear_bad_cache();
        }
    }

    pub fn read_queue(&mut self) -> Option<RtpPacket> {
        if self.cache.is_empty() || self.probation > 0 {
            return None;
        }

        let first_packet = self.cache.front().unwrap().to_owned();
        if self.first_ordered_seq == first_packet.header.sequence_number {
            self.first_ordered_seq = self.first_ordered_seq.wrapping_add(1);
        } else {
            if self.cache.len() < self.max_cache_size {
                return None;
            }
            self.first_ordered_seq = first_packet.header.sequence_number.wrapping_add(1);
        }

        self.cache.pop_front();
        Some(first_packet)
    }
}
