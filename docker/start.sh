#!/bin/sh

#sudo docker run -d -it --net=host --name xiu --privileged=true -v /Users/username/logs:/app/logs harlancn/xiu:0.1.49 /app/start.sh /app/config.toml 
echo "Specify configuration file path"
/app/xiu -c $1
