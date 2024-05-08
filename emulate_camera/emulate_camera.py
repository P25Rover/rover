import sys

sys.path.append('/Users/enzolevan/Documents/Informatique/Development/p25rover/rover/venv/lib/python3.12/site-packages')

from dora import Node

import cv2
import pyarrow as pa
import numpy as np

node = Node()

video_capture = np.zeros((350, 500, 3), dtype=np.uint8)

cv2.putText(
    video_capture,
    "Emulated camera",
    (50, 50),
    cv2.FONT_HERSHEY_SIMPLEX,
    1,
    (255, 255, 255),
    2
)

for event in node:
    if event["type"] == "INPUT":
        data = pa.array(video_capture.ravel())

        node.send_output(
            "image",
            data,
            event["metadata"]
        )

    elif event["type"] == "STOP":
        break
