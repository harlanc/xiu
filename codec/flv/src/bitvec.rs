use super::errors::MpegAacError;
use super::errors::MpegAacErrorValue;
use bitvec::prelude::*;
use bytes::BytesMut;

pub struct Mpeg4BitVec {
    data: BitVec,
}

impl Mpeg4BitVec {
    pub fn new() -> Self {
        Self {
            data: BitVec::new(),
        }
    }

    pub fn extend_from_bytesmut(&mut self, data: BytesMut) {
        for ele in data {
            let bit = BitSlice::<Msb0, _>::from_element(&ele);
            self.data.extend_from_bitslice(bit);
        }
    }

    pub fn read_n_bits(&mut self, n: usize) -> Result<u64, MpegAacError> {
        let bit_length = self.data.len();

        if n > bit_length {
            return Err(MpegAacError {
                value: MpegAacErrorValue::NotEnoughBitsToRead,
            });
        }
        let mut result: u64 = 0;

        for _ in 0..n {
            result <<= 1;
            if self.data.pop().unwrap() {
                result |= 1;
            }
        }

        Ok(result)
    }

    pub fn write_bits(&mut self, data: u64) {
        let mut mut_data = data;
        loop {
            if mut_data == 0 {
                break;
            }
            self.data.push((mut_data & 0x01) > 0);
            mut_data = mut_data >> 1;
        }
    }

    fn len(&mut self) -> usize {
        self.data.len()
    }
}

#[cfg(test)]
mod tests {

    use super::Mpeg4BitVec;
    use bytes::BytesMut;

    use bitvec::prelude::*;

    #[test]
    fn test_bit_vec() {
        let data_0 = 2u8;
        let bits_0 = BitSlice::<Msb0, _>::from_element(&data_0);

        let data_1 = 7u8;
        let bits_1 = BitSlice::<Msb0, _>::from_element(&data_1);

        let mut bit_vec: BitVec<Msb0> = BitVec::new();

        bit_vec.extend_from_bitslice(bits_0);
        bit_vec.extend_from_bitslice(bits_1);

        for ele in bit_vec.clone() {
            let mut v = 0;
            if ele {
                v = 1;
            }
            print!("{} ", v);
        }

        print!("\n");

        bit_vec.pop();

        for ele in bit_vec {
            let mut v = 0;
            if ele {
                v = 1;
            }
            print!("{} ", v);
        }
    }
    #[test]
    fn test_mpeg_bit_vec() {
        let mut v = Mpeg4BitVec::new();

        let mut bytes = BytesMut::new();

        bytes.extend_from_slice(&[2u8, 7u8]);

        v.extend_from_bytesmut(bytes);

        let length = v.len();

        for _ in 0..length {
            print!("{} ", v.read_n_bits(1).unwrap());
        }
    }
}
