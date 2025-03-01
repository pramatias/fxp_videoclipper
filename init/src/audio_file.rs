use crate::config::Config;

use anyhow::anyhow;
use anyhow::{Context, Result};
use log::debug;
use std::env;
use std::fs;
use std::path::PathBuf;

/// Retrieves the audio file (an MP3) from various sources.
///
/// This function determines the audio file based on the provided input,
/// checking in the following order: CLI argument, environment variable, and
/// configuration file. If no valid MP3 file is found, it returns `None`.
///
/// # Parameters
/// - `cli_audio`: Optional audio file or directory provided via the command line.
/// - `config`: Configuration containing fallback audio file information (e.g. `mp3_file`).
///
/// # Returns
/// - `Option<PathBuf>`: The resolved MP3 file path, or `None` if not found or invalid.
///
/// # Notes
/// - Prioritizes CLI argument over environment variable and configuration file.
/// - Uses `find_mp3_file` to validate and locate an MP3 file.
#[allow(dead_code)]
pub fn get_audio_file(cli_audio: Option<String>, config: &Config) -> Option<PathBuf> {
    debug!("Determining audio file...");

    // Capture the audio file from configuration (if available and not empty)
    let config_audio_file = config.mp3_path.clone().filter(|s| !s.trim().is_empty());

    // Determine the audio file source.
    // Note: We use the same environment variable name ("FXP_VIDEOCLIPPER_AUDIO") as before.
    let audio_file_source = if let Some(cli_file) = cli_audio {
        debug!("Using audio file provided via CLI argument: {}", cli_file);
        Some(cli_file)
    } else if let Ok(env_file) = env::var("FXP_VIDEOCLIPPER_AUDIO") {
        if !env_file.trim().is_empty() {
            debug!(
                "Using audio file from environment variable FXP_VIDEOCLIPPER_AUDIO: {}",
                env_file
            );
            Some(env_file)
        } else {
            None
        }
    } else if let Some(cfg_file) = config_audio_file {
        debug!("Using audio file from configuration file: {}", cfg_file);
        Some(cfg_file)
    } else {
        debug!("No audio file provided.");
        None
    };

    // If we have a candidate, verify it is a valid MP3 file (or that it contains one).
    if let Some(ref path_or_dir) = audio_file_source {
        match find_mp3_file(path_or_dir) {
            Ok(mp3_path) => {
                debug!("Resolved audio file: {:?}", mp3_path);
                Some(PathBuf::from(mp3_path))
            }
            Err(e) => {
                debug!("Failed to locate MP3 file: {}", e);
                None
            }
        }
    } else {
        None
    }
}

/// Searches for an MP3 file at the given path or within the given directory.
///
/// If the provided `path_or_dir` points to a file, the function checks that it has an MP3
/// extension. If it points to a directory, the function searches for the first file with an
/// MP3 extension. If no valid MP3 file is found, an error is returned.
///
/// # Parameters
/// - `path_or_dir`: The file or directory path to check.
///
/// # Returns
/// - `Result<String>`: The path to the found MP3 file as a string, or an error if no MP3 file is
///   found.
#[allow(dead_code)]
fn find_mp3_file(path_or_dir: &str) -> Result<String> {
    let path = std::path::Path::new(&path_or_dir);

    if path.is_file() {
        // If the path is a file, check if it is an MP3 file.
        if path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|s| s.eq_ignore_ascii_case("mp3"))
            == Some(true)
        {
            let mp3_path = path.to_string_lossy().to_string();
            debug!("Input is a valid MP3 file: {}", mp3_path);
            Ok(mp3_path)
        } else {
            debug!("Input path is a file but not an MP3: {}", path_or_dir);
            Err(anyhow!(
                "The specified path is a file but not an MP3: {}",
                path_or_dir
            ))
        }
    } else if path.is_dir() {
        // If the path is a directory, search for an MP3 file.
        debug!("Searching for MP3 file in directory: {}", path_or_dir);
        let mp3_file = fs::read_dir(path)
            .with_context(|| format!("Failed to read directory: {}", path_or_dir))?
            .filter_map(Result::ok)
            .find(|entry| {
                entry
                    .path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|s| s.eq_ignore_ascii_case("mp3"))
                    .unwrap_or(false)
            });

        if let Some(entry) = mp3_file {
            let mp3_path = entry.path().to_string_lossy().to_string();
            debug!("Found MP3 file: {}", mp3_path);
            Ok(mp3_path)
        } else {
            debug!("No MP3 file found in directory: {}", path_or_dir);
            Err(anyhow!("No MP3 file found in directory: {}", path_or_dir))
        }
    } else {
        debug!("Path does not exist or is not accessible: {}", path_or_dir);
        Err(anyhow!(
            "Path does not exist or is not accessible: {}",
            path_or_dir
        ))
    }
}
