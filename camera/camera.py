from dora import Node
import cv2
import pyarrow as pa

node = Node()

video_capture = cv2.VideoCapture(0)
video_capture.set(cv2.CAP_PROP_FRAME_WIDTH, 320)
video_capture.set(cv2.CAP_PROP_FRAME_HEIGHT, 240)

for event in node:
    if event["type"] == "INPUT":
        ret, frame = video_capture.read()
        if ret:
            frame = cv2.resize(frame, (320, 240))

            node.send_output(
                "image",
                pa.array(frame.ravel()),
                event["metadata"]
            )
