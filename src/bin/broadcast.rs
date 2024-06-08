use ::rtrom::*;
use core::panic;
use std::{
    collections::{HashMap, HashSet},
    io::StdoutLock,
    time::Duration,
    usize,
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
enum Payload {
    Broadcast {
        message: usize,
    },
    BroadcastOk,
    Read,
    ReadOk {
        messages: HashSet<usize>,
    },
    Topology {
        topology: HashMap<String, Vec<String>>,
    },
    TopologyOk,
    Gossip {
        seen: HashSet<usize>,
    },
}

enum InjectedPayload {
    Gossip,
}

struct BroadcastNode {
    msg_id: usize,
    node_id: String,
    messages: HashSet<usize>,
    //this is for keeping the values that we know, other nodes know
    known: HashMap<String, HashSet<usize>>,
    neighbour: Vec<String>,
}

impl Node<(), Payload, InjectedPayload> for BroadcastNode {
    fn from_init(
        _state: (),
        init: Init,
        tx: std::sync::mpsc::Sender<Event<Payload, InjectedPayload>>,
    ) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        std::thread::spawn(move || {
            // generate gossip events
            // TODO: handle EOF signal
            loop {
                std::thread::sleep(Duration::from_millis(300));
                if let Err(_) = tx.send(Event::Injected(InjectedPayload::Gossip)) {
                    break;
                };
            }
        });

        Ok(BroadcastNode {
            node_id: init.node_id,
            msg_id: 1,
            messages: HashSet::new(),
            known: init
                .node_ids
                .into_iter()
                .map(|nid| (nid, HashSet::new()))
                .collect(),
            neighbour: Vec::new(),
        })
    }

    fn handle(
        &mut self,
        input: Event<Payload, InjectedPayload>,
        output: &mut StdoutLock,
    ) -> anyhow::Result<()> {
        match input {
            Event::EOF => {}
            Event::Injected(payload) => match payload {
                InjectedPayload::Gossip => {
                    for n in &self.neighbour {
                        // can optimize this gossiping
                        let known_to_n = &self.known[n];
                        Message {
                            src: self.node_id.clone(),
                            dst: n.clone(),
                            body: Body {
                                id: None,
                                in_reply_to: None,
                                payload: Payload::Gossip {
                                    seen: self
                                        .messages
                                        .iter()
                                        .copied()
                                        .filter(|m| !known_to_n.contains(m))
                                        .collect(),
                                },
                            },
                        }
                        .send(&mut *output)
                        .with_context(|| format!("gossip to {}", n))?;
                    }
                }
            },
            Event::Message(input) => {
                let mut reply = input.into_reply(Some(&mut self.msg_id));
                match reply.body.payload {
                    Payload::Gossip { seen } => {
                        self.messages.extend(seen);
                    }
                    Payload::Broadcast { message } => {
                        reply.body.payload = Payload::BroadcastOk;
                        self.messages.insert(message);
                        reply.send(&mut *output).context("reply to broadcast")?;
                    }
                    Payload::Read => {
                        reply.body.payload = Payload::ReadOk {
                            messages: self.messages.clone(),
                        };
                        reply.send(&mut *output).context("reply to read")?;
                    }
                    Payload::Topology { mut topology } => {
                        self.neighbour = topology.remove(&self.node_id).unwrap_or_else(|| {
                            panic!("no topolgy given for node {:?}", self.node_id)
                        });
                        reply.body.payload = Payload::TopologyOk;
                        reply.send(&mut *output).context("reply to topology")?;
                    }
                    Payload::ReadOk { .. } => {}
                    Payload::TopologyOk => {}
                    Payload::BroadcastOk => {}
                }
            }
        }
        Ok(())
    }
}

pub fn main() -> Result<()> {
    main_loop::<_, BroadcastNode, _, _>(())
}
