//! Tracing-based logging configured from the `logging` block of ccg.json.
//!
//! Renders each record as a tab-separated line: `ts \t LEVEL \t target \t msg`.
//! A size-rotating file handler honors `max_bytes` + `backup_count`; `to_stdout`
//! toggles a stdout layer; `quiet_targets` lower the level of noisy crates.

use std::fmt;

use rolling_file::{BasicRollingFileAppender, RollingConditionBasic};
use tracing::{Event, Subscriber};
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::{FmtContext, FormatEvent, FormatFields};
use tracing_subscriber::prelude::*;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::EnvFilter;

use crate::config::LoggingConfig;

const DEFAULT_MAX_BYTES: u64 = 10 * 1024 * 1024;
const DEFAULT_BACKUPS: usize = 7;

/// Attend-style one-line formatter: `date \t LEVEL \t target \t message`.
struct TabFormatter;

impl<S, N> FormatEvent<S, N> for TabFormatter
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        let meta = event.metadata();
        let ts = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        write!(writer, "{}\t{}\t{}\t", ts, meta.level(), meta.target())?;
        ctx.field_format().format_fields(writer.by_ref(), event)?;
        writeln!(writer)
    }
}

fn build_filter(cfg: &LoggingConfig) -> EnvFilter {
    let level = cfg.level.clone().unwrap_or_else(|| "INFO".into());
    let mut filter = EnvFilter::new(level);
    for target in &cfg.quiet_targets {
        if let Ok(directive) = target.parse() {
            filter = filter.add_directive(directive);
        }
    }
    filter
}

/// Initialize the global tracing subscriber. The returned guard must be held for
/// the lifetime of the program (it flushes the non-blocking file writer on drop).
pub fn init(cfg: &LoggingConfig) -> Option<tracing_appender::non_blocking::WorkerGuard> {
    let filter = build_filter(cfg);

    let stdout_layer = if cfg.to_stdout {
        Some(
            tracing_subscriber::fmt::layer()
                .event_format(TabFormatter)
                .with_ansi(false)
                .with_writer(std::io::stdout),
        )
    } else {
        None
    };

    let mut guard = None;
    let file_layer = match &cfg.path {
        Some(path) => {
            if let Some(parent) = std::path::Path::new(path).parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let condition = RollingConditionBasic::new()
                .max_size(cfg.max_bytes.unwrap_or(DEFAULT_MAX_BYTES));
            match BasicRollingFileAppender::new(
                path,
                condition,
                cfg.backup_count.unwrap_or(DEFAULT_BACKUPS),
            ) {
                Ok(appender) => {
                    let (nb, g) = tracing_appender::non_blocking(appender);
                    guard = Some(g);
                    Some(
                        tracing_subscriber::fmt::layer()
                            .event_format(TabFormatter)
                            .with_ansi(false)
                            .with_writer(nb),
                    )
                }
                Err(e) => {
                    eprintln!("warning: could not open log file {path}: {e}");
                    None
                }
            }
        }
        None => None,
    };

    tracing_subscriber::registry()
        .with(filter)
        .with(stdout_layer)
        .with(file_layer)
        .init();

    guard
}
