use crate::rss::FeedItem;
use html_escape::{encode_quoted_attribute, encode_text};

pub const MAX_MESSAGE: usize = 4096;
pub const MAX_CAPTION: usize = 1024;

fn release_label(item: &FeedItem) -> String {
    let language = match (
        item.languages.iter().any(|value| value == "CHS"),
        item.languages.iter().any(|value| value == "CHT"),
    ) {
        (true, true) => "简繁日",
        (true, false) => "简日",
        (false, true) => "繁日",
        (false, false) if item.languages.iter().any(|value| value == "JP") => "日语",
        (false, false) if item.languages.iter().any(|value| value == "EN") => "英语",
        _ => "未知字幕",
    };
    let subtitle = match item.subtitle.as_str() {
        "EMBEDDED" => "内嵌",
        "INTERNAL" => "内封",
        "NONE" => "生肉",
        _ => "",
    };
    format!("{language}{subtitle}")
}

fn release_row(item: &FeedItem) -> String {
    let release_label = release_label(item);
    let label = encode_text(&release_label);
    let detail = if item.link.is_empty() {
        format!("【{label}】")
    } else {
        format!(
            "<a href=\"{}\">【{label}】</a>",
            encode_quoted_attribute(&item.link)
        )
    };
    let hash = if item.info_hash.is_empty() {
        encode_text(&item.magnet).to_string()
    } else {
        encode_text(&item.info_hash).to_string()
    };
    let torrent = if item.torrent_url.is_empty() {
        String::new()
    } else {
        format!(
            " · <a href=\"{}\">种子下载</a>",
            encode_quoted_attribute(&item.torrent_url)
        )
    };
    format!("{detail} <code>{hash}</code>{torrent}")
}

pub fn render(items: &[FeedItem], _max_len: usize) -> String {
    let Some(item) = items.last() else {
        return String::new();
    };
    let display_title = if item.anime_title.is_empty() {
        &item.title
    } else {
        &item.anime_title
    };
    let episode = if item.episode.is_empty() {
        String::new()
    } else {
        format!(" EP{}", encode_text(&item.episode))
    };
    let mut parts = vec![format!("<b>{}</b>{episode}", encode_text(display_title))];
    if !item.anime_title_english.is_empty() {
        parts.push(encode_text(&item.anime_title_english).to_string());
    }
    if !item.published.is_empty() {
        parts.push(format!(
            "📅 <b>发布时间：</b>{}",
            encode_text(&item.published)
        ));
    }
    let mut metadata = Vec::new();
    if !item.group_name.is_empty() {
        metadata.push(item.group_name.as_str());
    }
    if !item.resolution.is_empty() {
        metadata.push(item.resolution.as_str());
    }
    if !metadata.is_empty() {
        parts.push(format!("🏷️ {}", encode_text(&metadata.join(" · "))));
    }
    parts.push(items.iter().map(release_row).collect::<Vec<_>>().join("\n"));
    parts.join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::{render, MAX_MESSAGE};
    use crate::rss::FeedItem;

    fn item() -> FeedItem {
        FeedItem {
            uid: "1".into(),
            title: "A <title>".into(),
            anime_title: "A <title>".into(),
            anime_title_english: String::new(),
            bgm_id: "1".into(),
            episode: "7".into(),
            episode_key: "7".into(),
            group_name: "Group".into(),
            group_slug: "group".into(),
            languages: vec!["CHS".into()],
            subtitle: "EMBEDDED".into(),
            resolution: "1080p".into(),
            link: "https://example.com/?a=1&b=2".into(),
            torrent_url: "https://example.com/1.torrent".into(),
            info_hash: "abcdef".into(),
            magnet: "magnet:?xt=urn:btih:abcdef".into(),
            published: String::new(),
            published_timestamp: Some(1_000),
            image_url: None,
        }
    }

    #[test]
    fn escapes_html_and_link_attributes() {
        let text = render(&[item()], MAX_MESSAGE);
        assert!(text.contains("A &lt;title&gt;"));
        assert!(text.contains("a=1&amp;b=2"));
        assert!(text.contains("简日内嵌"));
        assert!(text.contains("<code>abcdef</code>"));
    }

    #[test]
    fn renders_multiple_variants_in_one_post() {
        let first = item();
        let mut second = item();
        second.languages = vec!["CHT".into(), "JP".into()];
        second.info_hash = "second".into();
        let text = render(&[first, second], MAX_MESSAGE);
        assert_eq!(text.matches("种子下载").count(), 2);
        assert!(text.contains("简日内嵌"));
        assert!(text.contains("繁日内嵌"));
    }
}
