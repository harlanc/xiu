use bytes::BytesMut;
pub fn print(data: BytesMut) {
    print!("==========={}\n", data.len());
    let mut idx = 0;
    for i in data {
        print!("{:02X} ", i);
        idx = idx + 1;
        match idx % 16 {
            0 => {
                print!("\n")
            }
            _ => {}
        }
    }

    print!("===========\n")
}

pub fn printu8(data: BytesMut) {
    print!("==========={}\n", data.len());
    let mut idx = 0;
    for i in data {
        print!("{} ", i);
        idx = idx + 1;
        match idx % 16 {
            0 => {
                print!("\n")
            }
            _ => {}
        }
    }

    print!("===========\n")
}

pub fn print_array(data: &[u8], len: usize) {
    let mut idx = 0;
    for i in 0..len {
        print!("{:02X} ", data[i]);
        idx = idx + 1;
        match idx % 16 {
            0 => {
                print!("\n")
            }
            _ => {}
        }
    }
}
