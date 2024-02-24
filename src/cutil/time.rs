use chrono::{DateTime, Local};

pub fn now() -> DateTime<Local> {
  Local::now()
}

pub fn now_string() -> String {
  Local::now().to_rfc3339()
}

pub fn now_millis() -> i64 {
  return Local::now().timestamp_millis();
}

pub fn now_secs() -> i64 {
  return Local::now().timestamp();
}
