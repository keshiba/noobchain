
use std::collections::HashSet;

use libp2p::{ 
    identity, 
    PeerId, 
    NetworkBehaviour,
    swarm::{ NetworkBehaviourEventProcess },
    mdns::{ Mdns, MdnsEvent },
    floodsub::{ Floodsub, Topic, FloodsubEvent }, autonat::Event, Swarm,
};
use super::block::Block;
use serde::{ Serialize, Deserialize };
use identity::Keypair;
//use std::lazy::Lazy;
use once_cell::sync::Lazy;
use tokio::sync::mpsc;
use crate::App;

pub static KEYS: Lazy<Keypair> = Lazy::new(Keypair::generate_ed25519);
pub static PEER_ID: Lazy<PeerId> = Lazy::new(|| PeerId::from(KEYS.public()));
pub static CHAIN_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("chains"));
pub static BLOCK_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("block"));

#[derive(Debug, Serialize, Deserialize)]
pub struct ChainResponse {
    pub blocks: Vec<Block>,
    pub receiver: String
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LocalChainRequest {
    pub from_peer_id: String
}

#[derive(Debug)]
pub enum EventType {
    LocalChainResponse(ChainResponse),
    Input(String),
    Init
}

#[derive(NetworkBehaviour)]
#[behaviour(event_process = true)]
pub struct AppBehaviour {
    pub floodsub: Floodsub,
    pub mdns: Mdns,
    #[behaviour(ignore)]
    pub response_sender: mpsc::UnboundedSender<ChainResponse>,
    #[behaviour(ignore)]
    pub init_sender: mpsc::UnboundedSender<EventType>,
    #[behaviour(ignore)]
    pub app: App
}

impl AppBehaviour {
    pub async fn new(
        app: App,
        response_sender: mpsc::UnboundedSender<ChainResponse>,
        init_sender: mpsc::UnboundedSender<EventType>
    ) -> Self {
        let mut app_behaviour = Self {
            app: app,
            floodsub: Floodsub::new(*PEER_ID),
            mdns: Mdns::new(Default::default())
                    .await
                    .expect("can create mdns"),
            response_sender: response_sender,
            init_sender: init_sender
        };

        app_behaviour.floodsub.subscribe(CHAIN_TOPIC.clone());
        app_behaviour.floodsub.subscribe(BLOCK_TOPIC.clone());

        return app_behaviour;
    }
}

impl NetworkBehaviourEventProcess<MdnsEvent> for AppBehaviour {
    
    fn inject_event(&mut self, event: MdnsEvent) {
        match event {
            MdnsEvent::Discovered(discovered_list) => {
                for (peer, _addr) in discovered_list {
                    self.floodsub.add_node_to_partial_view(peer);
                }
            },
            MdnsEvent::Expired(expired_list) => {
                for (peer, _addr) in expired_list {
                    if !self.mdns.has_node(&peer) {
                        self.floodsub.remove_node_from_partial_view(&peer);
                    }
                }
            }
        }
    }
}

impl NetworkBehaviourEventProcess<FloodsubEvent> for AppBehaviour {
    fn inject_event(&mut self, event: FloodsubEvent) {
        if let FloodsubEvent::Message(msg) = event {
            if let Ok(resp) = serde_json::from_slice::<ChainResponse>(&msg.data) {

                if resp.receiver == PEER_ID.to_string() {
                    info!("Response from {}:", msg.source);
                    resp.blocks.iter().for_each(|r| info!("{:?}", r));

                    self.app.blocks = self.app.choose_chain(self.app.blocks.clone(), resp.blocks);
                }
            } else if let Ok(resp) = serde_json::from_slice::<LocalChainRequest>(&msg.data) {

                info!("sending local chain to {}", msg.source.to_string());
                let peer_id = resp.from_peer_id;
                if PEER_ID.to_string() == peer_id {
                    if let Err(e) = self.response_sender.send(ChainResponse {
                        blocks: self.app.blocks.clone(),
                        receiver: msg.source.to_string()
                    }) {
                        error!("error sending response via channel {}", e);
                    }
                }
            } else if let Ok(block) = serde_json::from_slice::<Block>(&msg.data) {

                info!("received new block from {}", msg.source.to_string());
                self.app.try_add_block(block);
            }
        }
    }
}

pub fn get_peers_list(swarm: &Swarm<AppBehaviour>) -> Vec<String> {

    let nodes = swarm.behaviour().mdns.discovered_nodes();
    let mut unique_peers = HashSet::new();

    for peer in nodes {
        unique_peers.insert(peer);
    }

    let unique_peer_names = 
        unique_peers.iter().map(|p| p.to_string()).collect();

    return unique_peer_names;
}

pub fn handle_create_block(cmd: &str, swarm: &mut Swarm<AppBehaviour>) {

    if let Some(data) = cmd.strip_prefix("create b") {
        let behaviour = swarm.behaviour_mut();
        let last_block = behaviour.app.blocks.last().expect("asda");
        let new_block = Block::new(
            last_block.id + 1,
            last_block.hash.clone(),
            data.to_owned()
        );

        let block_json = serde_json::to_string(&new_block).expect("mskjahslkjahg");
        behaviour.app.blocks.push(new_block);
        info!("broadcasting new block");
        
        behaviour.floodsub.publish(BLOCK_TOPIC.clone(), block_json.as_bytes());
    }
}