use crate::rss::FeedItem;
use html_escape::{encode_quoted_attribute, encode_text};

pub const MAX_MESSAGE: usize = 4096;
pub const MAX_CAPTION: usize = 1024;

fn language_name(code: &str) -> Option<(&'static str, &'static str)> {
    match code {
        "CHS" => Some(("简", "简体")),
        "CHT" => Some(("繁", "繁体")),
        "ZH" => Some(("中", "中文")),
        "JP" | "JA" => Some(("日", "日语")),
        "EN" => Some(("英", "英语")),
        "DE" => Some(("德", "德语")),
        "ID" => Some(("印尼", "印尼语")),
        "IT" => Some(("意", "意大利语")),
        "MS" => Some(("马来", "马来语")),
        "TH" => Some(("泰", "泰语")),
        "VI" => Some(("越", "越南语")),
        "KR" | "KO" => Some(("韩", "韩语")),
        "ES" => Some(("西", "西班牙语")),
        "FR" => Some(("法", "法语")),
        "PT" => Some(("葡", "葡萄牙语")),
        "RU" => Some(("俄", "俄语")),
        _ => None,
    }
}

fn normalized_title(title: &str) -> String {
    title
        .replace('簡', "简")
        .replace('體', "体")
        .replace('內', "内")
}

fn title_language_label(title: &str) -> Option<String> {
    let title = normalized_title(title);
    for (tag, label) in [
        ("简繁日", "简繁日"),
        ("简日", "简日"),
        ("繁日", "繁日"),
        ("简繁", "简繁"),
        ("简体", "简体"),
        ("繁体", "繁体"),
        ("简中", "简体"),
        ("繁中", "繁体"),
    ] {
        if title.contains(tag) {
            return Some(label.to_string());
        }
    }

    let title = title.to_ascii_uppercase();
    let has_simplified = title.contains("CHS") || title.contains("SC_TC");
    let has_traditional = title.contains("CHT") || title.contains("SC_TC");
    match (has_simplified, has_traditional) {
        (true, true) => Some("简繁".to_string()),
        (true, false) => Some("简体".to_string()),
        (false, true) => Some("繁体".to_string()),
        (false, false) => None,
    }
}

fn language_label(item: &FeedItem) -> String {
    let mut languages: Vec<(String, String, String)> = Vec::new();
    for value in &item.languages {
        let code = match value.trim().to_ascii_uppercase().as_str() {
            "JA" => "JP".to_string(),
            "KO" => "KR".to_string(),
            code => code.to_string(),
        };
        if code.is_empty() || languages.iter().any(|(seen, _, _)| seen == &code) {
            continue;
        }
        let (short, full) = language_name(&code)
            .map(|(short, full)| (short.to_string(), full.to_string()))
            .unwrap_or_else(|| (code.clone(), code.clone()));
        languages.push((code, short, full));
    }

    if languages.len() > 3 {
        let has_chinese = languages
            .iter()
            .any(|(code, _, _)| matches!(code.as_str(), "CHS" | "CHT" | "ZH"));
        return if has_chinese {
            "中文多语".to_string()
        } else {
            "多语字幕".to_string()
        };
    }
    if languages.len() == 1 {
        return languages[0].2.clone();
    }
    if !languages.is_empty() {
        return languages
            .iter()
            .map(|(_, short, _)| short.as_str())
            .collect::<String>();
    }

    title_language_label(&item.title).unwrap_or_else(|| "字幕未知".to_string())
}

fn subtitle_label(item: &FeedItem) -> &'static str {
    match item.subtitle.trim().to_ascii_uppercase().as_str() {
        "EMBEDDED" | "HARDSUB" | "HARD_SUB" => "内嵌",
        "INTERNAL" | "SOFTSUB" | "SOFT_SUB" => "内封",
        "EXTERNAL" => "外挂",
        "NONE" | "RAW" => "生肉",
        _ => {
            let title = normalized_title(&item.title);
            if title.contains("内嵌") {
                "内嵌"
            } else if title.contains("内封") {
                "内封"
            } else if title.contains("外挂") {
                "外挂"
            } else if title.contains("生肉") || title.to_ascii_uppercase().contains("[RAW]") {
                "生肉"
            } else {
                "封装未知"
            }
        }
    }
}

fn release_label(item: &FeedItem) -> String {
    let language = language_label(item);
    let subtitle = subtitle_label(item);
    match (language.as_str(), subtitle) {
        ("字幕未知", "生肉") => "生肉".to_string(),
        ("字幕未知", _) | (_, "封装未知") => format!("{language} · {subtitle}"),
        _ => format!("{language}{subtitle}"),
    }
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
    let label = if item.link.is_empty() {
        encode_text(&label).to_string()
    } else {
        format!(
            "<a href=\"{}\">{}</a>",
            encode_quoted_attribute(&item.link),
            encode_text(&label)
        )
    };
    let magnet = magnet_link(item);
    if magnet.is_empty() {
        format!("🧲 <b>「{label}」磁力链接：</b>\n暂无")
    } else {
        format!(
            "🧲 <b>「{label}」磁力链接：</b>\n<code>{}</code>",
            encode_text(&magnet)
        )
    }
}

fn torrent_label(item: &FeedItem) -> String {
    let language = language_label(item);
    if language.starts_with("简繁") && subtitle_label(item) == "内封" {
        "内封".to_string()
    } else if language == "字幕未知" {
        "种子".to_string()
    } else {
        language
    }
}

fn torrent_link(item: &FeedItem) -> Option<String> {
    if item.torrent_url.is_empty() {
        None
    } else {
        Some(format!(
            "「<a href=\"{}\">{}</a>」",
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
    let mut parts = vec![format!("📽️ {}{episode}", encode_text(display_title))];
    let mut metadata = Vec::new();
    let group_tag = hashtag(&item.group_name);
    if !group_tag.is_empty() {
        metadata.push(format!("👤 #{}", encode_text(&group_tag)));
    }
    if !item.published.is_empty() {
        metadata.push(format!("📅 {}", encode_text(&item.published)));
    }
    if !metadata.is_empty() {
        parts.push(metadata.join("\n"));
    }
    parts.push(
        items
            .iter()
            .map(magnet_section)
            .collect::<Vec<_>>()
            .join("\n"),
    );
    let torrent_links = items.iter().filter_map(torrent_link).collect::<Vec<_>>();
    if !torrent_links.is_empty() {
        parts.push(format!("🌎 种子链接：{}", torrent_links.join("")));
    }
    parts.join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::{release_label, render, MAX_MESSAGE};
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
        assert!(text.contains("📽️ A &lt;title&gt; EP 7\n\n👤 #Group"));
        assert!(text.contains("简体内嵌"));
        assert!(text.contains("「<a href=\"https://example.com/?a=1&amp;b=2\">简体内嵌</a>」"));
        assert!(text.contains("<code>magnet:?xt=urn:btih:abcdef</code>"));
        assert!(text.contains("https://example.com/1.torrent"));
    }

    #[test]
    fn renders_multiple_variants_in_one_post() {
        let mut first = item();
        first.languages.push("JP".into());
        let mut second = item();
        second.languages = vec!["CHT".into(), "JP".into()];
        second.info_hash = "second".into();
        let text = render(&[first, second], MAX_MESSAGE);
        assert_eq!(text.matches("https://example.com/1.torrent").count(), 2);
        assert!(text.contains("简日内嵌"));
        assert!(text.contains("繁日内嵌"));
        assert!(text.contains("「<a href=\"https://example.com/1.torrent\">简日</a>」「"));
        assert!(text.contains("繁日</a>」"));
        assert!(text.chars().count() <= super::MAX_CAPTION);
    }

    #[test]
    fn combines_language_and_subtitle_labels() {
        let mut release = item();

        release.languages = vec!["CHS".into(), "JP".into()];
        release.subtitle = "EMBEDDED".into();
        assert_eq!(release_label(&release), "简日内嵌");

        release.languages = vec!["CHT".into(), "JP".into()];
        assert_eq!(release_label(&release), "繁日内嵌");

        release.languages = vec!["CHS".into(), "CHT".into(), "JP".into()];
        release.subtitle = "INTERNAL".into();
        assert_eq!(release_label(&release), "简繁日内封");

        release.languages = vec!["CHS".into()];
        release.subtitle = "EMBEDDED".into();
        assert_eq!(release_label(&release), "简体内嵌");

        release.languages = vec!["CHT".into()];
        release.subtitle = "INTERNAL".into();
        assert_eq!(release_label(&release), "繁体内封");

        release.languages = vec!["JP".into(), "EN".into()];
        release.subtitle = "EXTERNAL".into();
        assert_eq!(release_label(&release), "日英外挂");

        release.languages = vec!["JP".into()];
        release.subtitle = "NONE".into();
        assert_eq!(release_label(&release), "日语生肉");
    }

    #[test]
    fn supports_all_languages_currently_used_by_anibt() {
        let mut release = item();
        release.subtitle = "EMBEDDED".into();
        for (code, expected) in [
            ("DE", "德语内嵌"),
            ("EN", "英语内嵌"),
            ("ID", "印尼语内嵌"),
            ("IT", "意大利语内嵌"),
            ("JP", "日语内嵌"),
            ("MS", "马来语内嵌"),
            ("TH", "泰语内嵌"),
            ("VI", "越南语内嵌"),
        ] {
            release.languages = vec![code.into()];
            assert_eq!(release_label(&release), expected);
        }

        release.languages = vec!["DE".into(), "EN".into(), "DE".into()];
        release.subtitle = "INTERNAL".into();
        assert_eq!(release_label(&release), "德英内封");
    }

    #[test]
    fn abbreviates_more_than_three_unique_languages() {
        let mut release = item();

        release.languages = vec!["CHS".into(), "JP".into(), "EN".into()];
        release.subtitle = "EMBEDDED".into();
        assert_eq!(release_label(&release), "简日英内嵌");

        release.languages = vec!["EN".into(), "CHS".into(), "CHT".into(), "ID".into()];
        release.subtitle = "INTERNAL".into();
        assert_eq!(release_label(&release), "中文多语内封");

        release.languages = vec!["JP".into(), "EN".into(), "DE".into(), "FR".into()];
        assert_eq!(release_label(&release), "多语字幕内封");

        release.languages = vec!["JP".into(), "JA".into(), "EN".into(), "DE".into()];
        assert_eq!(release_label(&release), "日英德内封");
    }

    #[test]
    fn falls_back_to_title_tags_and_marks_missing_metadata() {
        let mut release = item();
        release.languages.clear();
        release.subtitle.clear();
        release.title = "[Group] Anime [繁日內嵌]".into();
        assert_eq!(release_label(&release), "繁日内嵌");

        release.title = "[Group] Anime [CHS][内封]".into();
        assert_eq!(release_label(&release), "简体内封");

        release.title = "[Group] Anime [RAW]".into();
        assert_eq!(release_label(&release), "生肉");

        release.title = "Release without metadata".into();
        assert_eq!(release_label(&release), "字幕未知 · 封装未知");

        release.languages = vec!["XX".into()];
        assert_eq!(release_label(&release), "XX · 封装未知");
    }
}
