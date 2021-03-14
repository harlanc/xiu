pub enum ChannelErrorValue {
    NoAppName,
    NoStreamName,
    Exists,
}

pub struct ChannelError {
    pub value: ChannelErrorValue,
}
