# Telegram bot that checks if url or ip is blocked in Russia

# Dependencies
- rustc, cargo
- Postgresql

# Usage
First of all, fill in .env file with your configurations:
- `TELEGRAM_TOKEN` - telegram bot token
- `DB_*` - fields about postgresql configuration

To compile the bot, use `./start.sh` script, as you would use `cargo` (e.g.
`./start.sh run`). The explanation for using separate build file, instead of
plain `cargo` is in Cargo.toml. On first start it should create
isitblockedinrussia db in your postgresql, and fill it with some downloaded
data.

To run bot with E2E-testing, set `TELEGRAM_API_URL` environment variable to your
telegram bot server.
