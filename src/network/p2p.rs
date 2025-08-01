use libp2p::{
    gossipsub,
    identity,
    mdns,
    noise,
    swarm::NetworkBehaviour,
    tcp,
    PeerId, Swarm, SwarmBuilder,
    futures::StreamExt,
};
use std::collections::HashSet;
use tokio::sync::mpsc;
use tracing::{error, info};
use crate::blockchain::{block::Block, chain::Blockchain};
use crate::core::transaction::Transaction;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum P2pMessage {
    ChainRequest,
    ChainResponse(Blockchain),
    Block(Block),
    Transaction(Transaction),
}

#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "P2pEvent")]
pub struct P2pBehaviour {
    pub gossipsub: gossipsub::Behaviour,
    pub mdns: mdns::tokio::Behaviour,
}

#[derive(Debug)]
pub enum P2pEvent {
    Gossipsub(gossipsub::Event),
    Mdns(mdns::Event),
}

impl From<gossipsub::Event> for P2pEvent {
    fn from(event: gossipsub::Event) -> Self {
        P2pEvent::Gossipsub(event)
    }
}

impl From<mdns::Event> for P2pEvent {
    fn from(event: mdns::Event) -> Self {
        P2pEvent::Mdns(event)
    }
}

pub struct P2p {
    pub swarm: Swarm<P2pBehaviour>,
    pub topic: gossipsub::IdentTopic,
    pub message_receiver: mpsc::UnboundedReceiver<P2pMessage>,
    pub message_sender: mpsc::UnboundedSender<P2pMessage>,
    pub peers: HashSet<PeerId>,
}

impl P2p {
    pub async fn new(message_sender: mpsc::UnboundedSender<P2pMessage>, message_receiver: mpsc::UnboundedReceiver<P2pMessage>) -> Self {
        let id_keys = identity::Keypair::generate_ed25519();
        let peer_id = PeerId::from(id_keys.public());
        info!("Peer ID: {}", peer_id);

        let topic = gossipsub::IdentTopic::new("sierpchain-blocks");
        let topic_for_behaviour = topic.clone();

        let mut swarm = SwarmBuilder::with_existing_identity(id_keys.clone())
            .with_tokio()
            .with_tcp(
                tcp::Config::default().nodelay(true),
                noise::Config::new,
                libp2p::yamux::Config::default,
            )
            .unwrap()
            .with_behaviour(move |key| {
                let mut gossipsub = gossipsub::Behaviour::new(
                    gossipsub::MessageAuthenticity::Signed(key.clone()),
                    gossipsub::Config::default(),
                )
                .unwrap();
                gossipsub.subscribe(&topic_for_behaviour).unwrap();

                let mdns = mdns::tokio::Behaviour::new(mdns::Config::default(), peer_id).unwrap();
                P2pBehaviour { gossipsub, mdns }
            })
            .unwrap()
            .build();

        Swarm::listen_on(&mut swarm, "/ip4/0.0.0.0/tcp/0".parse().unwrap()).unwrap();

        Self { swarm, topic, message_receiver, message_sender, peers: HashSet::new() }
    }

    pub async fn run(mut self) {
        loop {
            tokio::select! {
                Some(message) = self.message_receiver.recv() => {
                    let json = serde_json::to_vec(&message).unwrap();
                    if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(self.topic.clone(), json) {
                        error!("Failed to publish message: {:?}", e);
                    }
                }
                event = self.swarm.select_next_some() => {
                    match event {
                        libp2p::swarm::SwarmEvent::Behaviour(P2pEvent::Mdns(mdns::Event::Discovered(list))) => {
                            for (peer, _) in list {
                                self.swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer);
                                self.peers.insert(peer);
                            }
                            self.message_sender.send(P2pMessage::ChainRequest).unwrap();
                        }
                        libp2p::swarm::SwarmEvent::Behaviour(P2pEvent::Mdns(mdns::Event::Expired(list))) => {
                            for (peer, _) in list {
                                if !self.swarm.behaviour().mdns.has_node(&peer) {
                                    self.swarm.behaviour_mut().gossipsub.remove_explicit_peer(&peer);
                                    self.peers.remove(&peer);
                                }
                            }
                        }
                        libp2p::swarm::SwarmEvent::Behaviour(P2pEvent::Gossipsub(gossipsub::Event::Message {
                            propagation_source: peer_id,
                            message_id: _id,
                            message,
                        })) => {
                            let msg: Result<P2pMessage, serde_json::Error> = serde_json::from_slice(&message.data);
                            if let Ok(msg) = msg {
                                info!("Received message from peer {:?}: {:#?}", peer_id, msg);
                                self.message_sender.send(msg).unwrap();
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}
