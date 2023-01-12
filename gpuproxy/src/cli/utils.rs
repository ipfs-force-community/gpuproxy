use chrono::{DateTime, Duration, Local, LocalResult, NaiveDateTime, TimeZone, Utc};

pub fn timestamp_to_string(tm: i64) -> String {
    match Local.timestamp_opt(tm, 0) {
        LocalResult::None => "".to_string(),
        LocalResult::Single(v) => v.to_string(),
        LocalResult::Ambiguous(v1, v2) => format!("{}, {}", v1, v2),
    }
}
