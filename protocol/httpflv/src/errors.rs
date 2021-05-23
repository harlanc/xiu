use failure::Fail;
use std::error::Error;
// #[derive(Debug)]
// pub struct ServerError {
//     pub value: ServerErrorValue,
// }

// #[derive(Debug, Fail)]
// pub enum ServerErrorValue {
//     #[fail(display = "server error")]
//     Error(Error),
// }

// impl From<Error> for ServerError {
//     fn from(error: Error) -> Self {
//         ServerError {
//             value: ServerErrorValue::Error(error),
//         }
//     }
// }
