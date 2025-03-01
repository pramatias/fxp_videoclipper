use anyhow::{Context, Result};
use log::debug;
use std::process::Command as StdCommand;

/// Retrieves the duration of a media file in milliseconds.
///
/// This function executes an `ffprobe` command to extract the duration
/// of the media file and converts it to milliseconds.
///
/// # Parameters
/// - `file_path`: The path to the media file as a string.
///
/// # Returns
/// - `Result<u64>`: The duration of the media file in milliseconds, or an error if the operation fails.
///
/// # Notes
/// - The function relies on the `ffprobe` command-line tool.
/// - The duration is converted from seconds to milliseconds before being returned.
pub fn media_duration(file_path: &str) -> Result<u64> {
    debug!("Attempting to get media duration for file: {}", file_path);

    let child = StdCommand::new("ffprobe")
        .args(&[
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
            file_path,
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .with_context(|| format!("Failed to spawn ffprobe command for file: {}", file_path))?;

    let output = child
        .wait_with_output()
        .context("Failed to capture ffprobe output")?;

    debug!("ffprobe command output: {:?}", output.stdout);

    let duration_str =
        String::from_utf8(output.stdout).context("Failed to parse ffprobe output as UTF-8")?;

    let duration_in_seconds = duration_str.trim().parse::<f64>().with_context(|| {
        format!(
            "Failed to parse duration as f64 from string: '{}'",
            duration_str
        )
    })?;

    debug!("Duration in seconds: {}", duration_in_seconds);

    let duration_in_milliseconds = (duration_in_seconds * 1000.0).round() as u64;

    debug!("Duration in milliseconds: {}", duration_in_milliseconds);

    Ok(duration_in_milliseconds)
}
