const FLV_TAG_HEADER_SIZE: i8 = 11;
const FLV_PREVIOUS_TAG_SIZE: i8 = 4;

const RTMP_MSG_SetChunkSize: i8 = 0x01;
const RTMP_MSG_AbortMessage: i8 = 0x02;
const RTMP_MSG_Acknowledgement: i8 = 0x03;
const RTMP_MSG_UserControlMessage: i8 = 0x04;
const RTMP_MSG_WindowAcknowledgementSize: i8 = 0x05;
const RTMP_MSG_SetPeerBandwidth: i8 = 0x06;
const RTMP_MSG_EdgeAndOriginServerCommand: i8 = 0x07;

const RTMP_MSG_AMF3CommandMessage: i8 = 17; // 0x11
const RTMP_MSG_AMF0CommandMessage: i8 = 20; // 0x14
/**
3.2. Data message
The client or the server sends this message to send Metadata or any
user data to the peer. Metadata includes details about the
data(audio, video etc.) like creation time, duration, theme and so
on. These messages have been assigned message type value of 18 for
AMF0 and message type value of 15 for AMF3.
*/
const RTMP_MSG_AMF0DataMessage: i8 = 18; // 0x12
const RTMP_MSG_AMF3DataMessage: i8 = 15; // 0x0F
/**
3.3. Shared object message
A shared object is a Flash object (a collection of name value pairs)
that are in synchronization across multiple clients, instances, and
so on. The message types kMsgContainer=19 for AMF0 and
kMsgContainerEx=16 for AMF3 are reserved for shared object events.
Each message can contain multiple events.
*/
const RTMP_MSG_AMF3SharedObject: i8 = 16; // 0x10
const RTMP_MSG_AMF0SharedObject: i8 = 19; // 0x13
/**
3.4. Audio message
The client or the server sends this message to send audio data to the
peer. The message type value of 8 is reserved for audio messages.
*/
const RTMP_MSG_AudioMessage: i8 = 8; // 0x08
                                     /* *
                                     3.5. Video message
                                     The client or the server sends this message to send video data to the
                                     peer. The message type value of 9 is reserved for video messages.
                                     These messages are large and can delay the sending of other type of
                                     messages. To avoid such a situation, the video message is assigned
                                     the lowest priority.
                                     */
const RTMP_MSG_VideoMessage: i8 = 9; // 0x09
/**
3.6. Aggregate message
An aggregate message is a single message that contains a list of submessages.
The message type value of 22 is reserved for aggregate
messages.
*/
const RTMP_MSG_AggregateMessage: i8 = 22; // 0x16

/****************************************************************************
 *****************************************************************************
 ****************************************************************************/
/**
 * the chunk stream id used for some under-layer message,
 * for example, the PC(protocol control) message.
 */
const RTMP_CID_ProtocolControl: i8 = 0x02;
/**
 * the AMF0/AMF3 command message, invoke method and return the result, over NetConnection.
 * generally use 0x03.
 */
const RTMP_CID_OverConnection: i8 = 0x03;
/**
 * the AMF0/AMF3 command message, invoke method and return the result, over NetConnection,
 * the midst state(we guess).
 * rarely used, e.g. onStatus(NetStream.Play.Reset).
 */
const RTMP_CID_OverConnection2: i8 = 0x04;
/**
 * the stream message(amf0/amf3), over NetStream.
 * generally use 0x05.
 */
const RTMP_CID_OverStream: i8 = 0x05;
/**
 * the stream message(amf0/amf3), over NetStream, the midst state(we guess).
 * rarely used, e.g. play("mp4:mystram.f4v")
 */
const RTMP_CID_OverStream2: i8 = 0x08;
/**
 * the stream message(video), over NetStream
 * generally use 0x06.
 */
const RTMP_CID_Video: i8 = 0x06;
/**
 * the stream message(audio), over NetStream.
 * generally use 0x07.
 */
const RTMP_CID_Audio: i32 = 0x07;

/**
 * 6.1. Chunk Format
 * Extended timestamp: 0 or 4 bytes
 * This field MUST be sent when the normal timsestamp is set to
 * 0xffffff, it MUST NOT be sent if the normal timestamp is set to
 * anything else. So for values less than 0xffffff the normal
 * timestamp field SHOULD be used in which case the extended timestamp
 * MUST NOT be present. For values greater than or equal to 0xffffff
 * the normal timestamp field MUST NOT be used and MUST be set to
 * 0xffffff and the extended timestamp MUST be sent.
 */
const RTMP_EXTENDED_TIMESTAMP: i32 = 0xFFFFFF;

struct MessageHeader {
    timestamp_delta: i32,
    payload_length: i32,
    message_type: i8,
    stream_id: i32,
    timestamp: i64,
    perfer_cid: i32,
}

impl MessageHeader {
    pub fn is_audio(self) -> bool {
        self.message_type == RTMP_MSG_AudioMessage
    }

    fn is_video(self) {
        self.message_type == RTMP_MSG_VideoMessage
    }

    fn is_amf0_command(self) {
        self.message_type == RTMP_MSG_AMF0CommandMessage
    }

    fn is_amf0_data(self) {
        self.message_type == RTMP_MSG_AMF0DataMessage
    }

    fn is_amf3_command(self) {
        self.message_type == RTMP_MSG_AMF3CommandMessage
    }

    fn is_amf3_data(self) {
        self.message_type == RTMP_MSG_AMF3DataMessage
    }

    fn is_window_ackledgement_size(self) {
        self.message_type == RTMP_MSG_WindowAcknowledgementSize
    }

    fn is_ackledgement(self) {
        self.message_type == RTMP_MSG_Acknowledgement
    }

    fn is_set_chunk_size(self) {
        self.message_type == RTMP_MSG_SetChunkSize
    }

    fn is_user_control_message(self) {
        self.message_type == RTMP_MSG_UserControlMessage
    }

    fn is_set_peer_bandwidth(self) {
        self.message_type == RTMP_MSG_SetPeerBandwidth
    }

    fn is_aggregate(self) {
        self.message_type == RTMP_MSG_AggregateMessage
    }

    fn initialize_amf0_script(&mut self, size: i32, stream: i32) {
        self.message_type = RTMP_MSG_AMF0DataMessage;
        self.payload_length = size;
        self.timestamp_delta = 0;
        self.timestamp = 0;
        self.stream_id = stream;
    }
    fn initialize_audio(&mut self, size: i32, time: i64, stream: i32) {
        self.message_type = RTMP_MSG_AudioMessage;
        self.payload_length = size;
        self.timestamp_delta = time;
        self.timestamp = time;
        self.stream_id = stream;

        // audio chunk-id
        self.perfer_cid = RTMP_CID_Audio;
    }

    fn initialize_video(&mut self, size: i32, time: i64, stream: i32) {
        self.message_type = RTMP_MSG_VideoMessage;
        self.payload_length = size;
        self.timestamp_delta = time;
        self.timestamp = time;
        self.stream_id = stream;
        // video chunk-id
        self.perfer_cid = RTMP_CID_Video;
    }
}
