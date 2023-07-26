use bytes::BytesMut;
pub fn print(data: BytesMut) {
    println!("==========={}", data.len());
    let mut idx = 0;
    for i in data {
        print!("{i:02X} ");
        idx += 1;
        if idx % 16 == 0 {
            println!()
        }
    }

    println!("===========")
}

pub fn print2(title: &str, data: BytesMut) {
    println!("==========={}:{}", title, data.len());
    let mut idx = 0;
    for i in data {
        print!("{i:02X} ");
        idx += 1;
        if idx % 16 == 0 {
            println!()
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
        if idx % 16 == 0 {
            println!()
        }
    }

    println!("===========")
}

pub fn print_array(data: &[u8], len: usize) {
    let mut idx = 0;

    for item in data.iter().take(len) {
        print!("{item:02X} ");
        idx += 1;
        if idx % 16 == 0 {
            println!()
        }
    }
}
