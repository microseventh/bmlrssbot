use anyhow::{Context, Result};
use chrono::FixedOffset;
use feed_rs::model::Entry;
use feed_rs::parser;
use regex::Regex;
use roxmltree::Node;
use sha2::{Digest, Sha256};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct FeedItem {
    pub uid: String,
    pub title: String,
    pub anime_title: String,
    pub anime_title_english: String,
    pub bgm_id: String,
    pub episode: String,
    pub episode_key: String,
    pub group_name: String,
    pub group_slug: String,
    pub languages: Vec<String>,
    pub subtitle: String,
    pub link: String,
    pub torrent_url: String,
    pub info_hash: String,
    pub magnet: String,
    pub published: String,
    pub published_timestamp: Option<i64>,
    pub image_url: Option<String>,
}

fn description(entry: &Entry) -> String {
    entry
        .summary
        .as_ref()
        .map(|s| s.content.clone())
        .or_else(|| entry.content.as_ref().and_then(|c| c.body.clone()))
        .unwrap_or_default()
}

#[derive(Default)]
struct AnibtFields {
    anime_title: String,
    anime_title_english: String,
    bgm_id: String,
    episode: String,
    episode_key: String,
    group_name: String,
    group_slug: String,
    languages: Vec<String>,
    subtitle: String,
    torrent_url: String,
    info_hash: String,
    magnet: String,
}

fn child_text(node: Node<'_, '_>, name: &str) -> String {
    node.children()
        .find(|child| child.is_element() && child.tag_name().name() == name)
        .and_then(|child| child.text())
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn anibt_fields(content: &[u8]) -> HashMap<String, AnibtFields> {
    let Ok(xml) = std::str::from_utf8(content) else {
        return HashMap::new();
    };
    let Ok(document) = roxmltree::Document::parse(xml) else {
        return HashMap::new();
    };
    document
        .descendants()
        .filter(|node| node.is_element() && node.tag_name().name() == "item")
        .filter_map(|item| {
            let guid = child_text(item, "guid");
            if guid.is_empty() {
                return None;
            }
            let languages = item
                .children()
                .filter(|child| child.is_element() && child.tag_name().name() == "language")
                .filter_map(|child| child.text())
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .collect();
            let torrent = item
                .children()
                .find(|child| child.is_element() && child.tag_name().name() == "torrent");
            Some((
                guid,
                AnibtFields {
                    anime_title: child_text(item, "animeTitle"),
                    anime_title_english: child_text(item, "animeTitleEnglish"),
                    bgm_id: child_text(item, "bgmId"),
                    episode: child_text(item, "episode"),
                    episode_key: child_text(item, "episodeKey"),
                    group_name: child_text(item, "groupName"),
                    group_slug: child_text(item, "groupSlug"),
                    languages,
                    subtitle: child_text(item, "subtitle"),
                    torrent_url: child_text(item, "torrentUrl"),
                    info_hash: torrent
                        .map(|node| child_text(node, "infohash"))
                        .unwrap_or_default(),
                    magnet: torrent
                        .map(|node| child_text(node, "magneturi"))
                        .unwrap_or_default(),
                },
            ))
        })
        .collect()
}

fn first_image(html: &str) -> Option<String> {
    let regex = Regex::new(r#"(?i)<img[^>]+src=[\"']([^\"']+)[\"']"#).ok()?;
    regex
        .captures(html)
        .and_then(|captures| captures.get(1))
        .map(|value| html_escape::decode_html_entities(value.as_str()).to_string())
}

pub fn parse_feed(content: &[u8], _source_url: &str, limit: usize) -> Result<Vec<FeedItem>> {
    let mut extensions = anibt_fields(content);
    let feed = parser::parse(content).context("invalid RSS/Atom document")?;
    Ok(feed
        .entries
        .into_iter()
        .take(limit)
        .map(|entry| {
            let fields = extensions.remove(&entry.id).unwrap_or_default();
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
                .flat_map(|media| media.thumbnails.iter())
                .map(|thumbnail| thumbnail.image.uri.clone())
                .next();
            let summary = description(&entry);
            let image_url = image_url.or_else(|| first_image(&summary)).or_else(|| {
                entry
                    .media
                    .iter()
                    .flat_map(|media| media.content.iter())
                    .filter(|content| {
                        content
                            .content_type
                            .as_ref()
                            .is_some_and(|kind| kind.to_string().starts_with("image/"))
                    })
                    .find_map(|content| content.url.clone())
                    .map(|url| url.to_string())
            });
            let published_at = entry.published.or(entry.updated);
            FeedItem {
                uid,
                title,
                anime_title: fields.anime_title,
                anime_title_english: fields.anime_title_english,
                bgm_id: fields.bgm_id,
                episode: fields.episode,
                episode_key: fields.episode_key,
                group_name: fields.group_name,
                group_slug: fields.group_slug,
                languages: fields.languages,
                subtitle: fields.subtitle,
                link,
                torrent_url: fields.torrent_url,
                info_hash: fields.info_hash,
                magnet: fields.magnet,
                published: published_at
                    .map(|date| {
                        date.with_timezone(
                            &FixedOffset::east_opt(8 * 60 * 60).expect("valid offset"),
                        )
                        .format("%Y-%m-%d %H:%M:%S")
                        .to_string()
                    })
                    .unwrap_or_default(),
                published_timestamp: published_at.map(|date| date.timestamp()),
                image_url,
            }
        })
        .collect())
}

pub async fn fill_cover(client: &reqwest::Client, item: &mut FeedItem) {
    if item.image_url.is_some() || item.link.is_empty() {
        return;
    }
    let Ok(response) = client.get(&item.link).send().await else {
        return;
    };
    let Ok(html) = response.text().await else {
        return;
    };
    let Ok(regex) =
        Regex::new(r#"(?i)<meta[^>]+property=[\"']og:image[\"'][^>]+content=[\"']([^\"']+)[\"']"#)
    else {
        return;
    };
    item.image_url = regex
        .captures(&html)
        .and_then(|captures| captures.get(1))
        .map(|value| html_escape::decode_html_entities(value.as_str()).to_string());
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
        assert_eq!(items[0].title, "Hello");
    }

    #[test]
    fn parses_anibt_extensions() {
        let xml = r#"<?xml version="1.0"?><rss version="2.0" xmlns:anibt="https://anibt.net/xmlns/rss/1.0/"><channel><title>AniBT</title><item><guid>rel_1</guid><title>Release</title><link>https://anibt.net/release/rel_1</link><anibt:animeTitle>番名</anibt:animeTitle><anibt:animeTitleEnglish>Anime</anibt:animeTitleEnglish><anibt:episode>7</anibt:episode><anibt:groupName>Group</anibt:groupName><anibt:language>CHS</anibt:language><anibt:language>CHT</anibt:language><anibt:subtitle>INTERNAL</anibt:subtitle><anibt:resolution>1080p</anibt:resolution><anibt:torrentUrl>https://anibt.net/1.torrent</anibt:torrentUrl><torrent xmlns="https://anibt.moe/xmlns/0.1/"><infohash>abcdef</infohash><magneturi>magnet:?xt=urn:btih:abcdef</magneturi></torrent></item></channel></rss>"#;
        let items = parse_feed(xml.as_bytes(), "https://anibt.net/rss", 5).unwrap();
        assert_eq!(items[0].anime_title, "番名");
        assert_eq!(items[0].languages, ["CHS", "CHT"]);
        assert_eq!(items[0].info_hash, "abcdef");
    }
}
