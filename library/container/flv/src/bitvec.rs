use super::errors::MpegAacError;
use super::errors::MpegAacErrorValue;
use bitvec::prelude::*;
use bytes::BytesMut;

pub enum BitVectorOpType {
    Read,
    Write,
}

pub struct Mpeg4BitVec {
    data: BitVec,
    /*cache for aligment*/
    //cache: VecDeque<bool>,
    read_offset: usize,
    pub write_offset: usize,
}

impl Mpeg4BitVec {
    pub fn new() -> Self {
        Self {
            data: BitVec::new(),
            //cache: VecDeque::new(),
            read_offset: 0,
            write_offset: 0,
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
            let mut pop_value: u64 = 0;
            if self.data.pop().unwrap() {
                pop_value = 1;
            }
            result |= pop_value;

            // /*cache 8 element for aligment*/
            // loop {
            //     if self.cache.len() > 7 {
            //         self.cache.pop_front();
            //     } else {
            //         break;
            //     }
            // }
            // self.cache.push_back(pop_value == 1);
        }

        self.read_offset += n;

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

            self.write_offset += 1;
        }
    }

    pub fn bits_aligment(
        &mut self,
        n: usize,
        op_type: BitVectorOpType,
    ) -> Result<(), MpegAacError> {
        match op_type {
            BitVectorOpType::Read => {
                let aligment_offset = (self.read_offset + n - 1) / n * n;
                let pop_number = aligment_offset - self.read_offset;

                self.read_n_bits(pop_number)?;
            }

            BitVectorOpType::Write => {
                let aligment_offset = (self.write_offset + n - 1) / n * n;
                let push_number = aligment_offset - self.write_offset;

                for _ in 0..push_number {
                    self.data.push(false);
                }
                self.write_offset += push_number;
            }
        }

        Ok(())
    }

    pub fn len(&mut self) -> usize {
        self.data.len()
    }
}

pub fn mpeg4_bits_copy(
    des: &mut Mpeg4BitVec,
    src: &mut Mpeg4BitVec,
    n: usize,
) -> Result<u64, MpegAacError> {
    let bits_val = src.read_n_bits(n)?;

    des.write_bits(bits_val);

    Ok(bits_val)
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

    #[test]
    fn test_bit_vec_pop_push() {
        /*test for stack : first in last out*/
        let mut v: BitVec = BitVec::new();

        v.push(true);

        v.push(false);

        assert_eq!(v.pop().unwrap(), false, "not success");

        assert_eq!(v.pop().unwrap(), true, "not success");
    }
}
