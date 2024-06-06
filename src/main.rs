use std::io::{StdoutLock, Write};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
struct Message {
    src: String,
    #[serde(rename = "dest")]
    dst: String,
    body: Body,
}

#[derive(Deserialize, Serialize)]
struct Body {
    #[serde(rename = "msg_id")]
    id: Option<usize>,
    in_reply_to: Option<usize>,
    #[serde(flatten)]
    payload: Payload,
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
enum Payload {
    Echo {
        echo: String,
    },
    EchoOk {
        echo: String,
    },
    Init {
        node_id: String,
        node_ids: Vec<String>,
    },
    InitOk,
}

struct EchoNode {
    id: usize,
}

impl EchoNode {
    pub fn handle(&mut self, input: Message, output: &mut StdoutLock) -> anyhow::Result<()> {
        match input.body.payload {
            Payload::Init { node_id, node_ids } => {
                let reply = Message {
                    src: input.dst,
                    dst: input.src,
                    body: Body {
                        in_reply_to: input.body.id,
                        id: Some(self.id),
                        payload: Payload::InitOk,
                    },
                };
                serde_json::to_writer(&mut *output, &reply)
                    .context("serialize response to init")?;
                output.write_all(b"\n").context("failed to write")?;
                self.id += 1;
            }
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
            Payload::InitOk { .. } => bail!("init OK"),
        }

        Ok(())
    }
}

fn main() -> Result<()> {
    let stdin = std::io::stdin().lock();
    let mut stdout = std::io::stdout().lock();
    //let mut output = serde_json::Serializer::new(stdout);
    let mut node = EchoNode { id: 0 };
    let inputs = serde_json::Deserializer::from_reader(stdin).into_iter::<Message>();

    for input in inputs {
        let input = input.context("maelstorm input could not be deserialized from STDIN")?;
        node.handle(input, &mut stdout)
            .context("node handle function failed")?;
    }
    Ok(())
}
