use anyhow::{Context, Result};
use env_logger::Builder;
use log::{debug, warn, LevelFilter};
use rolling_file::{BasicRollingFileAppender, RollingConditionBasic};
use std::fs::{create_dir_all, read_dir, remove_file};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Initializes a logger with specified log level and configuration.
///
/// This function sets up a logging system that includes a rolling file appender
/// and console output. It creates a log directory, manages log file sizes, and sets
/// a global log level for the application.
///
/// # Parameters
/// - `log_level`: The level of logging to be displayed (e.g., debug, info, warn, error)
///
/// # Returns
/// - `Result<()>`: Indicates successful initialization of the logger
///
/// # Notes
/// - Creates a "frames_exporter_logs" directory in the user's document directory (or "logs" in the current directory if the document directory isn't accessible)
/// - Implements rolling file logging with a maximum of 2 log files
/// - Sets a maximum file size of 5MB before rolling over to a new file
/// - Logs are formatted with timestamp, log level, and message
/// - Creates the log directory if it doesn't exist
/// - Deletes older log files if the maximum number of files is exceeded
/// - Initializes the global logger with the specified log level
/// - Logs errors when writing to the log file fails
pub fn initialize_logger(log_level: LevelFilter) -> Result<()> {
    let max_log_files = 2;
    let log_dir = directories::UserDirs::new()
        .and_then(|dirs| dirs.document_dir().map(|d| d.join("frames_exporter_logs")))
        .unwrap_or_else(|| PathBuf::from("logs"));

    debug!("Initializing logger with log directory: {:?}", log_dir);

    create_dir_all(&log_dir).context("Failed to create log directory")?;

    let log_file_path = log_dir.join("app.log");
    let size_limit = 5 * 1024 * 1024; // 5 MB

    let rolling_condition = RollingConditionBasic::new().max_size(size_limit);
    let rolling_appender =
        BasicRollingFileAppender::new(log_file_path.clone(), rolling_condition, max_log_files)
            .context("Failed to create rolling file appender")?;

    debug!(
        "Rolling file appender created successfully with max size: {} bytes",
        size_limit
    );

    let rolling_appender = Mutex::new(rolling_appender);

    manage_log_files(&log_dir, max_log_files).context("Failed to manage log files")?;

    let mut builder = Builder::new();
    builder.filter(None, log_level);

    builder.format(move |buf, record| {
        use console::style;
        use std::io::Write;

        let ts = buf.timestamp();
        let level = record.level();
        let msg = record.args();

        // Determine the color based on the log level.
        let color = match record.level() {
            log::Level::Error => console::Color::Red,
            log::Level::Warn => console::Color::Yellow,
            log::Level::Info => console::Color::Green,
            log::Level::Debug => console::Color::Blue,
            log::Level::Trace => console::Color::Cyan,
        };

        // Style the log level.
        let styled_level = style(record.level()).fg(color);

        // Write the styled log message to the console.
        writeln!(buf, "[{:<5}] {} - {}", styled_level, ts, msg)?;

        // Also write a plain-text log entry to the rolling file.
        let log_entry = format!("{} - {} - {}\n", ts, level, msg);
        if let Ok(mut appender) = rolling_appender.lock() {
            if let Err(e) = appender.write(log_entry.as_bytes()) {
                warn!("Failed to write log entry to file: {:?}", e);
            }
        }

        Ok(())
    });

    builder.try_init().context("Failed to initialize logger")?;

    debug!(
        "Logger initialized successfully. Logs will be written to {:?}",
        log_file_path
    );

    Ok(())
}

/// Manages log files in a directory, ensuring the number of files does not exceed a specified limit.
///
/// This function handles log file management by reading the directory, collecting and filtering log files,
/// sorting them by modification time, and deleting the oldest files if they exceed the specified maximum.
///
/// # Parameters
/// - `log_dir`: The directory path containing the log files to manage.
/// - `max_log_files`: The maximum number of log files to keep.
///
/// # Returns
/// - `Result<()>`: Indicates success or failure of the log file management operation.
///
/// # Notes
/// - Only files ending with the ".log" extension are considered.
/// - The directory must be writable for file deletion.
/// - It is the caller's responsibility to ensure the provided directory exists.
fn manage_log_files(log_dir: &Path, max_log_files: usize) -> Result<()> {
    debug!("Managing log files in directory: {:?}", log_dir);

    // Read the log directory
    let entries = read_dir(log_dir).context("Failed to read log directory")?;
    debug!("Successfully read log directory: {:?}", log_dir);

    // Collect and filter log files
    let mut log_files: Vec<PathBuf> = entries
        .filter_map(|entry| {
            let path = entry.ok().map(|e| e.path());
            if let Some(ref p) = path {
                debug!("Found file: {:?}", p);
            }
            path
        })
        .filter(|path| {
            let is_log = path.is_file() && path.extension().map_or(false, |ext| ext == "log");
            if is_log {
                debug!("Identified as log file: {:?}", path);
            }
            is_log
        })
        .collect();

    debug!("Total log files found: {}", log_files.len());

    // Sort log files by modification time (oldest first)
    log_files.sort_by_key(|path| path.metadata().and_then(|m| m.modified()).ok());
    debug!("Log files sorted by modification time.");

    // Delete old log files if the number exceeds the limit
    while log_files.len() > max_log_files {
        if let Some(old_file) = log_files.get(0).cloned() {
            debug!("Attempting to delete old log file: {:?}", old_file);
            if let Err(e) = remove_file(&old_file) {
                warn!("Failed to delete old log file {:?}: {}", old_file, e);
            } else {
                debug!("Successfully deleted old log file: {:?}", old_file);
                log_files.remove(0);
                debug!("Remaining log files: {}", log_files.len());
            }
        }
    }

    debug!("Log file management completed successfully.");
    Ok(())
}
