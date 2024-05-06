from dora import Node

node = Node()

event = node.next()
if event["type"] == "INPUT":
    print(
        f"""Node received:
    id: {event["id"]},
    value: {event["value"]},
    metadata: {event["metadata"]}"""
    )
