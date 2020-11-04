use failure::{Backtrace, Fail};
use std::fmt;
use std::io;

#[derive(Debug)]
pub struct ChunkUnpackError {
    pub kind: ChunkUnpackErrorKind,
}

#[derive(Debug, Fail)]
pub enum ChunkUnpackErrorKind {
    #[fail(
        display = "Received chunk with non-zero chunk type on csid {} prior to receiving a type 0 chunk",
        csid
    )]
    NoPreviousChunkOnStream { csid: u32 },

    #[fail(
        display = "Requested an invalid max chunk size of {}.  The largest chunk size possible is 2147483647",
        chunk_size
    )]
    InvalidMaxChunkSize { chunk_size: usize },

    #[fail(display = "_0")]
    Io(#[cause] io::Error),
}

impl fmt::Display for ChunkUnpackError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.kind, f)
    }
}

impl Fail for ChunkUnpackError {
    fn cause(&self) -> Option<&dyn Fail> {
        self.kind.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.kind.backtrace()
    }
}

impl From<ChunkUnpackErrorKind> for ChunkUnpackError {
    fn from(kind: ChunkUnpackErrorKind) -> Self {
        ChunkUnpackError { kind }
    }
}

impl From<io::Error> for ChunkUnpackError {
    fn from(error: io::Error) -> Self {
        ChunkUnpackError {
            kind: ChunkUnpackErrorKind::Io(error),
        }
    }
}
