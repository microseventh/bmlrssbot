use anyhow::{bail, Context, Result};
use std::{env, path::PathBuf};

#[derive(Clone, Debug)]
pub struct Settings {
    pub bot_token: String,
    pub chat_id: String,
    pub feed_urls: Vec<String>,
    pub poll_interval_seconds: u64,
    pub state_file: PathBuf,
    pub max_items_per_feed: usize,
    pub dry_run: bool,
    pub once: bool,
}

fn env_string(name: &str, default: &str) -> String {
    env::var(name)
        .unwrap_or_else(|_| default.to_string())
        .trim()
        .to_string()
}

impl Settings {
    pub fn from_env() -> Result<Self> {
        let dry_run = matches!(
            env_string("DRY_RUN", "false").as_str(),
            "1" | "true" | "yes"
        );
        let bot_token = env_string("TELEGRAM_BOT_TOKEN", "");
        let chat_id = env_string("TELEGRAM_CHAT_ID", "");
        if !dry_run && bot_token.is_empty() {
            bail!("TELEGRAM_BOT_TOKEN is required unless DRY_RUN=true");
        }
        if !dry_run && chat_id.is_empty() {
            bail!("TELEGRAM_CHAT_ID is required unless DRY_RUN=true");
        }
        let feed_urls: Vec<String> = env_string("RSS_FEEDS", "")
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect();
        if feed_urls.is_empty() {
            bail!("RSS_FEEDS must contain at least one URL");
        }
        Ok(Self {
            bot_token,
            chat_id,
            feed_urls,
            poll_interval_seconds: env_string("POLL_INTERVAL_SECONDS", "300")
                .parse()
                .context("invalid POLL_INTERVAL_SECONDS")?,
            state_file: PathBuf::from(env_string("STATE_FILE", "/data/state.json")),
            max_items_per_feed: env_string("MAX_ITEMS_PER_FEED", "20")
                .parse()
                .context("invalid MAX_ITEMS_PER_FEED")?,
            dry_run,
            once: env::args().any(|arg| arg == "--once"),
        })
    }
}
