use super::session::GB28181ServerSession;
use bytesio::bytesio::UdpIO;

use streamhub::define::StreamHubEventSender;
use tokio::io::Error;

pub struct GB28181Server {
    local_port: u16,
    event_producer: StreamHubEventSender,
    stream_name: String,
    need_dump: bool,
}

impl GB28181Server {
    pub fn new(
        local_port: u16,
        event_producer: StreamHubEventSender,
        stream_name: String,
        need_dump: bool,
    ) -> Self {
        log::info!("GB28181Server new");
        Self {
            local_port,
            event_producer,
            stream_name,
            need_dump,
        }
    }

    pub async fn run(&mut self) -> Result<Option<u16>, Error> {
        // let socket_addr: &SocketAddr = &self.address.parse().unwrap();
        // let listener = TcpListener::bind(socket_addr).await?;

        //     if let Some(rtp_io) =
        //     UdpIO::new(address.clone(), Some(rtp_port), 0).await
        // {

        log::info!("GB28181Server run");

        if let Some(udp_id) = UdpIO::new(self.local_port, None).await {
            let local_port = udp_id.get_local_port();

            if let Ok(mut session) = GB28181ServerSession::new(
                udp_id,
                self.event_producer.clone(),
                self.stream_name.clone(),
                self.need_dump,
            ) {
                log::info!("GB28181 server listening on udp://{}", self.local_port);
                tokio::spawn(async move {
                    if let Err(err) = session.run().await {
                        log::error!("session run error, err: {}", err);
                    }
                });
            }

            return Ok(local_port);
        }

        // log::info!("GB28181 server listening on tcp://{}", socket_addr);
        // loop {
        //     let (tcp_stream, _) = listener.accept().await?;
        //     if let Ok(mut session) =
        //         GB28181ServerSession::new(tcp_stream, self.event_producer.clone())
        //     {
        //         tokio::spawn(async move {
        //             if let Err(err) = session.run().await {
        //                 log::error!("session run error, err: {}", err);
        //             }
        //         });
        //     }
        // }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use byteorder::{BigEndian, ReadBytesExt};
    use std::fs::File;
    use std::io::{self, Read};
    use std::net::UdpSocket;

    use std::thread::{self, sleep};
    use std::time::Duration;
    #[test]
    fn send_dump_file() {
        let file_path = "/Users/zexu/Downloads/dump2"; // 替换为实际的文件路径
        let mut file = File::open(file_path).unwrap();

        // 创建 UDP socket
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap(); // 绑定到任意可用端口

        loop {
            let time_delta = match file.read_u16::<BigEndian>() {
                Ok(value) => value,
                Err(err) => {
                    log::error!("file read error: {}", err);
                    break;
                } // 文件已读取完毕或发生错误
            };
            sleep(Duration::from_millis(time_delta as u64));

            // 读取 10 个字节
            // 读取 4 个字节作为大端 u32
            let length = match file.read_u16::<BigEndian>() {
                Ok(value) => value,
                Err(err) => {
                    log::error!("file read error: {}", err);
                    break;
                } // 文件已读取完毕或发生错误
            };
            println!("length:{}", length);

            // 读取指定长度的字节
            let mut buffer = vec![0u8; length as usize];
            file.read_exact(&mut buffer);

            // 发送数据到 UDP 端口
            let addr = "127.0.0.1:30000"; // UDP 目标地址
            let sent_bytes = socket.send_to(&buffer, addr).unwrap();
            //  println!("Sent {} bytes to {}: {:?}", sent_bytes, addr, buffer);
            thread::sleep(Duration::from_millis(2));
        }
    }
}
