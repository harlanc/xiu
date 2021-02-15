pub struct SetPeerBandwidthProperties {
    window_size: u32,
    limit_type: u8,
}

impl SetPeerBandwidthProperties {
    pub fn new(window_size: u32, limit_type: u8) -> Self {
        Self {
            window_size: window_size,
            limit_type: limit_type,
        }
    }
}
