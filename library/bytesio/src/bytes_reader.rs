use {
    super::bytes_errors::{BytesReadError, BytesReadErrorValue},
    byteorder::{ByteOrder, ReadBytesExt},
    bytes::{BufMut, BytesMut},
    std::io::Cursor,
};

pub struct BytesReader {
    buffer: BytesMut,
}
impl BytesReader {
    pub fn new(input: BytesMut) -> Self {
        Self { buffer: input }
    }

    pub fn extend_from_slice(&mut self, extend: &[u8]) {
        let remaining_mut = self.buffer.remaining_mut();
        let extend_length = extend.len();

        if extend_length > remaining_mut {
            let additional = extend_length - remaining_mut;
            self.buffer.reserve(additional);
        }

        self.buffer.extend_from_slice(extend)
    }

    pub fn read_bytes(&mut self, bytes_num: usize) -> Result<BytesMut, BytesReadError> {
        if self.buffer.len() < bytes_num {
            return Err(BytesReadError {
                value: BytesReadErrorValue::NotEnoughBytes,
            });
        }
        Ok(self.buffer.split_to(bytes_num))
    }

    pub fn advance_bytes(&mut self, bytes_num: usize) -> Result<BytesMut, BytesReadError> {
        if self.buffer.len() < bytes_num {
            return Err(BytesReadError {
                value: BytesReadErrorValue::NotEnoughBytes,
            });
        }

        //here maybe optimised
        Ok(self.buffer.clone().split_to(bytes_num))
    }

    pub fn read_bytes_cursor(
        &mut self,
        bytes_num: usize,
    ) -> Result<Cursor<BytesMut>, BytesReadError> {
        let tmp_bytes = self.read_bytes(bytes_num)?;
        let tmp_cursor = Cursor::new(tmp_bytes);
        Ok(tmp_cursor)
    }

    pub fn advance_bytes_cursor(
        &mut self,
        bytes_num: usize,
    ) -> Result<Cursor<BytesMut>, BytesReadError> {
        let tmp_bytes = self.advance_bytes(bytes_num)?;
        let tmp_cursor = Cursor::new(tmp_bytes);
        Ok(tmp_cursor)
    }

    pub fn read_u8(&mut self) -> Result<u8, BytesReadError> {
        let mut cursor = self.read_bytes_cursor(1)?;

        Ok(cursor.read_u8()?)
    }

    pub fn advance_u8(&mut self) -> Result<u8, BytesReadError> {
        let mut cursor = self.advance_bytes_cursor(1)?;
        Ok(cursor.read_u8()?)
    }

    pub fn read_u16<T: ByteOrder>(&mut self) -> Result<u16, BytesReadError> {
        let mut cursor = self.read_bytes_cursor(2)?;
        let val = cursor.read_u16::<T>()?;
        Ok(val)
    }

    pub fn read_u24<T: ByteOrder>(&mut self) -> Result<u32, BytesReadError> {
        let mut cursor = self.read_bytes_cursor(3)?;
        let val = cursor.read_u24::<T>()?;
        Ok(val)
    }

    pub fn advance_u24<T: ByteOrder>(&mut self) -> Result<u32, BytesReadError> {
        let mut cursor = self.advance_bytes_cursor(3)?;
        Ok(cursor.read_u24::<T>()?)
    }

    pub fn read_u32<T: ByteOrder>(&mut self) -> Result<u32, BytesReadError> {
        let mut cursor = self.read_bytes_cursor(4)?;
        let val = cursor.read_u32::<T>()?;

        Ok(val)
    }

    pub fn read_f64<T: ByteOrder>(&mut self) -> Result<f64, BytesReadError> {
        let mut cursor = self.read_bytes_cursor(8)?;
        let val = cursor.read_f64::<T>()?;

        Ok(val)
    }

    pub fn get(&mut self, index: usize) -> Result<u8, BytesReadError> {
        if index >= self.len() {
            return Err(BytesReadError {
                value: BytesReadErrorValue::IndexOutofRange,
            });
        }

        Ok(self.buffer.get(index).unwrap().clone())
    }

    pub fn len(&mut self) -> usize {
        return self.buffer.len();
    }

    pub fn extract_remaining_bytes(&mut self) -> BytesMut {
        return self.buffer.split_to(self.buffer.len());
    }
    pub fn get_remaining_bytes(&mut self) -> BytesMut {
        return self.buffer.clone();
    }
}

#[cfg(test)]
mod tests {

    use super::BytesReader;
    use bytes::BytesMut;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn test_rc_refcell() {
        let reader = Rc::new(RefCell::new(BytesReader::new(BytesMut::new())));
        let xs: [u8; 3] = [1, 2, 3];
        reader.borrow_mut().extend_from_slice(&xs[..]);

        let mut rv = reader.borrow_mut().read_u8().unwrap();
        assert_eq!(rv, 1, "Incorrect value");

        rv = reader.borrow_mut().read_u8().unwrap();
        assert_eq!(rv, 2, "Incorrect value");

        rv = reader.borrow_mut().read_u8().unwrap();
        assert_eq!(rv, 3, "Incorrect value");
    }

    struct RefStruct {
        pub reader: Rc<RefCell<BytesReader>>,
    }

    impl RefStruct {
        pub fn new(reader: Rc<RefCell<BytesReader>>) -> Self {
            Self { reader: reader }
        }

        // pub fn read_u8(&mut self) -> u8 {
        //     return self.reader.borrow_mut().read_u8().unwrap();
        // }

        pub fn extend_from_slice(&mut self, data: &[u8]) {
            self.reader.borrow_mut().extend_from_slice(data);
        }
    }

    #[test]
    fn test_struct_rc_refcell() {
        let reader = Rc::new(RefCell::new(BytesReader::new(BytesMut::new())));

        let mut ref_struct = RefStruct::new(reader);

        let xs: [u8; 3] = [1, 2, 3];
        ref_struct.extend_from_slice(&xs);

        let mut reader = ref_struct.reader.borrow_mut();

        let mut rv = reader.read_u8().unwrap();
        assert_eq!(rv, 1, "Incorrect value");

        rv = reader.read_u8().unwrap();
        assert_eq!(rv, 2, "Incorrect value");

        rv = reader.read_u8().unwrap();
        assert_eq!(rv, 3, "Incorrect value");
    }
}
