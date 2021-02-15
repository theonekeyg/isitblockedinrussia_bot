use std::collections::HashMap;
use telegram_bot::{Api, UpdatesStream};
use telegram_bot::types::{ChatRef, UpdateKind, MessageKind, ToChatRef};
use telegram_bot::types::requests::{SendMessage};
use tokio_postgres::row::Row;
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

    fn construct_response(&self, rows: Vec<Row>) -> String {
        let mut s = String::with_capacity(rows.len() * 50);
        for row in rows.iter() {
            let ip:            String = row.get("ip");
            // let domain:        String = row.get("domain");
            // let url:           String = row.get("url");
            let decision_org:  String = row.get("decision_org");
            // let decision_num:  String = row.get("decision_num");
            let decision_date: String = row.get("decision_date");
            s += format!("ip {} is blocked on {} by {}\n",
                         ip, decision_date, decision_org
            ).as_str();
        }
        s.pop();
        s
    }

    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut stream = UpdatesStream::new(&self.api);
        let ipv4_regex = Regex::new(r"^[1-9][0-9]{0,2}(\.[1-9][0-9]{0,2}|\.0){3}$")?;
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
                            } else if ipv4_regex.is_match(data) || url_regex.is_match(data) {
                                println!("`{}` is ipv4 or url!", data);
                                let rows = self.db.get_blocked(data.to_string()).await?;
                                if rows.is_empty() {
                                    self.api.spawn(SendMessage::new(
                                            message.chat, format!("{} is not blocked yet", data)
                                        )
                                    );
                                } else {
                                    self.api.spawn(SendMessage::new(
                                            message.chat, self.construct_response(rows)
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

#[cfg(test)]
mod tests {
    use crate::{BlockedBot, BlockedDB};
    use std::collections::HashMap;
    use telegram_bot::types::requests::GetMe;

    #[tokio::test]
    async fn test_bot_getme() {
        dotenv::dotenv().expect("Unable to parse .env file");
        let args = std::env::vars().collect::<HashMap<String, String>>();
        let token = &args["TELEGRAM_TOKEN"];

        let db = BlockedDB::connect(
            format!("host={} user={} password={} dbname=isitblockedinrussia",
            args["DB_HOST"], args["DB_USER"], args["DB_PASSWORD"]).as_ref()
        ).await.expect("Error creating db instance");
        let bot = BlockedBot::new(token, HashMap::new(), db)
            .await.expect("Error creating a bot instance");
        let res = bot.api.send(GetMe).await;
        assert!(res.is_ok(), format!("Error reaching telegram bot server: {:?}", res));
    }
}
