pub fn pcr_write(pcr_result: &mut Vec<u8>, pcr: i64) {
    let pcr_base: i64 = pcr / 300;
    let pcr_ext: i64 = pcr % 300;

    pcr_result[0] = (pcr_base >> 25) as u8 & 0xFF;
    pcr_result[1] = (pcr_base >> 17) as u8 & 0xFF;
    pcr_result[2] = (pcr_base >> 9) as u8 & 0xFF;
    pcr_result[3] = (pcr_base >> 1) as u8 & 0xFF;
    pcr_result[4] = ((pcr_base & 0x01) << 7) as u8 | 0x7E | ((pcr_ext >> 8) & 0x01) as u8;
    pcr_result[5] = (pcr_ext & 0xFF) as u8;
}
