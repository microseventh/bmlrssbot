mod batch;
mod config;
mod format;
mod rss;
mod state;
mod telegram;

use anyhow::Result;
use chrono::Utc;
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
                    let batches = batch::build(items, settings.group_window_seconds);
                    let first = batches.len().saturating_sub(settings.max_posts_per_feed);
                    for mut batch in batches.into_iter().skip(first) {
                        if !batch.ready(Utc::now().timestamp(), settings.group_window_seconds) {
                            continue;
                        }
                        // A late variant after the closed window becomes a new post instead of
                        // repeating releases already sent in the original batch.
                        batch
                            .items
                            .retain(|item| !state.published.contains(&item.uid));
                        if batch.items.is_empty() {
                            continue;
                        }
                        if let Some(item) = batch.items.last_mut() {
                            rss::fill_cover(&client, item).await;
                        }
                        let text = format::render(&batch.items, format::MAX_MESSAGE);
                        let representative = batch.items.last().expect("batch is non-empty");
                        let result = if settings.dry_run {
                            info!(title=%representative.title, variants=batch.items.len(), message=%text, image=?representative.image_url, "dry-run post");
                            Ok(())
                        } else {
                            telegram::publish(
                                &client,
                                &settings.bot_token,
                                &settings.chat_id,
                                representative,
                                &text,
                            )
                            .await
                        };
                        match result {
                            Ok(()) => {
                                state
                                    .published
                                    .extend(batch.items.into_iter().map(|item| item.uid));
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
