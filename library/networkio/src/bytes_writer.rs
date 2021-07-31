use byteorder::{ByteOrder, WriteBytesExt};
use bytes::BytesMut;
use rand;
use rand::Rng;

use std::io::Write;

use super::bytes_errors::BytesWriteError;

use super::bytes_errors::BytesWriteErrorValue;

use super::networkio::NetworkIO;

use std::sync::Arc;

use tokio::sync::Mutex;

use std::time::Duration;

use std::ops::Index;
use std::ops::IndexMut;
use tokio::time::timeout;

pub struct BytesWriter {
    pub bytes: Vec<u8>,
}

impl BytesWriter {
    pub fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    pub fn write_u8(&mut self, byte: u8) -> Result<(), BytesWriteError> {
        self.bytes.write_u8(byte)?;
        Ok(())
    }

    pub fn or_u8_at(&mut self, position: usize, byte: u8) -> Result<(), BytesWriteError> {
        if position > self.bytes.len() {
            return Err(BytesWriteError {
                value: BytesWriteErrorValue::OutofIndex,
            });
        }
        self.bytes[position] |= byte;

        Ok(())
    }

    pub fn add_u8_at(&mut self, position: usize, byte: u8) -> Result<(), BytesWriteError> {
        if position > self.bytes.len() {
            return Err(BytesWriteError {
                value: BytesWriteErrorValue::OutofIndex,
            });
        }
        self.bytes[position] += byte;

        Ok(())
    }

    pub fn write_u8_at(&mut self, position: usize, byte: u8) -> Result<(), BytesWriteError> {
        if position > self.bytes.len() {
            return Err(BytesWriteError {
                value: BytesWriteErrorValue::OutofIndex,
            });
        }
        self.bytes[position] = byte;

        Ok(())
    }

    pub fn get(&mut self, position: usize) -> Option<&u8> {
        return self.bytes.get(position);
    }

    pub fn write_u16<T: ByteOrder>(&mut self, bytes: u16) -> Result<(), BytesWriteError> {
        self.bytes.write_u16::<T>(bytes)?;
        Ok(())
    }

    pub fn write_u24<T: ByteOrder>(&mut self, bytes: u32) -> Result<(), BytesWriteError> {
        self.bytes.write_u24::<T>(bytes)?;
        Ok(())
    }

    pub fn write_u32<T: ByteOrder>(&mut self, bytes: u32) -> Result<(), BytesWriteError> {
        self.bytes.write_u32::<T>(bytes)?;
        Ok(())
    }

    pub fn write_f64<T: ByteOrder>(&mut self, bytes: f64) -> Result<(), BytesWriteError> {
        self.bytes.write_f64::<T>(bytes)?;
        Ok(())
    }

    pub fn write(&mut self, buf: &[u8]) -> Result<(), BytesWriteError> {
        self.bytes.write(buf)?;
        Ok(())
    }

    pub fn prepend(&mut self, buf: &[u8]) -> Result<(), BytesWriteError> {
        let tmp_bytes = self.bytes.clone();
        self.bytes.clear();
        self.bytes.write(buf)?;
        self.bytes.write(tmp_bytes.as_slice())?;
        Ok(())
    }

    pub fn append(&mut self, writer: &mut BytesWriter) {
        self.bytes.append(&mut writer.bytes);
    }

    pub fn write_random_bytes(&mut self, length: u32) -> Result<(), BytesWriteError> {
        let mut rng = rand::thread_rng();
        for _ in 0..length {
            self.bytes.write_u8(rng.gen())?;
        }
        Ok(())
    }
    pub fn extract_current_bytes(&mut self) -> BytesMut {
        let mut rv_data = BytesMut::new();
        rv_data.extend_from_slice(&self.bytes.clone()[..]);
        self.bytes.clear();

        rv_data
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }
}

// impl Index<usize> for BytesWriter {
//     type Output = Option<&u8>;

//     fn index(&self, idx: usize) -> &Self::Output {
//         self.bytes.get(idx)
//     }
// }

// impl IndexMut<usize> for BytesWriter {
//     type Output = Option<&mut u8>;

//     fn index_mut(&mut self, idx: usize) -> &mut Self::Output {
//         return self.bytes.get(idx);
//     }
// }

// impl Index<Nucleotide> for NucleotideCount {
//     type Output = usize;

//     fn index(&self, nucleotide: Nucleotide) -> &Self::Output {
//         match nucleotide {
//             Nucleotide::A => &self.a,
//             Nucleotide::C => &self.c,
//             Nucleotide::G => &self.g,
//             Nucleotide::T => &self.t,
//         }
//     }
// }

pub struct AsyncBytesWriter {
    pub bytes_writer: BytesWriter,
    pub io: Arc<Mutex<NetworkIO>>,
}

impl AsyncBytesWriter {
    pub fn new(io: Arc<Mutex<NetworkIO>>) -> Self {
        Self {
            bytes_writer: BytesWriter::new(),
            io: io,
        }
    }

    pub fn write_u8(&mut self, byte: u8) -> Result<(), BytesWriteError> {
        self.bytes_writer.write_u8(byte)
    }

    pub fn write_u16<T: ByteOrder>(&mut self, bytes: u16) -> Result<(), BytesWriteError> {
        self.bytes_writer.write_u16::<T>(bytes)
    }

    pub fn write_u24<T: ByteOrder>(&mut self, bytes: u32) -> Result<(), BytesWriteError> {
        self.bytes_writer.write_u24::<T>(bytes)
    }

    pub fn write_u32<T: ByteOrder>(&mut self, bytes: u32) -> Result<(), BytesWriteError> {
        self.bytes_writer.write_u32::<T>(bytes)
    }

    pub fn write_f64<T: ByteOrder>(&mut self, bytes: f64) -> Result<(), BytesWriteError> {
        self.bytes_writer.write_f64::<T>(bytes)
    }

    pub fn write(&mut self, buf: &[u8]) -> Result<(), BytesWriteError> {
        self.bytes_writer.write(buf)
    }

    pub fn write_random_bytes(&mut self, length: u32) -> Result<(), BytesWriteError> {
        self.bytes_writer.write_random_bytes(length)
    }

    pub fn extract_current_bytes(&mut self) -> BytesMut {
        self.bytes_writer.extract_current_bytes()
    }

    pub async fn flush(&mut self) -> Result<(), BytesWriteError> {
        self.io
            .lock()
            .await
            .write(self.bytes_writer.bytes.clone().into())
            .await?;
        self.bytes_writer.bytes.clear();
        Ok(())
    }

    pub async fn flush_timeout(&mut self, duration: Duration) -> Result<(), BytesWriteError> {
        let message = timeout(
            duration,
            self.io
                .lock()
                .await
                .write(self.bytes_writer.bytes.clone().into()),
        )
        .await;

        match message {
            Ok(_) => {
                self.bytes_writer.bytes.clear();
            }
            Err(_) => {
                return Err(BytesWriteError {
                    value: BytesWriteErrorValue::Timeout,
                })
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    #[test]
    fn test_write_vec() {
        let mut v: Vec<u8> = Vec::new();

        v.push(0x01);
        assert_eq!(1, v.len());
        assert_eq!(0x01, v[0]);

        v[0] = 0x02;
        assert_eq!(0x02, v[0]);

        const FLV_HEADER: [u8; 9] = [
            0x46, // 'F'
            0x4c, //'L'
            0x56, //'V'
            0x01, //version
            0x05, //00000101  audio tag  and video tag
            0x00, 0x00, 0x00, 0x09, //flv header size
        ];

        let rv = v.write(&FLV_HEADER);

        match rv {
            Ok(val) => {
                print!("{} ", val);
            }
            _ => {}
        }

        assert_eq!(10, v.len());
    }
}
