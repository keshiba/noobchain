#![feature(trivial_bounds)]
#![feature(once_cell)]
extern crate pretty_env_logger;
#[macro_use] extern crate log;

mod app;

use crate::app::p2p::EventType;
use chrono::prelude::*;
use libp2p::{
    core::upgrade,
    futures::StreamExt,
    mplex,
    noise::{Keypair, NoiseConfig, X25519Spec},
    swarm::{Swarm, SwarmBuilder},
    tcp::TokioTcpConfig,
    Transport,
};
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::Duration;
use tokio::{
    io::{stdin, AsyncBufReadExt, BufReader},
    select, spawn,
    sync::mpsc,
    time::sleep,
};

use std::io::BufRead;
use tokio::sync::mpsc::unbounded_channel;
use crate::app::App;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    info!("Peer Id: {}", app::p2p::PEER_ID.clone());
    let (response_sender, mut response_rcv) = unbounded_channel();
    let (init_sender, mut init_rcv) = unbounded_channel();

    let auth_keys = Keypair::<X25519Spec>::new()
        .into_authentic(&app::p2p::KEYS)
        .expect("can create auth keys");

    let transp = TokioTcpConfig::new()
        .upgrade(upgrade::Version::V1)
        .authenticate(NoiseConfig::xx(auth_keys).into_authenticated())
        .multiplex(mplex::MplexConfig::new())
        .boxed();

    let behaviour = app::p2p::AppBehavior::new(App::new(), response_sender, init_sender.clone()).await;

    let mut swarm = SwarmBuilder::new(transp, behaviour, *app::p2p::PEER_ID)
        .executor(Box::new(|fut| {
            spawn(fut);
        }))
        .build();

    let mut stdin = BufReader::new(stdin()).lines();

    Swarm::listen_on(
        &mut swarm,
        "/ip4/0.0.0.0/tcp/0"
            .parse()
            .expect("can get a local socket"),
    )
    .expect("swarm can be started");

    spawn(async move {
        sleep(Duration::from_secs(1)).await;
        info!("sending init event");
        init_sender.send(true).expect("can send init event");
    });

    loop {
        let evt = {
            select! {
                line = stdin.next_line() => Some(EventType::Input(line.expect("can get line").expect("can read line from stdin"))),
                response = response_rcv.recv() => {
                    Some(EventType::LocalChainResponse(response.expect("response exists")))
                },
                _init = init_rcv.recv() => {
                    Some(EventType::Init)
                }
                event = swarm.select_next_some() => {
                    info!("Unhandled Swarm Event: {:?}", event);
                    None
                },
            }
        };

        if let Some(event) = evt {
            match event {
                app::p2p::EventType::Init => {
                    let peers = app::p2p::get_list_peers(&swarm);
                    swarm.behaviour_mut().app.genesis();

                    info!("connected nodes: {}", peers.len());
                    if !peers.is_empty() {
                        let req = app::p2p::LocalChainRequest {
                            from_peer_id: peers
                                .iter()
                                .last()
                                .expect("at least one peer")
                                .to_string(),
                        };

                        let json = serde_json::to_string(&req).expect("can jsonify request");
                        swarm
                            .behaviour_mut()
                            .floodsub
                            .publish(app::p2p::CHAIN_TOPIC.clone(), json.as_bytes());
                    }
                }
                app::p2p::EventType::LocalChainResponse(resp) => {
                    let json = serde_json::to_string(&resp).expect("can jsonify response");
                    swarm
                        .behaviour_mut()
                        .floodsub
                        .publish(app::p2p::CHAIN_TOPIC.clone(), json.as_bytes());
                }
                app::p2p::EventType::Input(line) => match line.as_str() {
                    "ls p" => app::p2p::handle_print_peers(&swarm),
                    cmd if cmd.starts_with("ls c") => app::p2p::handle_print_chain(&swarm),
                    cmd if cmd.starts_with("create b") => app::p2p::handle_create_block(cmd, &mut swarm),
                    _ => error!("unknown command"),
                },
            }
        }
    }
}