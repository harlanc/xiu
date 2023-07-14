pub trait Unmarshal {
    fn unmarshal(request_data: &str) -> Option<Self>
    where
        Self: Sized;
}

pub trait Marshal {
    fn marshal(&self) -> String;
}
