use std::{
    collections::VecDeque,
    sync::{
        Arc,
        Mutex
    }
};

use dora_node_api::{
    DoraNode,
    Event,
    IntoArrow,
    arrow::array::UInt8Array,
    dora_core::config::DataId
};

use simple_logger::SimpleLogger;

struct Packet {
    topic: String,
    data: Vec<u8>
}

struct Shared {
    running: bool,
    packets: VecDeque<Packet>
}

async fn run_listener(shared: Arc<Mutex<Shared>>) {
    let args: Vec<String> = std::env::args().collect();
    let listener = args.get(2).unwrap().clone();
    log::info!("Electrode Listener: {}", listener);

    let socket = tokio::net::UdpSocket::bind(listener).await.unwrap();

    loop {
        // Receive a packet

        let shared = shared.lock().unwrap();

        if !shared.running {
            break;
        }
    }
}


async fn run(shared: Arc<Mutex<Shared>>) {
    let (mut node, mut events) = DoraNode::init_from_env().unwrap();

    let args: Vec<String> = std::env::args().collect();
    let sender = args.get(4).unwrap().clone();
    log::info!("Electrode Sender: {}", sender);

    let socket = tokio::net::UdpSocket::bind(sender).await.unwrap();

    while let Some(event) = events.recv() {
        match event {
            Event::Input {
                id,
                metadata,
                data,
            } => {
                match id.as_str() {
                    "tick" => {
                        // Propagate every packet received from UDPListener into the dataflow

                        let mut shared = shared.lock().unwrap();
                        while let Some(packet) = shared.packets.pop_front() {
                            let topic = packet.topic;
                            let data = packet.data;

                            let status = node.send_output(
                                DataId::from(topic),
                                metadata.parameters.clone(),
                                data.into_arrow(),
                            );

                            if let Err(e) = status {
                                log::error!("Error sending output: {}", e);
                            }
                        }
                    },
                    other => {
                        // Send data received from dataflow to UDPListener

                        let buffer: UInt8Array = data.to_data().into();
                        let data = buffer.values();
                        let topic = other.as_bytes();
                        let topic_size = topic.len() as u8;

                        let mut final_data = Vec::new();
                        final_data.push(topic_size);
                        final_data.extend_from_slice(topic);
                        final_data.extend_from_slice(data);

                        let status = socket.send(
                            final_data.as_slice(),
                        ).await;

                        if let Err(e) = status {
                            log::error!("Error sending data: {}", e);
                        }
                    }
                }
            },
            Event::Stop => {
                let mut shared = shared.lock().unwrap();
                shared.running = false;

                break;
            },
            _ => {}
        }
    }

    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    let mut shared = shared.lock().unwrap();
    shared.running = false;
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    SimpleLogger::new().init().unwrap();

    log::info!("Starting Dora Node Electrode");

    let shared = Arc::new(Mutex::new(Shared {
        running: true,
        packets: VecDeque::new()
    }));

    let runtime = tokio::runtime::Runtime::new().unwrap();

    let listener_task = runtime.spawn(run_listener(shared.clone()));
    let main_task = runtime.spawn(run(shared.clone()));

    let listener = runtime.block_on(listener_task);
    let main = runtime.block_on(main_task);

    log::info!("Dora Node Electrode stopped");

    return match (listener, main) {
        (Ok(_), Ok(_)) => Ok(()),
        (Err(e), _) | (_, Err(e)) => Err(e.into()),
    };
}
