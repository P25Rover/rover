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

    let matches = Command::new("Electrode").about("Dora node for communication between dataflow").arg(arg!(--address <VALUE>).required(true)).arg(arg!(--listener <VALUE>).required(true)).arg(arg!(--sender <VALUE>).required(true)).get_matches();

    let (address, listener, sender) = (matches.get_one::<String>("address").expect("required").clone(), matches.get_one::<String>("listener").expect("required").clone(), matches.get_one::<String>("sender").expect("required").clone());
    let address = format!("udp/{}", address);

    let mut config = config::peer();
    config.connect.endpoints.push(address.parse().unwrap());

    let session = zenoh::open(config).res().await.unwrap();

    let listener_expression = format!("{}/*", listener);

    let listener_packets = packets.clone();

    let listener = session.declare_subscriber(listener_expression).callback(move |sample| {
        // remove the listener prefix from the topic

        let topic = sample.key_expr.to_string();
        let topic = topic.replace(&format!("{}/", listener), "");

        let packet = Packet {
            topic,
            data: sample.value.payload.contiguous().to_vec()
        };

        listener_packets.lock().unwrap().push_back(packet);
    });

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

                        while let Some(packet) = packets.lock().unwrap().pop_front() {
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
