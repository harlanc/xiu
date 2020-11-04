use std::io::Read;
use std::io::Write;

struct Buffer {
    data: Vec<u8>,
}

impl Read for Buffer {
    fn read(buf: &mut [u8]) -> Result<usize> {}
}

impl Write for Buffer {
    fn write(buf: &[u8]) -> Result<usize> {}
}
