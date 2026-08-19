use crate::rss::FeedItem;
use html_escape::encode_text;

pub const MAX_MESSAGE: usize = 4096;
pub const MAX_CAPTION: usize = 1024;

fn strip_tags(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut in_tag = false;
    for c in input.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn render(item: &FeedItem, max_len: usize) -> String {
    let mut parts = vec![
        format!("<b>{}</b>", encode_text(&item.title)),
        format!("<i>{}</i>", encode_text(&item.feed_title)),
    ];
    if !item.published.is_empty() {
        parts.push(encode_text(&item.published).to_string());
    }
    let summary = strip_tags(&item.summary);
    if !summary.is_empty() {
        parts.push(encode_text(&summary).to_string());
    }
    if !item.link.is_empty() {
        parts.push(format!(
            "<a href=\"{}\">阅读原文</a>",
            encode_text(&item.link)
        ));
    }
    let mut text = parts.join("\n");
    if text.chars().count() > max_len {
        text = text
            .chars()
            .take(max_len.saturating_sub(3))
            .collect::<String>()
            + "...";
    }
    text
}

#[cfg(test)]
mod tests {
    use super::{render, MAX_MESSAGE};
    use crate::rss::FeedItem;

    fn item() -> FeedItem {
        FeedItem {
            uid: "1".into(),
            feed_title: "Feed".into(),
            title: "A <title>".into(),
            link: "https://example.com/?a=1&b=2".into(),
            summary: "<p>Hello</p>".into(),
            published: String::new(),
            image_url: None,
        }
    }

    #[test]
    fn escapes_html_and_link_attributes() {
        let text = render(&item(), MAX_MESSAGE);
        assert!(text.contains("A &lt;title&gt;"));
        assert!(text.contains("a=1&amp;b=2"));
        assert!(text.contains("Hello"));
    }

    #[test]
    fn respects_limit() {
        let mut value = item();
        value.summary = "x".repeat(10_000);
        assert_eq!(render(&value, 100).chars().count(), 100);
    }
}
