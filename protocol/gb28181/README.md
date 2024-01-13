A gb28181 library.



- 启动 RTMP 和 GB28181 api server

        xiu -r 1935 -g 3000

- 使用如下命令开启UDP端口，创建UDP server socket，接收PS数据，并自动转封装成RTMP

        curl -X POST -d "stream_id=test&secret=xiu&port=30000"  "http://localhost:3000/index/api/openRtpServer"

    会返回本地upd端口：

        {"code":0,"port":30000}

 所以port参数也可以传递0，server端会随机选择一个端口并返回。

 可以开启dump录制文件，用于重现问题。
        
        curl -X POST -d "stream_id=test&secret=xiu&port=30000&need_dump=true"  "http://localhost:3000/index/api/openRtpServer"

- 使用如下命令关闭udp socket
    
        curl -X POST -d "stream_id=test&secret=xiu"  "http://localhost:3000/index/api/clostRtpServer"
    

- RTMP拉流地址：

        rtmp://localhost:1935/gb28181/{stream_id}

比如上面的例子就是 : 
        
        rtmp://localhost:1935/gb28181/test

