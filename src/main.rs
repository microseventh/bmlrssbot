mod config;
mod format;
mod rss;
mod state;
mod telegram;

use anyhow::Result;
use config::Settings;
use state::State;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    let settings = Settings::from_env()?;
    let client = reqwest::Client::builder()
        .user_agent("bmlrssbot/0.1")
        .build()?;
    loop {
        let mut state = State::load(&settings.state_file);
        let mut count = 0;
        for url in &settings.feed_urls {
            match rss::fetch_feed(&client, url, settings.max_items_per_feed).await {
                Ok(items) => {
                    for item in items.into_iter().rev() {
                        if state.published.contains(&item.uid) {
                            continue;
                        }
                        let text = format::render(&item, format::MAX_MESSAGE);
                        let result = if settings.dry_run {
                            info!(title=%item.title, "dry-run post");
                            Ok(())
                        } else {
                            telegram::publish(
                                &client,
                                &settings.bot_token,
                                &settings.chat_id,
                                &item,
                                &text,
                            )
                            .await
                        };
                        match result {
                            Ok(()) => {
                                state.published.insert(item.uid);
                                count += 1;
                            }
                            Err(err) => error!(feed=%url, error=%err, "publish failed"),
                        }
                    }
                }
                Err(err) => error!(feed=%url, error=%err, "feed fetch failed"),
            }
        }
        state.save(&settings.state_file)?;
        info!(published = count, "poll complete");
        if settings.once {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_secs(
            settings.poll_interval_seconds,
        ))
        .await;
    }
    Ok(())
}
