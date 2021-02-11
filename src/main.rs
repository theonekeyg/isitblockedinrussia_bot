use std::collections::HashMap;
use std::env;
use telegram_bot::types::{ChatRef, MessageKind, UpdateKind, ToChatRef};
use telegram_bot::types::requests::{SendMessage, SendVenue};
use telegram_bot::{Api, UpdatesStream};
use futures::StreamExt;
use regex::Regex;
use isitblockedinrussia_bot::BlockedDB;

fn get_default_options<'a>()
    -> HashMap<&'a str, Box<dyn Fn(&Api, ChatRef)>> {
    let mut map: HashMap<&'a str, Box<dyn Fn(&Api, ChatRef)>>
        = HashMap::with_capacity(5);
    map.insert("/start", Box::new(|api: &Api, chat: ChatRef| {
        api.spawn(
            SendMessage::new(
                chat,
                concat!("I'm a bot that checks if provided resource ",
                        "is blocked in Russian Federaiton")
            )
        );
    }));
    map.insert("/help", Box::new(|api: &Api, chat: ChatRef| {
        api.spawn(
            SendMessage::new(
                chat,
                concat!("I'm a bot that checks if provided resource ",
                        "is blocked in Russian Federaiton")
            )
        );
    }));
    map.insert("/venue", Box::new(|api: &Api, chat: ChatRef| {
        api.spawn(SendVenue::new(chat, 32.21211, 54.12317,
                  "Meeting place", "W/e address where we meet"));
    }));
    return map;
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv()?;
    let args = env::vars().collect::<HashMap<String, String>>();
    let token = &args["TELEGRAM_TOKEN"];
    let api = Api::new(token);
    let db = BlockedDB::connect(
        format!("host={} user={} password={} dbname=isitblockedinrussia",
        args["DB_HOST"], args["DB_USER"], args["DB_PASSWORD"]).as_ref()
    ).await?;
    let mut stream = UpdatesStream::new(&api);
    let ipv4_regex = Regex::new(r"^[1-9][0-9]{0,2}(.[1-9][0-9]{0,2}|.0){3}$")?;
    let url_regex = Regex::new(r"^(https?://)?([a-zA-Z0-9]+.)[a-z]{2,}$")?;
    let operations = get_default_options();
    while let Some(update) = stream.next().await {
        let update = update?;
        println!("{:?}", update);
        match update.kind {
            UpdateKind::Message(message) => {
                match message.kind {
                    MessageKind::Text{data, ..} => {
                        let data = data.as_ref();
                        if let Some(func) = operations.get(data) {
                            func(&api, message.chat.to_chat_ref());
                        } else if url_regex.is_match(data) {
                            println!("{} is url!", data);
                            /* TODO: Get ip from DNS server */
                        } else if ipv4_regex.is_match(data) {
                            println!("{} is ipv4!", data);
                            let rows = db.get_blocked(data.to_string()).await?;
                            if rows.is_empty() {
                                api.spawn(SendMessage::new(message.chat, format!("ip {} is not blocked yet", data)));
                            } else {
                                api.spawn(SendMessage::new(message.chat, format!("ip {} is blocked", data)));
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
