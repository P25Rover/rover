use std::collections::VecDeque;
use dora_node_api::{DoraNode, Event, EventStream, IntoArrow};
use std::error::Error;
use std::ops::DerefMut;
use std::str::from_utf8;
use std::sync::{Arc, Mutex};
use dora_node_api::arrow::array::{Array, UInt8Array};
use dora_node_api::dora_core::config::DataId;
use tokio::net::UdpSocket;

struct Packet {
    topic: String,
    data: Vec<u8>,
}

async fn run_listener(packets: Arc<Mutex<VecDeque<Packet>>>) {
    let listener = env!("LISTENER");

    let socket = UdpSocket::bind(listener).await.unwrap();
    let mut buf = [0; 1024 * 1024];

    loop {
        let (len, addr) = socket.recv_from(&mut buf).await.unwrap();

        let topic_size = buf[0];
        let topic_utf8 = &buf[1..(1 + topic_size as usize)];
        let topic_str = from_utf8(&topic_utf8).unwrap().to_string();

        let data = (&buf[(1 + topic_size as usize)..1024 * 1024]).to_vec();

        let mut packets = packets.lock().unwrap();
        packets.deref_mut().push_back(Packet {
            topic: topic_str,
            data,
        });
    }
}


async fn run(node: &mut DoraNode, events: &mut EventStream) {
    let packets: Arc<Mutex<VecDeque<Packet>>> = Arc::new(Mutex::new(VecDeque::new()));

    let listener = run_listener(packets.clone());

    let sender = env!("SENDER");
    let socket = UdpSocket::bind(sender).await.unwrap();

    while let Some(event) = events.recv() {
        match event {
            Event::Input {
                id,
                metadata,
                data,
            } => match id.as_str() {
                "tick" => {
                    let mut packets_data = packets.lock().unwrap();
                    while !packets_data.deref_mut().is_empty() {
                        let Packet {
                            topic,
                            data
                        } = packets_data.deref_mut().pop_front().unwrap();

                        node.send_output(
                            DataId::from(topic),
                            metadata.parameters.clone(),
                            data.into_arrow(),
                        ).unwrap();

                        println! ("data received from udp {}", topic);
                    }
                }
                other => {
                    let buffer: UInt8Array = data.to_data().into();

                    println!("data received from dataflow {}", other);

                    socket.send(buffer.values()).await.expect("TODO: panic message");
                }
            },
            _ => {}
        }
    }

    listener.await;
}

fn main() -> Result<(), Box<dyn Error>> {
    let (mut node, mut events) = DoraNode::init_from_env()?;

    pollster::block_on(run(&mut node, &mut events));

    Ok(())
}
