use anyhow::{bail, Result};
use reqwest::Client;
use serde_json::Value;
use tokio::time::{sleep, Duration};

use crate::{format::MAX_CAPTION, rss::FeedItem};

pub async fn publish(
    client: &Client,
    token: &str,
    chat_id: &str,
    item: &FeedItem,
    text: &str,
) -> Result<()> {
    let method = if item.image_url.is_some() {
        "sendPhoto"
    } else {
        "sendMessage"
    };
    let mut form = vec![
        ("chat_id", chat_id.to_string()),
        ("parse_mode", "HTML".to_string()),
    ];
    if let Some(url) = &item.image_url {
        form.push(("photo", url.clone()));
        form.push(("caption", text.chars().take(MAX_CAPTION).collect()));
    } else {
        form.push(("text", text.to_string()));
    }
    for attempt in 0..3 {
        let response = client
            .post(format!("https://api.telegram.org/bot{token}/{method}"))
            .form(&form)
            .send()
            .await;
        match response {
            Ok(resp) if resp.status().is_success() => {
                let payload: Value = resp.json().await?;
                if payload["ok"].as_bool() == Some(true) {
                    return Ok(());
                }
                bail!("Telegram API error: {payload}");
            }
            Ok(resp) if attempt == 2 => bail!("Telegram HTTP error: {}", resp.status()),
            Ok(_) | Err(_) => sleep(Duration::from_secs(2_u64.pow(attempt))).await,
        }
    }
    bail!("Telegram publish failed")
}
