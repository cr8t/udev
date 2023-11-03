#[allow(dead_code)]
pub fn init() {
    let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug"))
        .format_timestamp(Some(env_logger::fmt::TimestampPrecision::Millis))
        .try_init();
}
