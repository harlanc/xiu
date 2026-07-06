use std::fmt::{self, Display, Debug};
use std::os::raw::{c_uint, c_int};

use fdk_aac_sys as sys;

pub use sys::CStreamInfo as StreamInfo;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct DecoderError(sys::AAC_DECODER_ERROR);

impl DecoderError {
    pub const OUT_OF_MEMORY: DecoderError = DecoderError(sys::AAC_DECODER_ERROR_AAC_DEC_OUT_OF_MEMORY);
    pub const UNKNOWN: DecoderError = DecoderError(sys::AAC_DECODER_ERROR_AAC_DEC_UNKNOWN);
    pub const TRANSPORT_SYNC_ERROR: DecoderError = DecoderError(sys::AAC_DECODER_ERROR_AAC_DEC_TRANSPORT_SYNC_ERROR);
    pub const NOT_ENOUGH_BITS: DecoderError = DecoderError(sys::AAC_DECODER_ERROR_AAC_DEC_NOT_ENOUGH_BITS);
    pub const INVALID_HANDLE: DecoderError = DecoderError(sys::AAC_DECODER_ERROR_AAC_DEC_INVALID_HANDLE);
    pub const UNSUPPORTED_AOT: DecoderError = DecoderError(sys::AAC_DECODER_ERROR_AAC_DEC_UNSUPPORTED_AOT);
    pub const UNSUPPORTED_FORMAT: DecoderError = DecoderError(sys::AAC_DECODER_ERROR_AAC_DEC_UNSUPPORTED_FORMAT);
    pub const UNSUPPORTED_ER_FORMAT: DecoderError = DecoderError(sys::AAC_DECODER_ERROR_AAC_DEC_UNSUPPORTED_ER_FORMAT);
    pub const UNSUPPORTED_EPCONFIG: DecoderError = DecoderError(sys::AAC_DECODER_ERROR_AAC_DEC_UNSUPPORTED_EPCONFIG);
    pub const UNSUPPORTED_MULTILAYER: DecoderError = DecoderError(sys::AAC_DECODER_ERROR_AAC_DEC_UNSUPPORTED_MULTILAYER);
    pub const UNSUPPORTED_CHANNELCONFIG: DecoderError = DecoderError(sys::AAC_DECODER_ERROR_AAC_DEC_UNSUPPORTED_CHANNELCONFIG);
    pub const UNSUPPORTED_SAMPLINGRATE: DecoderError = DecoderError(sys::AAC_DECODER_ERROR_AAC_DEC_UNSUPPORTED_SAMPLINGRATE);
    pub const INVALID_SBR_CONFIG: DecoderError = DecoderError(sys::AAC_DECODER_ERROR_AAC_DEC_INVALID_SBR_CONFIG);
    pub const SET_PARAM_FAIL: DecoderError = DecoderError(sys::AAC_DECODER_ERROR_AAC_DEC_SET_PARAM_FAIL);
    pub const NEED_TO_RESTART: DecoderError = DecoderError(sys::AAC_DECODER_ERROR_AAC_DEC_NEED_TO_RESTART);
    pub const OUTPUT_BUFFER_TOO_SMALL: DecoderError = DecoderError(sys::AAC_DECODER_ERROR_AAC_DEC_OUTPUT_BUFFER_TOO_SMALL);
    pub const TRANSPORT_ERROR: DecoderError = DecoderError(sys::AAC_DECODER_ERROR_AAC_DEC_TRANSPORT_ERROR);
    pub const PARSE_ERROR: DecoderError = DecoderError(sys::AAC_DECODER_ERROR_AAC_DEC_PARSE_ERROR);
    pub const UNSUPPORTED_EXTENSION_PAYLOAD: DecoderError = DecoderError(sys::AAC_DECODER_ERROR_AAC_DEC_UNSUPPORTED_EXTENSION_PAYLOAD);
    pub const DECODE_FRAME_ERROR: DecoderError = DecoderError(sys::AAC_DECODER_ERROR_AAC_DEC_DECODE_FRAME_ERROR);
    pub const CRC_ERROR: DecoderError = DecoderError(sys::AAC_DECODER_ERROR_AAC_DEC_CRC_ERROR);
    pub const INVALID_CODE_BOOK: DecoderError = DecoderError(sys::AAC_DECODER_ERROR_AAC_DEC_INVALID_CODE_BOOK);
    pub const UNSUPPORTED_PREDICTION: DecoderError = DecoderError(sys::AAC_DECODER_ERROR_AAC_DEC_UNSUPPORTED_PREDICTION);
    pub const UNSUPPORTED_CCE: DecoderError = DecoderError(sys::AAC_DECODER_ERROR_AAC_DEC_UNSUPPORTED_CCE);
    pub const UNSUPPORTED_LFE: DecoderError = DecoderError(sys::AAC_DECODER_ERROR_AAC_DEC_UNSUPPORTED_LFE);
    pub const UNSUPPORTED_GAIN_CONTROL_DATA: DecoderError = DecoderError(sys::AAC_DECODER_ERROR_AAC_DEC_UNSUPPORTED_GAIN_CONTROL_DATA);
    pub const UNSUPPORTED_SBA: DecoderError = DecoderError(sys::AAC_DECODER_ERROR_AAC_DEC_UNSUPPORTED_SBA);
    pub const TNS_READ_ERROR: DecoderError = DecoderError(sys::AAC_DECODER_ERROR_AAC_DEC_TNS_READ_ERROR);
    pub const RVLC_ERROR: DecoderError = DecoderError(sys::AAC_DECODER_ERROR_AAC_DEC_RVLC_ERROR);
    pub const ANC_DATA_ERROR: DecoderError = DecoderError(sys::AAC_DECODER_ERROR_AAC_DEC_ANC_DATA_ERROR);
    pub const TOO_SMALL_ANC_BUFFER: DecoderError = DecoderError(sys::AAC_DECODER_ERROR_AAC_DEC_TOO_SMALL_ANC_BUFFER);
    pub const TOO_MANY_ANC_ELEMENTS: DecoderError = DecoderError(sys::AAC_DECODER_ERROR_AAC_DEC_TOO_MANY_ANC_ELEMENTS);

    pub fn message(&self) -> &'static str {
        match self.0 {
            sys::AAC_DECODER_ERROR_AAC_DEC_OK => "No error occurred. Output buffer is valid and error free.",
            sys::AAC_DECODER_ERROR_AAC_DEC_OUT_OF_MEMORY => "Heap returned NULL pointer. Output buffer is invalid.",
            sys::AAC_DECODER_ERROR_AAC_DEC_UNKNOWN => "Error condition is of unknown reason, or from a another module. Output buffer is invalid.",
            sys::AAC_DECODER_ERROR_aac_dec_sync_error_start => "Synchronization errors. Output buffer is invalid.",
            sys::AAC_DECODER_ERROR_AAC_DEC_TRANSPORT_SYNC_ERROR => "The transport decoder had synchronization problems. Do not exit decoding. Just feed new bitstream data.",
            sys::AAC_DECODER_ERROR_AAC_DEC_NOT_ENOUGH_BITS => "The input buffer ran out of bits.",
            sys::AAC_DECODER_ERROR_aac_dec_init_error_start => "Initialization errors. Output buffer is invalid.",
            sys::AAC_DECODER_ERROR_AAC_DEC_INVALID_HANDLE => "The handle passed to the function call was invalid (NULL).",
            sys::AAC_DECODER_ERROR_AAC_DEC_UNSUPPORTED_AOT => "The AOT found in the configuration is not supported.",
            sys::AAC_DECODER_ERROR_AAC_DEC_UNSUPPORTED_FORMAT => "The bitstream format is not supported. ",
            sys::AAC_DECODER_ERROR_AAC_DEC_UNSUPPORTED_ER_FORMAT => "The error resilience tool format is not supported.",
            sys::AAC_DECODER_ERROR_AAC_DEC_UNSUPPORTED_EPCONFIG => "The error protection format is not supported.",
            sys::AAC_DECODER_ERROR_AAC_DEC_UNSUPPORTED_MULTILAYER => "More than one layer for AAC scalable is not supported.",
            sys::AAC_DECODER_ERROR_AAC_DEC_UNSUPPORTED_CHANNELCONFIG => "The channel configuration (either number or arrangement) is not supported.",
            sys::AAC_DECODER_ERROR_AAC_DEC_UNSUPPORTED_SAMPLINGRATE => "The sample rate specified in the configuration is not supported.",
            sys::AAC_DECODER_ERROR_AAC_DEC_INVALID_SBR_CONFIG => "The SBR configuration is not supported.",
            sys::AAC_DECODER_ERROR_AAC_DEC_SET_PARAM_FAIL => "The parameter could not be set. Either the value was out of range or the parameter does  not exist.",
            sys::AAC_DECODER_ERROR_AAC_DEC_NEED_TO_RESTART => "The decoder needs to be restarted, since the required configuration change cannot be performed.",
            sys::AAC_DECODER_ERROR_AAC_DEC_OUTPUT_BUFFER_TOO_SMALL => "The provided output buffer is too small.",
            sys::AAC_DECODER_ERROR_aac_dec_decode_error_start => "Decode errors. Output buffer is valid but concealed.",
            sys::AAC_DECODER_ERROR_AAC_DEC_TRANSPORT_ERROR => "The transport decoder encountered an unexpected error.",
            sys::AAC_DECODER_ERROR_AAC_DEC_PARSE_ERROR => "Error while parsing the bitstream. Most probably it is corrupted, or the system crashed.",
            sys::AAC_DECODER_ERROR_AAC_DEC_UNSUPPORTED_EXTENSION_PAYLOAD => "Error while parsing the extension payload of the bitstream. The extension payload type found is not supported.",
            sys::AAC_DECODER_ERROR_AAC_DEC_DECODE_FRAME_ERROR => "The parsed bitstream value is out of range. Most probably the bitstream is corrupt, or the system crashed.",
            sys::AAC_DECODER_ERROR_AAC_DEC_CRC_ERROR => "The embedded CRC did not match.",
            sys::AAC_DECODER_ERROR_AAC_DEC_INVALID_CODE_BOOK => "An invalid codebook was signaled. Most probably the bitstream is corrupt, or the system  crashed.",
            sys::AAC_DECODER_ERROR_AAC_DEC_UNSUPPORTED_PREDICTION => "Predictor found, but not supported in the AAC Low Complexity profile. Most probably the bitstream is corrupt, or has a wrong format.",
            sys::AAC_DECODER_ERROR_AAC_DEC_UNSUPPORTED_CCE => "A CCE element was found which is not supported. Most probably the bitstream is corrupt, or has a wrong format.",
            sys::AAC_DECODER_ERROR_AAC_DEC_UNSUPPORTED_LFE => "A LFE element was found which is not supported. Most probably the bitstream is corrupt, or has a wrong format.",
            sys::AAC_DECODER_ERROR_AAC_DEC_UNSUPPORTED_GAIN_CONTROL_DATA => "Gain control data found but not supported. Most probably the bitstream is corrupt, or has a wrong format.",
            sys::AAC_DECODER_ERROR_AAC_DEC_UNSUPPORTED_SBA => "SBA found, but currently not supported in the BSAC profile.",
            sys::AAC_DECODER_ERROR_AAC_DEC_TNS_READ_ERROR => "Error while reading TNS data. Most probably the bitstream is corrupt or the system crashed.",
            sys::AAC_DECODER_ERROR_AAC_DEC_RVLC_ERROR => "Error while decoding error resilient data.",
            sys::AAC_DECODER_ERROR_aac_dec_anc_data_error_start => "Ancillary data errors. Output buffer is valid.",
            sys::AAC_DECODER_ERROR_AAC_DEC_ANC_DATA_ERROR => "Non severe error concerning the ancillary data handling.",
            sys::AAC_DECODER_ERROR_AAC_DEC_TOO_SMALL_ANC_BUFFER => "The registered ancillary data buffer is too small to receive the parsed data.",
            sys::AAC_DECODER_ERROR_AAC_DEC_TOO_MANY_ANC_ELEMENTS => "More than the allowed number of ancillary data elements should be written to buffer.",
            _ => "Unknown error",
        }
    }
}

impl Debug for DecoderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "DecoderError {{ code: {:?}, message: {:?} }}", self.0 as c_int, self.message())
    }
}

impl Display for DecoderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message())
    }
}

fn check(e: sys::AACENC_ERROR) -> Result<(), DecoderError> {
    if e == sys::AAC_DECODER_ERROR_AAC_DEC_OK {
        Ok(())
    } else {
        Err(DecoderError(e))
    }
}

#[derive(Debug)]
pub struct Decoder {
    handle: sys::HANDLE_AACDECODER,
}

unsafe impl Send for Decoder {}
unsafe impl Sync for Decoder {}

impl Decoder {
    pub fn new(transport: Transport) -> Self {
        let handle = match transport {
            Transport::Adts => {
                unsafe { sys::aacDecoder_Open(sys::TRANSPORT_TYPE_TT_MP4_ADTS, 1) }
            }
        };

        Decoder { handle }
    }

    pub fn config_raw(&mut self, audio_specic_config: &[u8]) -> Result<(), DecoderError> {
        unsafe {
            let mut asc_ptr = audio_specic_config.as_ptr() as *mut u8;
            let asc_len = audio_specic_config.len() as c_uint;
            check(sys::aacDecoder_ConfigRaw(self.handle, &mut asc_ptr as *mut _, &asc_len as *const _))
        }
    }

    pub fn set_min_output_channels(&mut self, channels: usize) -> Result<(), DecoderError> {
        unsafe {
            check(sys::aacDecoder_SetParam(self.handle,
                sys::AACDEC_PARAM_AAC_PCM_MIN_OUTPUT_CHANNELS,
                channels as i32))
        }
    }

    pub fn set_max_output_channels(&mut self, channels: usize) -> Result<(), DecoderError> {
        unsafe {
            check(sys::aacDecoder_SetParam(self.handle,
                sys::AACDEC_PARAM_AAC_PCM_MAX_OUTPUT_CHANNELS,
                channels as i32))
        }
    }

    pub fn fill(&mut self, data: &[u8]) -> Result<usize, DecoderError> {
        unsafe {
            let mut data_ptr = data.as_ptr() as *const u8 as *mut u8;
            let data_len = data.len() as c_uint;
            let mut bytes_valid: c_uint = data_len;

            check(sys::aacDecoder_Fill(self.handle,
                &mut data_ptr as *mut _,
                &data_len as *const _,
                &mut bytes_valid as *mut _))?;

            Ok(data.len() - bytes_valid as usize)
        }
    }

    pub fn decode_frame(&mut self, pcm: &mut [i16]) -> Result<(), DecoderError> {
        unsafe {
            check(sys::aacDecoder_DecodeFrame(self.handle,
                pcm.as_mut_ptr() as *mut i16,
                pcm.len() as c_int,
                0))
        }
    }

    pub fn decoded_frame_size(&self) -> usize {
        let stream_info = self.stream_info();

        stream_info.numChannels as usize * stream_info.frameSize as usize
    }

    pub fn stream_info(&self) -> &StreamInfo {
        unsafe { &*sys::aacDecoder_GetStreamInfo(self.handle) }
    }
}

impl Drop for Decoder {
    fn drop(&mut self) {
        unsafe { sys::aacDecoder_Close(self.handle); }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Transport {
    Adts,
}
