use anyhow::{Context, Result};
use feed_rs::model::Entry;
use feed_rs::parser;
use sha2::{Digest, Sha256};

#[derive(Clone, Debug)]
pub struct FeedItem {
    pub uid: String,
    pub feed_title: String,
    pub title: String,
    pub link: String,
    pub summary: String,
    pub published: String,
    pub image_url: Option<String>,
}

fn text(entry: &Entry) -> String {
    entry
        .summary
        .as_ref()
        .map(|s| s.content.clone())
        .or_else(|| entry.content.as_ref().and_then(|c| c.body.clone()))
        .unwrap_or_default()
}

pub fn parse_feed(content: &[u8], source_url: &str, limit: usize) -> Result<Vec<FeedItem>> {
    let feed = parser::parse(content).context("invalid RSS/Atom document")?;
    let feed_title = feed
        .title
        .map(|t| t.content)
        .unwrap_or_else(|| source_url.to_string());
    Ok(feed
        .entries
        .into_iter()
        .take(limit)
        .map(|entry| {
            let title = entry
                .title
                .as_ref()
                .map(|t| t.content.clone())
                .unwrap_or_else(|| "(untitled)".to_string());
            let key = if entry.id.is_empty() {
                entry
                    .links
                    .first()
                    .map(|l| l.href.clone())
                    .unwrap_or_else(|| title.clone())
            } else {
                entry.id.clone()
            };
            let mut hasher = Sha256::new();
            hasher.update(key.as_bytes());
            let uid = format!("{:x}", hasher.finalize());
            let link = entry
                .links
                .iter()
                .find(|l| l.rel.as_deref() == Some("alternate"))
                .or_else(|| entry.links.first())
                .map(|l| l.href.clone())
                .unwrap_or_default();
            let image_url = entry
                .media
                .iter()
                .flat_map(|m| m.content.iter())
                .find_map(|m| m.url.clone());
            FeedItem {
                uid,
                feed_title: feed_title.clone(),
                title,
                link,
                summary: text(&entry),
                published: entry
                    .published
                    .or(entry.updated)
                    .map(|d| d.to_rfc3339())
                    .unwrap_or_default(),
                image_url: image_url.map(|url| url.to_string()),
            }
        })
        .collect())
}

pub async fn fetch_feed(
    client: &reqwest::Client,
    url: &str,
    limit: usize,
) -> Result<Vec<FeedItem>> {
    let bytes = client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    parse_feed(&bytes, url, limit)
}

#[cfg(test)]
mod tests {
    use super::parse_feed;

    #[test]
    fn parses_rss() {
        let xml = br#"<?xml version="1.0"?><rss version="2.0"><channel><title>News</title><item><guid>x-1</guid><title>Hello</title><link>https://example.com/1</link><description><![CDATA[<p>Body</p>]]></description></item></channel></rss>"#;
        let items = parse_feed(xml, "https://example.com/feed", 20).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].feed_title, "News");
        assert_eq!(items[0].title, "Hello");
    }
}
