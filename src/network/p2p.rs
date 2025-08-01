use libp2p::{
    gossipsub,
    identity,
    mdns,
    noise,
    swarm::NetworkBehaviour,
    tcp,
    PeerId, Swarm, SwarmBuilder,
    futures::StreamExt, Multiaddr,
};
use std::collections::HashSet;
use tokio::sync::mpsc;
use tracing::{error, info, warn};
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
    pub async fn new(
        message_sender: mpsc::UnboundedSender<P2pMessage>,
        message_receiver: mpsc::UnboundedReceiver<P2pMessage>,
        p2p_port: u16,
        initial_peers: Vec<Multiaddr>,
    ) -> Self {
        let id_keys = identity::Keypair::generate_ed25519();
        let peer_id = PeerId::from(id_keys.public());
        info!("Peer ID: {}", peer_id);

        let topic = gossipsub::IdentTopic::new("sierpchain-blocks");

        let mut swarm = SwarmBuilder::with_existing_identity(id_keys.clone())
            .with_tokio()
            .with_tcp(
                tcp::Config::default().nodelay(true),
                noise::Config::new,
                libp2p::yamux::Config::default,
            )
            .unwrap()
            .with_behaviour(|key| {
                let gossipsub = gossipsub::Behaviour::new(
                    gossipsub::MessageAuthenticity::Signed(key.clone()),
                    gossipsub::Config::default(),
                )
                .unwrap();
                let mdns = mdns::tokio::Behaviour::new(mdns::Config::default(), peer_id).unwrap();
                P2pBehaviour { gossipsub, mdns }
            })
            .unwrap()
            .build();

        swarm.behaviour_mut().gossipsub.subscribe(&topic).unwrap();

        let listen_addr = format!("/ip4/0.0.0.0/tcp/{}", p2p_port);
        let addr: Multiaddr = listen_addr.parse().expect("Failed to parse listen address");
        swarm.listen_on(addr.clone()).unwrap();
        info!("Listening on {}", addr);

        for peer in initial_peers {
            info!("Dialing peer at {}", peer);
            if let Err(e) = swarm.dial(peer) {
                warn!("Failed to dial peer: {}", e);
            }
        }

        Self {
            swarm,
            topic,
            message_receiver,
            message_sender,
            peers: HashSet::new(),
        }
    }

    pub async fn run(mut self) {
        loop {
            tokio::select! {
                Some(message) = self.message_receiver.recv() => {
                    if let Ok(json) = serde_json::to_vec(&message) {
                        if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(self.topic.clone(), json) {
                            error!("Failed to publish message: {:?}", e);
                        }
                    }
                }
                event = self.swarm.select_next_some() => {
                    match event {
                        libp2p::swarm::SwarmEvent::NewListenAddr { address, .. } => {
                            info!("Listening on {:?}", address);
                        }
                        libp2p::swarm::SwarmEvent::Behaviour(P2pEvent::Mdns(mdns::Event::Discovered(list))) => {
                            for (peer_id, _multiaddr) in list {
                                info!("mDNS discovered a new peer: {peer_id}");
                                self.swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                                self.peers.insert(peer_id);
                            }
                            if !self.peers.is_empty() {
                                self.message_sender.send(P2pMessage::ChainRequest).unwrap();
                            }
                        }
                        libp2p::swarm::SwarmEvent::Behaviour(P2pEvent::Mdns(mdns::Event::Expired(list))) => {
                            for (peer_id, _multiaddr) in list {
                                if !self.swarm.behaviour().mdns.has_node(&peer_id) {
                                    self.swarm.behaviour_mut().gossipsub.remove_explicit_peer(&peer_id);
                                    self.peers.remove(&peer_id);
                                }
                            }
                        }
                        libp2p::swarm::SwarmEvent::Behaviour(P2pEvent::Gossipsub(gossipsub::Event::Message {
                            propagation_source: peer_id,
                            message_id: _id,
                            message,
                        })) => {
                            if let Ok(msg) = serde_json::from_slice::<P2pMessage>(&message.data) {
                                info!("Received message from peer {:?}: {:#?}", peer_id, msg);
                                self.message_sender.send(msg).unwrap();
                            }
                        }
                        libp2p::swarm::SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                            info!("Connected to {peer_id}");
                            self.peers.insert(peer_id);
                            self.message_sender.send(P2pMessage::ChainRequest).unwrap();
                        }
                        libp2p::swarm::SwarmEvent::ConnectionClosed { peer_id, .. } => {
                            warn!("Disconnected from {peer_id}");
                            self.peers.remove(&peer_id);
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}
