#![feature(once_cell)]
#![feature(trivial_bounds)]

extern crate pretty_env_logger;
#[macro_use] extern crate log;

mod app;

use std::time::Duration;

use libp2p::{
    core::upgrade,
    noise::{ X25519Spec, Keypair, NoiseConfig },
    tcp::TokioTcpConfig,
    Transport, 
    mplex, 
    swarm:: { SwarmBuilder, Swarm }, 
    futures::StreamExt
};
use tokio::{
    sync::mpsc, 
    time,
    io::{ stdin, BufReader, AsyncBufReadExt }
};
use crate::app::{
    App,
    p2p::{self, EventType},
};

#[tokio::main]
async fn main() {

    pretty_env_logger::init();

    info!("Peer Id: {}", p2p::PEER_ID.clone());
    let (response_sender, mut response_rcv):
        (tokio::sync::mpsc::UnboundedSender<p2p::ChainResponse>, tokio::sync::mpsc::UnboundedReceiver<p2p::ChainResponse>) = 
        mpsc::unbounded_channel();
    let (init_sender, mut init_rcv):
        (tokio::sync::mpsc::UnboundedSender<p2p::EventType>, tokio::sync::mpsc::UnboundedReceiver<p2p::EventType>) = 
        mpsc::unbounded_channel();
    
    let auth_keys = Keypair::<X25519Spec>::new()
        .into_authentic(&p2p::KEYS)
        .expect("can create auth keys");

    let transport = 
        TokioTcpConfig::new()
            .ttl(u32::MAX)
            .upgrade(upgrade::Version::V1)
            .authenticate(NoiseConfig::xx(auth_keys).into_authenticated())
            .multiplex(mplex::MplexConfig::new())
            .boxed();

    let app_behaviour = 
        p2p::AppBehaviour::new(App::new(), response_sender, init_sender.clone()).await;
    
    let mut swarm = 
        SwarmBuilder::new(transport, app_behaviour, *p2p::PEER_ID)
            .executor(Box::new(|fut| {
                tokio::spawn(fut);
            }))
            .build();
    
    let mut stdin = BufReader::new(stdin()).lines();

    Swarm::listen_on(
        &mut swarm, 
        "/ip4/0.0.0.0/tcp/0".parse().expect("can get local socket")
    ).expect("swarm can be started");

    tokio::spawn(async move {
        time::sleep(Duration::from_secs(1)).await;
        info!("sending init event");
        init_sender.send(p2p::EventType::Init).expect("unable to send init event");
    });

    loop {
        let evt = {
            tokio::select! {
                line = stdin.next_line() => { 
                    Some(EventType::Input(line.expect("read line").expect("bleh")))
                },
                response = response_rcv.recv() => {
                    Some(EventType::LocalChainResponse(response.expect("aksdl")))
                },
                _init = init_rcv.recv() => {
                    Some(EventType::Init)
                },
                event = swarm.select_next_some() => {
                    info!("unhandled swarm event {:?}", event);
                    None
                }
            }
        };

        if let Some(event) = evt {
            match event {
                EventType::Init => {
                    swarm.behaviour_mut().app.genesis();
                    let peers = p2p::get_peers_list(&swarm);
                    
                    if peers.is_empty() == false {
                        let selected_peer_id = 
                            peers.iter().last().expect("kajsdlkfj").to_string();
                        
                        let chain_request = p2p::LocalChainRequest {
                            from_peer_id: selected_peer_id
                        };

                        let json = serde_json::to_string(&chain_request).expect("msaskjhkjg");
                        swarm
                            .behaviour_mut()
                            .floodsub.publish(p2p::CHAIN_TOPIC.clone(), json.as_bytes());
                    }
                },
                EventType::LocalChainResponse(resp) => {
                    let json = serde_json::to_string(&resp).expect("cant jsonify resp");
                    swarm
                        .behaviour_mut()
                        .floodsub.publish(p2p::CHAIN_TOPIC.clone(), json.as_bytes());

                },
                EventType::Input(line) => match line.as_str() {
                    "ls p" => {
                        info!("Connected Peers");
                        let peers_names = p2p::get_peers_list(&swarm);
                        peers_names.iter().for_each(|p| info!("{}", p));
                    },
                    cmd if cmd.starts_with("ls c") => {
                        info!("Local Blockchain");
                        let blocks = &swarm.behaviour().app.blocks;
                        let json = serde_json::to_string_pretty(blocks).expect("mskashg");
                        info!("{}", json);
                    },
                    cmd if cmd.starts_with("create b") => {
                        info!("Creating block");
                        p2p::handle_create_block(cmd, &mut swarm);
                    },
                    _ => error!("unrecognized command")
                }
            };
        }
    }

}
