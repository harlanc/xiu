use std::fmt;

pub mod rtsp_method_name {
    pub const OPTIONS: &str = "OPTIONS";
    pub const DESCRIBE: &str = "DESCRIBE";
    pub const ANNOUNCE: &str = "ANNOUNCE";
    pub const SETUP: &str = "SETUP";
    pub const PLAY: &str = "PLAY";
    pub const PAUSE: &str = "PAUSE";
    pub const TEARDOWN: &str = "TEARDOWN";
    pub const GET_PARAMETER: &str = "GET_PARAMETER";
    pub const SET_PARAMETER: &str = "SET_PARAMETER";
    pub const REDIRECT: &str = "REDIRECT";
    pub const RECORD: &str = "RECORD";

    pub const ARRAY: [&str; 11] = [
        OPTIONS,
        DESCRIBE,
        ANNOUNCE,
        SETUP,
        PLAY,
        PAUSE,
        TEARDOWN,
        GET_PARAMETER,
        SET_PARAMETER,
        REDIRECT,
        RECORD,
    ];
}

pub enum SessionType {
    Client,
    Server,
}

impl fmt::Display for SessionType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let client_type = match self {
            SessionType::Client => String::from("client"),
            SessionType::Server => String::from("server"),
        };
        write!(f, "{client_type}")
    }
}
