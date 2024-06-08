use ::rtrom::*;
use core::panic;
use std::{
    io::{StdoutLock, Write},
    sync::mpsc::Sender,
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
enum Payload {
    Generate,
    GenerateOk {
        #[serde(rename = "id")]
        guid: String,
    },
}

struct UniqueNode {
    msg_id: usize,
    node_id: String,
}

impl Node<(), Payload, ()> for UniqueNode {
    fn from_init(_state: (), init: Init, _tx: Sender<Event<Payload>>) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(UniqueNode {
            node_id: init.node_id,
            msg_id: 1,
        })
    }
    fn handle(&mut self, input: Event<Payload>, output: &mut StdoutLock) -> anyhow::Result<()> {
        let Event::Message(input) = input else {
            panic!("got injected an event");
        };
        match input.body.payload {
            Payload::Generate { .. } => {
                let guid = format!("{}-{}", self.node_id, self.msg_id);
                let reply = Message {
                    src: input.dst,
                    dst: input.src,
                    body: Body {
                        in_reply_to: input.body.id,
                        id: input.body.id,
                        payload: Payload::GenerateOk { guid },
                    },
                };
                serde_json::to_writer(&mut *output, &reply)
                    .context("serialize response to generate")?;
                output.write_all(b"\n").context("failed to write")?;
                self.msg_id += 1;
            }
            Payload::GenerateOk { .. } => {}
        }
        Ok(())
    }
}

pub fn main() -> Result<()> {
    main_loop::<_, UniqueNode, _, _>(())
}
