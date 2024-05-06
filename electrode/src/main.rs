use dora_node_api::{DoraNode, Event, EventStream, IntoArrow};
use std::error::Error;
use tokio::net::UdpSocket;

async fn run_listener(node: &mut DoraNode) {
    let listener = env!("LISTENER");

    let socket = UdpSocket::bind(listener).await?;
    let mut buf = [0; 1024 * 1024];

    loop {
        let (len, addr) = socket.recv_from(&mut buf).await?;
    }
}

async fn run(node: &mut DoraNode, events: &mut EventStream) {
    let listener = run_listener(node);

    while let Some(event) = events.recv() {
        match event {
            Event::Input {
                id,
                metadata,
                data: _,
            } => match id.as_str() {
                other => eprintln!("Received input `{other}`"),
            },
            _ => {}
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let (mut node, mut events) = DoraNode::init_from_env()?;

    pollster::block_on(run(&mut node, &mut events));

    Ok(())
}
