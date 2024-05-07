use std::{
    collections::VecDeque,
    sync::{
        Arc,
        Mutex
    }
};
use clap::{arg, Command};

use dora_node_api::{
    DoraNode,
    Event,
    IntoArrow,
    arrow::array::UInt8Array,
    dora_core::config::DataId
};

use simple_logger::SimpleLogger;
use tokio::net::TcpStream;

struct Packet {
    topic: String,
    data: Vec<u8>
}

struct Shared {
    running: bool,
    packets: VecDeque<Packet>
}

async fn run_listener(listener: String, shared: Arc<Mutex<Shared>>) {
    let socket = tokio::net::UdpSocket::bind(listener.clone()).await.unwrap();

    log::info!("Listener: {}", listener);

    loop {
        // Receive a packet, read it and create a new Packet struct

        let mut buf = [0; 1024];

        let status = socket.try_recv(&mut buf);
        if let Ok(len) = status {
            let data = buf[..len].to_vec();
            let data = String::from_utf8(data).unwrap();

            println!("Listener {} received data from sender: {}", listener, data);
        }

        let shared = shared.lock().unwrap();

        if !shared.running {
            break;
        }
    }
}


async fn run(host: String, sender: String, shared: Arc<Mutex<Shared>>) {
    let (mut node, mut events) = DoraNode::init_from_env().unwrap();

    let socket = tokio::net::UdpSocket::bind(host.clone()).await.unwrap();
    socket.connect(sender.clone()).await.unwrap();

    log::info!("Sender: {}", sender);
    log::info!("node running on : {}", socket.local_addr().unwrap());

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

                        println!("len data {}", final_data.len());

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

    let mut shared = shared.lock().unwrap();
    shared.running = false;
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    SimpleLogger::new().init().unwrap();

    let matches = Command::new("Electrode").about("Dora node for communication between dataflow").arg(arg!(--address <VALUE>).required(true)).arg(arg!(--listener <VALUE>).required(true)).arg(arg!(--sender <VALUE>).required(true)).get_matches();

    let (address, listener, sender) = (matches.get_one::<String>("address").expect("required").clone(), matches.get_one::<String>("listener").expect("required").clone(), matches.get_one::<String>("sender").expect("required").clone());

    let listener = address.clone() + ":" + &listener;
    let sender = address.clone() + ":" + &sender;
    let host = address.clone() + ":0";

    let shared = Arc::new(Mutex::new(Shared {
        running: true,
        packets: VecDeque::new()
    }));

    let runtime = tokio::runtime::Runtime::new().unwrap();

    let listener_task = runtime.spawn(run_listener(listener, shared.clone()));
    let main_task = runtime.spawn(run(host, sender, shared.clone()));

    let listener = runtime.block_on(listener_task);
    let main = runtime.block_on(main_task);

    return match (listener, main) {
        (Ok(_), Ok(_)) => Ok(()),
        (Err(e), _) | (_, Err(e)) => Err(e.into()),
    };
}
