use log::LevelFilter;

pub fn init() -> crate::errors::Result<()> {
    fern::Dispatch::new()
        .format(|out, message, record| {

            let source = format!("{}:{}", record.target(), record.line().unwrap_or_default());
            let gap = if source.len() < 35 {
                " ".repeat(35 - source.len())
            } else {
                " ".to_string()
            };

            out.finish(format_args!(
                "[{} | {:6}| {}]{} {}",
                chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                source,
                gap,
                message
            ))
        })
        .level(LevelFilter::Info)
        .chain(std::io::stdout())
        .apply()
        .map_err(|e| crate::errors::Error::LoggerSetupError(e))?;
    Ok(())
}