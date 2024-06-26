use std::{
    collections::VecDeque,
    sync::{
        Arc,
        Mutex
    }
};

use clap::{arg, Command};

use zenoh::{
    prelude::r#async::AsyncResolve,
    config,
    SessionDeclarations
};

use dora_node_api::{
    DoraNode,
    Event,
    IntoArrow,
    arrow::array::UInt8Array,
    dora_core::config::DataId
};

use simple_logger::SimpleLogger;
use zenoh::prelude::SplitBuffer;

struct Packet {
    topic: String,
    data: Vec<u8>
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let packets = Arc::new(Mutex::new(VecDeque::new()));

    let matches = Command::new("Electrode").version("1.0").about("Dora node for communication between dataflow").arg(arg!(--port <VALUE>).required(true)).arg(arg!(--protocol <VALUE>).required(true)).arg(arg!(--listen <VALUE>).required(true)).arg(arg!(--connect <VALUE>).required(true)).get_matches();

    let (listener, sender) = (matches.get_one::<String>("listen").expect("required").clone(), matches.get_one::<String>("connect").expect("required").clone());
    let (protocol, port) = (matches.get_one::<String>("protocol").expect("required").clone(), matches.get_one::<String>("port").expect("required").clone());

    let listener_session = format!("{}/{}:{}", protocol, listener, port);
    let sender_session = format!("{}/{}:{}", protocol, sender, port);

    let mut config = config::peer();
    config.listen.endpoints.push(listener_session.parse().unwrap());
    config.connect.endpoints.push(sender_session.parse().unwrap());

    let session = zenoh::open(config).res().await.unwrap();

    let listener_expression = format!("{}/*", listener);

    // remove the listener prefix from the topic
    let listener_packets = packets.clone();
    let subscriber = session.declare_subscriber(listener_expression).callback(move |sample| {
        let topic = sample.key_expr.to_string();
        let topic = topic.replace(&format!("{}/", listener), "");

        let packet = Packet {
            topic,
            data: sample.value.payload.contiguous().to_vec()
        };

        // If there is not more than 10 packets, push the packet to the queue
        if listener_packets.lock().unwrap().len() < 10 {
            listener_packets.lock().unwrap().push_back(packet);
        }
    }).res().await.unwrap();

    let (mut node, mut events) = DoraNode::init_from_env().unwrap();

    while let Some(event) = events.recv() {
        match event {
            Event::Input {
                id,
                metadata,
                data,
            } => {
                match id.as_str() {
                    "tick" => {
                        // Send data received from Zenoh to dataflow

                        let mut packets = packets.lock().unwrap();

                        while let Some(packet) = packets.pop_front() {
                            let data_id = DataId::from(packet.topic);
                            let data = packet.data.into_arrow();

                            let status = node.send_output(
                                data_id,
                                metadata.parameters.clone(),
                                data
                            );

                            if let Err(error) = status {
                                log::error!("Error sending data to dataflow: {}", error);
                            }
                        }
                    },
                    other => {
                        // Send data received from dataflow to Zenoh

                        let data_expr = format!("{}/{}", sender, other);
                        let buffer: UInt8Array = data.to_data().into();
                        let data = buffer.values().to_vec();

                        session.put(data_expr, data).res().await.unwrap();
                    }
                }
            },
            Event::Stop => {
                subscriber.undeclare().res().await.unwrap();

                session.close().res().await.unwrap();

                break;
            },
            _ => {}
        }
    }

    return Ok(());
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    SimpleLogger::new().init().unwrap();

    let runtime = tokio::runtime::Runtime::new().unwrap();

    return runtime.block_on(crate::run());
}
