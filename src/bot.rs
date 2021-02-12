use std::collections::HashMap;
use telegram_bot::{Api, UpdatesStream};
use telegram_bot::types::{ChatRef, UpdateKind, MessageKind, ToChatRef};
use telegram_bot::types::requests::{SendMessage};
use regex::Regex;
use futures::StreamExt;

use crate::BlockedDB;

pub struct BlockedBot<'a> {
    api: Api,
    operations: HashMap<&'a str, Box<dyn Fn(&Api, ChatRef)>>,
    db: BlockedDB
}

impl<'a> BlockedBot<'a> {
    pub async fn new(
        token: &'a str,
        operations: HashMap<&'a str, Box<dyn Fn(&Api, ChatRef)>>,
        db: BlockedDB
        ) -> Result<BlockedBot<'a>, Box<dyn std::error::Error>> {
        let api = Api::new(token);

        Ok(BlockedBot { api: api, operations: operations, db: db})
    }

    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut stream = UpdatesStream::new(&self.api);
        let ipv4_regex = Regex::new(r"^[1-9][0-9]{0,2}(.[1-9][0-9]{0,2}|.0){3}$")?;
        let url_regex = Regex::new(r"^(https?://)?([a-zA-Z0-9]+.)[a-z]{2,}$")?;

        while let Some(update) = stream.next().await {
            let update = update?;
            println!("{:?}", update);
            match update.kind {
                UpdateKind::Message(message) => {
                    match message.kind {
                        MessageKind::Text{data, ..} => {
                            let data = data.as_ref();
                            if let Some(func) = self.operations.get(data) {
                                func(&self.api, message.chat.to_chat_ref());
                            } else if url_regex.is_match(data) {
                                println!("{} is url!", data);
                                /* TODO: Get ip from DNS server */
                            } else if ipv4_regex.is_match(data) {
                                println!("{} is ipv4!", data);
                                let rows = self.db.get_blocked(data.to_string()).await?;
                                if rows.is_empty() {
                                    self.api.spawn(SendMessage::new(
                                            message.chat, format!("ip {} is not blocked yet", data)
                                        )
                                    );
                                } else {
                                    self.api.spawn(SendMessage::new(
                                            message.chat, format!("ip {} is blocked", data)
                                        )
                                    );
                                }
                            } else {
                                /* TODO: Handle invalid user input */
                            }
                        }
                        unknown => {
                            eprintln!("Unsupported message kind received: {:?}", unknown);
                        }
                    }
                }
                unknown => {
                    eprintln!("Unsupported operation received: {:?}", unknown);
                }
            }
        }
        Ok(())
    }
}
