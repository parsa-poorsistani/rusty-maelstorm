use ::rtrom::*;
use core::panic;
use std::{
    io::{StdoutLock, Write},
    sync::mpsc::Sender,
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
enum Payload {
    Echo { echo: String },
    EchoOk { echo: String },
}

struct EchoNode {
    pub id: usize,
}

impl Node<(), Payload, ()> for EchoNode {
    fn from_init(_state: (), _init: Init, _tx: Sender<Event<Payload>>) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(EchoNode { id: 1 })
    }

    fn handle(&mut self, input: Event<Payload>, output: &mut StdoutLock) -> anyhow::Result<()> {
        let Event::Message(input) = input else {
            panic!("got injected an event");
        };

        match input.body.payload {
            Payload::Echo { echo } => {
                let reply = Message {
                    src: input.dst,
                    dst: input.src,
                    body: Body {
                        in_reply_to: input.body.id,
                        id: input.body.id,
                        payload: Payload::EchoOk { echo },
                    },
                };
                serde_json::to_writer(&mut *output, &reply)
                    .context("serialize response to init")?;
                output.write_all(b"\n").context("failed to write")?;
                self.id += 1;
            }
            Payload::EchoOk { .. } => {}
        }
        Ok(())
    }
}

pub fn main() -> Result<()> {
    main_loop::<_, EchoNode, _, _>(())
}
