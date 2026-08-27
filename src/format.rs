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
        (true, false) => "简体",
        (false, true) => "繁体",
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

fn hashtag(value: &str) -> String {
    let mut tag = String::with_capacity(value.len());
    let mut previous_was_separator = false;
    for character in value.trim().chars() {
        if character.is_alphanumeric() || character == '_' {
            tag.push(character);
            previous_was_separator = false;
        } else if !tag.is_empty() && !previous_was_separator {
            tag.push('_');
            previous_was_separator = true;
        }
    }
    while tag.ends_with('_') {
        tag.pop();
    }
    tag
}

fn magnet_link(item: &FeedItem) -> String {
    if item.info_hash.is_empty() {
        item.magnet.clone()
    } else {
        format!("magnet:?xt=urn:btih:{}", item.info_hash)
    }
}

fn magnet_section(item: &FeedItem) -> String {
    let label = release_label(item);
    let magnet = magnet_link(item);
    if magnet.is_empty() {
        format!("🧲 <b>「{}」磁力链接：</b>\n暂无", encode_text(&label))
    } else {
        format!(
            "🧲 <b>「{}」磁力链接：</b>\n<code>{}</code>",
            encode_text(&label),
            encode_text(&magnet)
        )
    }
}

fn torrent_label(item: &FeedItem) -> &'static str {
    match (
        item.languages.iter().any(|value| value == "CHS"),
        item.languages.iter().any(|value| value == "CHT"),
        item.subtitle.as_str(),
    ) {
        (true, true, "INTERNAL") => "内封",
        (true, true, _) => "简繁",
        (true, false, _) => "简日",
        (false, true, _) => "繁日",
        (false, false, _) if item.languages.iter().any(|value| value == "JP") => "日语",
        (false, false, _) if item.languages.iter().any(|value| value == "EN") => "英语",
        _ => "种子",
    }
}

fn torrent_link(item: &FeedItem) -> Option<String> {
    if item.torrent_url.is_empty() {
        None
    } else {
        Some(format!(
            "<a href=\"{}\">{}</a>",
            encode_quoted_attribute(&item.torrent_url),
            torrent_label(item)
        ))
    }
}

pub fn render(items: &[FeedItem], _max_len: usize) -> String {
    let Some(item) = items.last() else {
        return String::new();
    };
    let display_title = if !item.anime_title.is_empty() {
        &item.anime_title
    } else if !item.anime_title_english.is_empty() {
        &item.anime_title_english
    } else {
        &item.title
    };
    let episode = if item.episode.is_empty() {
        String::new()
    } else {
        format!(" EP {}", encode_text(&item.episode))
    };
    let mut parts = vec![format!("<b>{}</b>{episode}", encode_text(display_title))];
    let anime_tag = hashtag(display_title);
    if !anime_tag.is_empty() {
        let resolution = if item.resolution.is_empty() {
            String::new()
        } else {
            format!(" · {}", encode_text(&item.resolution))
        };
        parts.push(format!(
            "📺 <b>番组：</b> #{}{resolution}",
            encode_text(&anime_tag)
        ));
    }
    let group_tag = hashtag(&item.group_name);
    if !group_tag.is_empty() {
        parts.push(format!("👤 <b>发布组：</b> #{}", encode_text(&group_tag)));
    }
    if !item.published.is_empty() {
        parts.push(format!(
            "📅 <b>发布时间：</b> {}",
            encode_text(&item.published)
        ));
    }
    parts.push(
        items
            .iter()
            .map(magnet_section)
            .collect::<Vec<_>>()
            .join("\n\n"),
    );
    let torrent_links = items.iter().filter_map(torrent_link).collect::<Vec<_>>();
    if !torrent_links.is_empty() {
        parts.push(format!("🌎 <b>种子链接</b>\n{}", torrent_links.join("   ")));
    }
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
        assert!(text.contains("#A_title"));
        assert!(text.contains("简体内嵌"));
        assert!(text.contains("<code>magnet:?xt=urn:btih:abcdef</code>"));
        assert!(text.contains("https://example.com/1.torrent"));
    }

    #[test]
    fn renders_multiple_variants_in_one_post() {
        let first = item();
        let mut second = item();
        second.languages = vec!["CHT".into(), "JP".into()];
        second.info_hash = "second".into();
        let text = render(&[first, second], MAX_MESSAGE);
        assert_eq!(text.matches("https://example.com/1.torrent").count(), 2);
        assert!(text.contains("简体内嵌"));
        assert!(text.contains("繁体内嵌"));
        assert!(text.contains("简日</a>"));
        assert!(text.contains("繁日</a>"));
        assert!(text.chars().count() <= super::MAX_CAPTION);
    }
}
