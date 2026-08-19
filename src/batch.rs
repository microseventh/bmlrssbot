use crate::rss::FeedItem;

#[derive(Debug)]
pub struct PostBatch {
    key: String,
    earliest_timestamp: Option<i64>,
    latest_timestamp: Option<i64>,
    pub items: Vec<FeedItem>,
}

impl PostBatch {
    fn new(key: String, item: FeedItem) -> Self {
        Self {
            key,
            earliest_timestamp: item.published_timestamp,
            latest_timestamp: item.published_timestamp,
            items: vec![item],
        }
    }

    fn can_join(&self, key: &str, timestamp: Option<i64>, window_seconds: i64) -> bool {
        if self.key != key {
            return false;
        }
        match (self.earliest_timestamp, timestamp) {
            (Some(earliest), Some(current)) => current - earliest <= window_seconds,
            _ => false,
        }
    }

    fn push(&mut self, item: FeedItem) {
        self.latest_timestamp = item.published_timestamp.or(self.latest_timestamp);
        self.items.push(item);
    }

    pub fn ready(&self, now_timestamp: i64, window_seconds: u64) -> bool {
        let window_seconds = i64::try_from(window_seconds).unwrap_or(i64::MAX);
        self.earliest_timestamp.is_none_or(|earliest| {
            // Some RSS providers publish timestamps ahead of the consumer clock. Treat
            // those batches as ready instead of waiting until the source clock catches up.
            earliest > now_timestamp || earliest <= now_timestamp - window_seconds
        })
    }
}

fn group_key(item: &FeedItem) -> String {
    let group = if item.group_slug.is_empty() {
        &item.group_name
    } else {
        &item.group_slug
    };
    let anime = if item.bgm_id.is_empty() {
        &item.anime_title
    } else {
        &item.bgm_id
    };
    let episode = if item.episode_key.is_empty() {
        &item.episode
    } else {
        &item.episode_key
    };
    format!("{group}\u{1f}{anime}\u{1f}{episode}")
}

pub fn build(mut items: Vec<FeedItem>, window_seconds: u64) -> Vec<PostBatch> {
    items.sort_by_key(|item| item.published_timestamp.unwrap_or_default());
    let window_seconds = i64::try_from(window_seconds).unwrap_or(i64::MAX);
    let mut batches: Vec<PostBatch> = Vec::new();

    for item in items {
        let key = group_key(&item);
        if let Some(batch) = batches
            .iter_mut()
            .rev()
            .find(|batch| batch.can_join(&key, item.published_timestamp, window_seconds))
        {
            batch.push(item);
        } else {
            batches.push(PostBatch::new(key, item));
        }
    }
    batches.sort_by_key(|batch| batch.latest_timestamp.unwrap_or_default());
    batches
}

#[cfg(test)]
mod tests {
    use super::build;
    use crate::rss::FeedItem;

    fn item(uid: &str, timestamp: i64, language: &str) -> FeedItem {
        FeedItem {
            uid: uid.into(),
            title: "Release".into(),
            anime_title: "Anime".into(),
            anime_title_english: String::new(),
            bgm_id: "123".into(),
            episode: "6".into(),
            episode_key: "6".into(),
            group_name: "Group".into(),
            group_slug: "group".into(),
            languages: vec![language.into(), "JP".into()],
            subtitle: "EMBEDDED".into(),
            resolution: "1080p".into(),
            link: String::new(),
            torrent_url: String::new(),
            info_hash: uid.into(),
            magnet: String::new(),
            published: String::new(),
            published_timestamp: Some(timestamp),
            image_url: None,
        }
    }

    #[test]
    fn merges_language_variants_in_ten_minute_window() {
        let batches = build(
            vec![
                item("chs", 1_000, "CHS"),
                item("cht", 1_001, "CHT"),
                item("internal", 1_002, "CHS"),
            ],
            600,
        );
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].items.len(), 3);
    }

    #[test]
    fn splits_batches_spanning_more_than_ten_minutes() {
        let batches = build(
            vec![item("early", 1_000, "CHS"), item("late", 1_601, "CHT")],
            600,
        );
        assert_eq!(batches.len(), 2);
    }

    #[test]
    fn batch_is_ready_ten_minutes_after_first_release() {
        let mut batches = build(
            vec![item("first", 1_000, "CHS"), item("last", 1_599, "CHT")],
            600,
        );
        let batch = batches.pop().unwrap();
        assert!(!batch.ready(1_599, 600));
        assert!(batch.ready(1_600, 600));
    }

    #[test]
    fn future_source_timestamp_does_not_block_a_batch() {
        let mut batches = build(vec![item("future", 2_000, "CHS")], 600);
        let batch = batches.pop().unwrap();
        assert!(batch.ready(1_000, 600));
    }
}
