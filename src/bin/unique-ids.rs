use ::rtrom::*;
use std::io::{StdoutLock, Write};

use anyhow::{bail, Context, Result};
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
    Init {
        node_id: String,
        node_ids: Vec<String>,
    },
    InitOk,
}

struct UniqueNode {
    pub msg_id: usize,
}

impl Node<Payload> for UniqueNode {
    fn handle(&mut self, input: Message<Payload>, output: &mut StdoutLock) -> anyhow::Result<()> {
        match input.body.payload {
            Payload::Init { node_id, node_ids } => {
                let reply = Message {
                    src: input.dst,
                    dst: input.src,
                    body: Body {
                        in_reply_to: input.body.id,
                        id: Some(self.msg_id),
                        payload: Payload::InitOk,
                    },
                };
                serde_json::to_writer(&mut *output, &reply)
                    .context("serialize response to init")?;
                output.write_all(b"\n").context("failed to write")?;
                self.msg_id += 1;
            }
            Payload::Generate { .. } => {
                let guid = ulid::Ulid::new().to_string();
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
            Payload::InitOk { .. } => bail!("init OK"),
        }
        Ok(())
    }
}

pub fn main() -> Result<()> {
    return main_loop(UniqueNode { msg_id: 0 });
}
