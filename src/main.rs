fn main() {
    println!("Hello, world!");
}


#[cfg(test)]
mod tests {

    // #[test]

    // fn test_byte_order() {
    //     use byteorder::{ByteOrder, LittleEndian,BigEndian};

    //     let phi = 1.6180339887;
    //     let mut buf = [0; 8];
    //     BigEndian::write_f64(&mut buf, phi);
    //     assert_eq!(phi, BigEndian::read_f64(&buf));
    //     println!("tsetstt")
    // }

    #[test]
    fn test_vector(){

     let mut v: Vec<u8> = Vec::new();

     v.push(2);
     v.push(3);
     v.push(4);

     //println!(v.get(0).unwrap())

     println!("{} days=========", v.get(0).unwrap());




    }
}
