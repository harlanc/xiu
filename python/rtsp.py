import cv2
import os

os.environ["OPENCV_FFMPEG_CAPTURE_OPTIONS"] = "rtsp_transport;tcp"

stream_url = "rtsp://10.10.181.175:9554/aioa/112233"
cap = cv2.VideoCapture(stream_url, cv2.CAP_FFMPEG)
# cap.set(cv2.CAP_PROP_FFMPEG_OPTIONS, "rtsp_transport=tcp")

if not cap.isOpened():
    raise ConnectionError("rtsp open failed")

while True:
    ret, frame = cap.read()
    if not ret:
        break
    
    cv2.imshow("rtsp stream", frame)
    if cv2.waitKey(1) & 0xFF == ord('q'):
        break
cap.release()
cv2.destroyAllWindows()