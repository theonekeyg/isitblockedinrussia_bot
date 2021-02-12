use std::collections::HashMap;
use std::env;
use telegram_bot::types::{ChatRef};
use telegram_bot::types::requests::{SendMessage, SendVenue};
use telegram_bot::{Api};
use isitblockedinrussia_bot::{BlockedDB, BlockedBot};

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
    let db = BlockedDB::connect(
        format!("host={} user={} password={} dbname=isitblockedinrussia",
        args["DB_HOST"], args["DB_USER"], args["DB_PASSWORD"]).as_ref()
    ).await?;
    let operations = get_default_options();
    let bot = BlockedBot::new(token, operations, db).await?;
    bot.run().await?;
    Ok(())
}
