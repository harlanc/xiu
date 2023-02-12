use bytes::BytesMut;
pub fn print(data: BytesMut) {
    println!("==========={}", data.len());
    let mut idx = 0;
    for i in data {
        print!("{i:02X} ");
        idx += 1;
        match idx % 16 {
            0 => {
                println!()
            }
            _ => {}
        }
    }

    println!("===========")
}

pub fn printu8(data: BytesMut) {
    println!("==========={}", data.len());
    let mut idx = 0;
    for i in data {
        print!("{i} ");
        idx += 1;
        match idx % 16 {
            0 => {
                println!()
            }
            _ => {}
        }
    }

    println!("===========")
}

pub fn print_array(data: &[u8], len: usize) {
    let mut idx = 0;
    for i in 0..len {
        print!("{:02X} ", data[i]);
        idx += 1;
        match idx % 16 {
            0 => {
                println!()
            }
            _ => {}
        }
    }
}
