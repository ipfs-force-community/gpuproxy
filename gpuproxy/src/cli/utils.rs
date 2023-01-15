use chrono::{DateTime, Duration, Local, LocalResult, NaiveDateTime, TimeZone, Utc};

pub fn timestamp_to_string(tm: i64) -> String {
    match Local.timestamp_opt(tm, 0) {
        LocalResult::None => "".to_string(),
        LocalResult::Single(v) => v.to_string(),
        LocalResult::Ambiguous(v1, v2) => format!("{}, {}", v1, v2),
    }
}

pub fn short_msg(msg: String, len: usize) -> String {
    if msg.len() > len {
        let mut pre_msg = msg[..len].to_string();
        pre_msg.push_str("...");
        pre_msg
    } else {
        msg
    }
}
