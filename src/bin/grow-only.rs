use ::rtrom::*;
use std::{
    collections::{HashMap, HashSet},
    io::StdoutLock,
    time::Duration,
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
enum Payload {
    Add { delta: usize },
    AddOk,
    Read,
    ReadOk { value: usize },
    Gossip { values: HashMap<String, usize> },
}

struct GrowOnlyNode {
    //maybe we could use msg_id instead
    latest_val: usize,
    msg_id: usize,
    node_id: String,
    //messages: Vec<(String, usize, usize)>,
    //this is for keeping the values that we know, other nodes know
    known: HashMap<String, usize>,
    neighbours: Vec<String>,
}

enum InjectedPayload {
    Gossip,
}

impl Node<(), Payload, InjectedPayload> for GrowOnlyNode {
    fn from_init(
        state: (),
        init: Init,
        tx: std::sync::mpsc::Sender<Event<Payload, InjectedPayload>>,
    ) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_millis(300));
            if let Err(_) = tx.send(Event::Injected(InjectedPayload::Gossip)) {
                break;
            }
        });
        // Ring Based Partitioning
        let nodes = init.node_ids.clone();
        let noden = nodes.len();
        let k = std::cmp::max(1, std::cmp::min(3, noden / 2));
        let index = nodes
            .clone()
            .into_iter()
            .position(|x| x == init.node_id)
            .unwrap();
        let neighbours: Vec<String> = (1..=k)
            .map(|i| nodes[(index + i) % noden].clone())
            .collect();

        eprintln!(
            "Node {} initialized with neighbors: {:?}",
            init.node_id, neighbours
        );
        Ok(GrowOnlyNode {
            msg_id: 1,
            latest_val: 0,
            node_id: init.node_id,
            neighbours,
            known: HashMap::new(),
        })
    }

    fn handle(
        &mut self,
        input: Event<Payload, InjectedPayload>,
        output: &mut StdoutLock,
    ) -> anyhow::Result<()> {
        match input {
            Event::Injected(payload) => match payload {
                InjectedPayload::Gossip => {
                    let mut values = self.known.clone();
                    values.insert(self.node_id.clone(), self.latest_val);
                    for n in &self.neighbours {
                        eprintln!(
                            "Node {} gossiping to {:} values: {:?}",
                            self.node_id, n, values
                        );
                        Message {
                            src: self.node_id.clone(),
                            dst: n.clone(),
                            body: Body {
                                id: None,
                                in_reply_to: None,
                                payload: Payload::Gossip {
                                    values: values.clone(),
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
                    Payload::Add { delta } => {
                        eprintln!("Node {} adding: {}", self.node_id, delta);
                        self.latest_val += delta;
                        reply.body.payload = Payload::AddOk;
                        reply.send(&mut *output).context("reply to add");
                    }
                    Payload::Gossip { values } => {
                        for (node, val) in values {
                            if self.node_id != node && self.known.get(&node).unwrap_or(&0) <= &val {
                                self.known.insert(node.clone(), val);
                            }
                        }
                        eprintln!(
                            "Node {} updated known values: {:?}",
                            self.node_id, self.known
                        );
                    }
                    Payload::AddOk {} => {}
                    Payload::Read {} => {
                        let s: usize =
                            self.known.values().copied().sum::<usize>() + self.latest_val;
                        eprintln!("Node {} read value: {}", self.node_id, s);
                        reply.body.payload = Payload::ReadOk { value: s };
                        reply.send(&mut *output).context("reply to read");
                    }
                    Payload::ReadOk { .. } => {}
                }
            }
            Event::EOF => {}
        }
        Ok(())
    }
}
pub fn main() -> Result<()> {
    main_loop::<_, GrowOnlyNode, _, _>(())
}
