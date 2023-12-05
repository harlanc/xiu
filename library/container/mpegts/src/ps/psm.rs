use super::errors::MpegPsError;
use crate::define::epes_stream_id::PES_SID_PSM;
use byteorder::BigEndian;
use {bytes::BytesMut, bytesio::bytes_reader::BytesReader};

#[derive(Default)]
pub struct ElementaryStreamMap {
    pub stream_type: u8,
    pub elementary_stream_id: u8,
    pub elementary_stream_info_length: u16,
    pub pseudo_descriptor_tag: u8,
    pub pseudo_descriptor_length: u8,
    pub elementary_stream_id_extension: u8,
}
//T-REC-H.222.0-201703-S!!PDF-E.pdf Table 2-41 P69
// program_stream_map() {
//     packet_start_code_prefix    		            24 bslbf
//     map_stream_id               		            8 uimsbf
//     program_stream_map_length 			        16 uimsbf
//     current_next_indicator		 		        1 bslbf
//     single_extension_stream_flag 		        1 bslbf
//     reserved 							        1 bslbf
//     program_stream_map_version 			        5 uimsbf
//     reserved						 	            7 bslbf
//     marker_bit 							        1 bslbf
//     program_stream_info_length 			        16 uimsbf
//     for (i = 0; i < N; i++) {
//     	  descriptor()
//     }
//     elementary_stream_map_length 		        16 uimsbf
//     for (i = 0; i < N1; i++) {
//     	stream_type					 	            8 uimsbf
//     	elementary_stream_id 			            8 uimsbf
//     	elementary_stream_info_length	            16 Uimsbf
//    	if ( elementary_stream_id = = 0xFD &&
//     		single_extension_stream_flag == 0) {
//     		pseudo_descriptor_tag 				    8 Uimsbf
//     		pseudo_descriptor_length 			    8 Uimsbf
//     		marker_bit 							    1 Bslbf
//     		elementary_stream_id_extension 		    7 Uimsbf
//     		for (i = 3; i < N2; i++) {
//     			descriptor()
//     		}
//     	}
//     	else {
//     		for (i = 0; i < N2; i++) {
//     			descriptor()
//     		}
//     	}
//     }
//     CRC_32 32 rpchof
// }

#[derive(Default)]
pub struct ProgramStreamMap {
    map_stream_id: u8,
    program_stream_map_length: u16,
    current_next_indicator: u8,
    single_extension_stream_flag: u8,

    program_stream_map_version: u8,
    program_stream_info_length: u16,
    elementary_stream_map_length: u16,
    pub stream_map: Vec<ElementaryStreamMap>,
}

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

impl ProgramStreamMap {
    pub fn parse(&mut self, bytes_reader: &mut BytesReader) -> Result<(), MpegPsError> {
        // let psm_length = (bytes_reader.get(4)? as usize) << 8 | bytes_reader.get(5)? as usize;
        // log::info!(
        //     "current pes packet length: {} : {}",
        //     psm_length,
        //     bytes_reader.len() - 6
        // );
        // print(bytes_reader.get_remaining_bytes());
        // if psm_length > bytes_reader.len() - 6 {
        //     return Err(MpegPsError {
        //         value: crate::ps::errors::MpegPsErrorValue::NotEnoughBytes,
        //     });
        // }

        // 00 00 01 BC
        // 00 12
        // E0
        // FF 00 00 00 08 1B E0 00 00
        // 90 C0 00 00 00 00 00 00

        // 00 00 01 E0 00 1D 84 80
        // 05 21 00 55 D4 79 00 00 00 01 67 42 C0 16 DA 82
        // 80 F4 9A 81 01 01 03 C2 01 0A 80 00 00 01 E0 00
        // 10 84 80 05 21 00 55 D4 79 00 00 00 01 68 CE 3C
        // 80

        bytes_reader.backup();

        let start = bytes_reader.read_bytes(4)?;

        // log::info!("psm start");

        if start.to_vec() != [0x00, 0x00, 0x01, PES_SID_PSM] {
            return Err(MpegPsError {
                value: super::errors::MpegPsErrorValue::StartCodeNotCorrect,
            });
        }
        // log::info!("psm start1");
        self.map_stream_id = PES_SID_PSM;
        self.program_stream_map_length = bytes_reader.read_u16::<BigEndian>()?;

        if self.program_stream_map_length as usize > bytes_reader.len() {
            bytes_reader.restore();
            return Err(MpegPsError {
                value: crate::ps::errors::MpegPsErrorValue::NotEnoughBytes,
            });
        }

        let byte_7 = bytes_reader.read_u8()?;
        self.current_next_indicator = byte_7 >> 7;
        self.single_extension_stream_flag = (byte_7 >> 6) & 0x01;
        self.program_stream_map_version = byte_7 & 0x1F;
        bytes_reader.read_u8()?;

        self.program_stream_info_length = bytes_reader.read_u16::<BigEndian>()?;
        // log::info!("psm start2 : {}", self.program_stream_info_length);
        if self.program_stream_info_length as usize + 2 > bytes_reader.len() {
            bytes_reader.restore();
            return Err(MpegPsError {
                value: crate::ps::errors::MpegPsErrorValue::NotEnoughBytes,
            });
        }

        // if bs.RemainBytes() < int(psm.Program_stream_info_length)+2 {
        //     bs.UnRead(10 * 8)
        //     return errNeedMore
        // }

        bytes_reader.read_bytes(self.program_stream_info_length as usize)?;

        self.elementary_stream_map_length = bytes_reader.read_u16::<BigEndian>()?;

        // log::info!(
        //     "elementary_stream_map_length: {}",
        //     self.elementary_stream_map_length
        // );

        if self.elementary_stream_map_length as usize + 4 > bytes_reader.len() {
            bytes_reader.restore();
            return Err(MpegPsError {
                value: crate::ps::errors::MpegPsErrorValue::NotEnoughBytes,
            });
        }

        // if bs.RemainBytes() < int(psm.Elementary_stream_map_length)+4 {
        //     bs.UnRead(12*8 + int(psm.Program_stream_info_length)*8)
        //     return errNeedMore
        // }

        let remaining_bytes = bytes_reader.len() - self.elementary_stream_map_length as usize;

        while bytes_reader.len() > remaining_bytes {
            let stream_type = bytes_reader.read_u8()?;
            let elementary_stream_id = bytes_reader.read_u8()?;
            let elementary_stream_info_length = bytes_reader.read_u16::<BigEndian>()?;

            let (pseudo_descriptor_tag, pseudo_descriptor_length, elementary_stream_id_extension) =
                if elementary_stream_id == 0xFD && self.single_extension_stream_flag == 0 {
                    let pseudo_descriptor_tag = bytes_reader.read_u8()?;
                    let pseudo_descriptor_length = bytes_reader.read_u8()?;
                    let elementary_stream_id_extension = bytes_reader.read_u8()? & 0x7F;
                    bytes_reader.read_bytes(elementary_stream_info_length as usize - 3)?;
                    (
                        pseudo_descriptor_tag,
                        pseudo_descriptor_length,
                        elementary_stream_id_extension,
                    )
                } else {
                    bytes_reader.read_bytes(elementary_stream_info_length as usize)?;
                    (0, 0, 0)
                };

            self.stream_map.push(ElementaryStreamMap {
                stream_type,
                elementary_stream_id,
                elementary_stream_info_length,
                pseudo_descriptor_tag,
                pseudo_descriptor_length,
                elementary_stream_id_extension,
            });
        }
        bytes_reader.read_bytes(4)?;
        // log::info!("psm end");
        Ok(())
    }
}
