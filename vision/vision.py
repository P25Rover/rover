import sys

sys.path.append('/Users/enzolevan/Documents/Informatique/Development/p25rover/rover/venv/lib/python3.12/site-packages')

from dora import Node

node = Node()

event = node.next()
for event in node:
    if event["type"] == "INPUT":
        pass
    elif event["type"] == "STOP":
        break
