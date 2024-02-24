use time::macros::format_description;
use time::UtcOffset;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::{non_blocking, rolling};
use tracing_error::ErrorLayer;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::fmt::time::OffsetTime;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, layer::SubscriberExt, prelude::*};
use tracing_subscriber::{Layer, Registry};

static mut _WORKER_GUARD: Option<WorkerGuard> = None;

pub fn configure(level: LevelFilter, bucket_path: String, app_name: String) {
  let local_time = OffsetTime::new(
    UtcOffset::from_hms(8, 0, 0).unwrap_or(UtcOffset::UTC),
    format_description!("[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:3]"),
  );
  let console_layer = fmt::layer()
    .pretty()
    .with_timer(local_time.clone())
    .with_writer(std::io::stdout)
    .with_filter(level);

  let file_appender = rolling::daily(format!("{}/log/", bucket_path), format!("{}.log", app_name));
  let (non_blocking_appender, _guard) = non_blocking(file_appender);
  unsafe {
    _WORKER_GUARD = Some(_guard);
  }

  let file_layer = fmt::layer()
    .with_timer(local_time.clone())
    .with_ansi(false)
    .with_writer(non_blocking_appender)
    .with_filter(LevelFilter::INFO);

  Registry::default()
    .with(ErrorLayer::default())
    .with(console_layer)
    .with(file_layer)
    .init();
}
