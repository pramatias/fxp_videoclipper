use crate::config::Config;
use crate::media_duration::media_duration;

use anyhow::{anyhow, Context, Result};
use log::debug;
use std::path::{Path, PathBuf};
use std::{env, fs};

/// Enum to represent the source of the MP3 file
enum Mp3Source {
    CliArgument(String),
    SearchInExportPath(String),
    FromConfigFile(String),
}

/// Retrieves the duration of an MP3 audio file from various sources.
///
/// This function determines the MP3 file source based on the provided input,
/// checking in the following order: CLI argument, environment variable, and
/// configuration file. It then calculates and returns the audio duration.
///
/// # Parameters
/// - `cli_mp3`: Optional path to the MP3 file provided via command line.
/// - `config`: Configuration containing fallback MP3 path information.
///
/// # Returns
/// - `Result<Option<u64>>`: Duration in milliseconds if successful, `None`
///   if duration calculation fails, or an error if no MP3 source is found.
///
/// # Notes
/// - Prioritizes CLI argument over environment variable and configuration file.
/// - Returns `Ok(None)` if unable to calculate duration without raising an error.
pub fn get_audio_duration(cli_mp3: Option<String>, config: &Config) -> Result<Option<u64>> {
    debug!("Mp3 audio duration...");

    // Load the default configuration and capture the path
    debug!("Loading default configuration for MP3...");
    let config_path = config.mp3_path.clone();

    // Determine the MP3 source
    let mp3_source = if let Some(mp3_path) = cli_mp3 {
        debug!("Using MP3 path provided via CLI argument: {}", mp3_path);
        Mp3Source::CliArgument(mp3_path)
    } else if let Ok(music_video_dir) = env::var("FXP_VIDEOCLIPPER_AUDIO") {
        debug!(
            "Searching for MP3 in MUSIC_VIDEO environment variable path: {}",
            music_video_dir
        );
        Mp3Source::SearchInExportPath(music_video_dir)
    } else if let Some(mp3_path) = &config.mp3_path {
        debug!("Using MP3 path from configuration file: {}", mp3_path);
        Mp3Source::FromConfigFile(mp3_path.clone())
    } else {
        debug!("No MP3 source provided.");
        return Err(anyhow!(
            "No MP3 source provided and no fallback methods available"
        ));
    };

    // Resolve the MP3 path, passing the config_path as an argument
    debug!("Considering all MP3 sources...");
    let mp3_path =
        resolve_mp3_path(mp3_source, config_path).context("Failed to resolve MP3 path")?;

    // Calculate the duration of the MP3 file
    if let Some(ref mp3_path_str) = mp3_path {
        match media_duration(mp3_path_str) {
            Ok(duration) => {
                debug!("Duration of the MP3 file: {} ms", duration);
                Ok(Some(duration))
            }
            Err(e) => {
                debug!("Failed to get MP3 duration: {}", e);
                Ok(None) // Keep returning `Ok(None)` if duration calculation fails
            }
        }
    } else {
        debug!("No valid MP3 path resolved.");
        Ok(None)
    }
}

/// Retrieves the MP3 file path from various sources.
///
/// This function determines the MP3 file source based on the provided input,
/// checking in the following order: CLI argument, environment variable, and
/// configuration file. It then resolves and returns the MP3 file path.
///
/// # Parameters
/// - `cli_mp3`: Optional path to the MP3 file provided via command line.
/// - `config`: Configuration containing fallback MP3 path information.
///
/// # Returns
/// - `Result<Option<PathBuf>>`: The MP3 file path if found and resolved, or `None` if no path is found.
///
/// # Notes
/// - Prioritizes CLI argument over environment variable and configuration file.
/// - Returns `Ok(None)` if no valid MP3 path is found.
pub fn get_audio_file(cli_mp3: Option<String>, config: &Config) -> Result<Option<PathBuf>> {
    debug!("Retrieving MP3 file path...");

    // Load the default configuration and capture the path
    debug!("Loading default configuration for MP3...");
    let config_path = config.mp3_path.clone();

    // Determine the MP3 source
    let mp3_source = if let Some(mp3_path) = cli_mp3 {
        debug!("Using MP3 path provided via CLI argument: {}", mp3_path);
        Mp3Source::CliArgument(mp3_path)
    } else if let Ok(music_video_dir) = env::var("FXP_VIDEOCLIPPER_AUDIO") {
        debug!(
            "Searching for MP3 in MUSIC_VIDEO environment variable path: {}",
            music_video_dir
        );
        Mp3Source::SearchInExportPath(music_video_dir)
    } else if let Some(mp3_path) = &config.mp3_path {
        debug!("Using MP3 path from configuration file: {}", mp3_path);
        Mp3Source::FromConfigFile(mp3_path.clone())
    } else {
        debug!("No MP3 source provided.");
        return Ok(None);
    };

    // Resolve the MP3 path, passing the config_path as an argument
    debug!("Considering all MP3 sources...");
    let mp3_path =
        resolve_mp3_path(mp3_source, config_path).context("Failed to resolve MP3 path")?;

    if let Some(ref mp3_path_str) = mp3_path {
        let path_buf = PathBuf::from(mp3_path_str);
        if path_buf.exists() {
            debug!("Resolved MP3 file path: {:?}", path_buf);
            Ok(Some(path_buf))
        } else {
            debug!("Resolved MP3 path does not exist: {:?}", path_buf);
            Ok(None)
        }
    } else {
        debug!("No valid MP3 path resolved.");
        Ok(None)
    }
}

/// Resolves the path to an MP3 file based on the provided source.
///
/// This function attempts to locate an MP3 file by checking different sources
/// in a specific order: CLI argument, export path, and configuration file.
///
/// # Parameters
/// - `mp3_source`: Specifies where to look for the MP3 file.
/// - `config_path`: Optional path to a configuration file for fallback.
///
/// # Returns
/// - `Result<Option<String>>`: Path to the found MP3 file if successful, `None` if not found.
///
/// # Notes
/// - Prioritizes sources in the order: CLI argument > Export directory > Configuration file.
/// - Logs debug information for each resolution step.
fn resolve_mp3_path(mp3_source: Mp3Source, config_path: Option<String>) -> Result<Option<String>> {
    debug!("Resolving MP3 path based on the provided source...");

    let mut config_mp3_result: Option<Result<String>> = None;

    let result = match mp3_source {
        Mp3Source::CliArgument(path) => {
            debug!("Using MP3 path provided via CLI argument: {}", path);
            find_mp3_file(&path)
                .map(Some)
                .context("Failed to find MP3 file from CLI argument")
        }
        Mp3Source::SearchInExportPath(dir) => {
            debug!("Searching for MP3 file in $PATH directory: {}", dir);
            match find_mp3_file(&dir) {
                Ok(mp3_path) => Ok(Some(mp3_path)),
                Err(err) => {
                    debug!("No MP3 file found in $EXPORT_PATH directory: {}", err);
                    debug!("Falling back to configuration path...");

                    if let Some(config_path) = &config_path {
                        if config_mp3_result.is_none() {
                            debug!(
                                "Attempting to use configuration path for MP3 file resolution: {}",
                                config_path
                            );
                            config_mp3_result = Some(
                                find_mp3_file(config_path)
                                    .context("Failed to find MP3 file in configuration path"),
                            );
                        }

                        match &config_mp3_result {
                            Some(Ok(mp3_path)) => Ok(Some(mp3_path.clone())),
                            Some(Err(err)) => {
                                Err(anyhow!("Configuration MP3 resolution error: {}", err))
                            }
                            None => Ok(None),
                        }
                    } else {
                        debug!("Configuration path is not provided or is unavailable.");
                        Ok(None)
                    }
                }
            }
        }
        Mp3Source::FromConfigFile(config_path) => {
            debug!("Using MP3 path from configuration file: {}", config_path);
            if config_mp3_result.is_none() {
                config_mp3_result = Some(
                    find_mp3_file(&config_path)
                        .context("Failed to find MP3 file in configuration file"),
                );
            }

            match config_mp3_result {
                Some(Ok(ref mp3_path)) => Ok(Some(mp3_path.clone())),
                Some(Err(ref err)) => Err(anyhow!("Error using MP3 from config file: {}", err)),
                None => Ok(None),
            }
        }
    };

    // Log and return result gracefully
    if let Ok(Some(ref path)) = result {
        debug!("Resolved MP3 path: {}", path);
    } else {
        debug!("Failed to resolve MP3 path.");
    }

    result.or_else(|err| {
        debug!("Returning None due to resolution failure: {}", err);
        Ok(None)
    })
}

/// Locates and validates an MP3 file from a given path or directory.
///
/// This function checks if the provided path is a file or directory. If it's a file,
/// it verifies if it's an MP3. If it's a directory, it searches for an MP3 file within.
///
/// # Parameters
/// - `path_or_dir`: A string representing the path to a file or directory to search.
///
/// # Returns
/// - `Result<String>`: The path to the found MP3 file on success. Returns an error if no MP3 is found or if the path is invalid.
///
/// # Notes
/// - If the path is a directory, the function returns the first MP3 file found.
/// - If no MP3 file is found in the directory, an error is returned
fn find_mp3_file(path_or_dir: &str) -> Result<String> {
    // Attempt to get the export path from the $MUSIC_VIDEO environment variable
    let path = Path::new(&path_or_dir);

    if path.is_file() {
        // If the path is a file, check if it is an MP3 file
        if path.extension().and_then(|ext| ext.to_str()) == Some("mp3") {
            let mp3_path = path.to_string_lossy().to_string();
            debug!("Input is a valid MP3 file: {}", mp3_path);
            return Ok(mp3_path);
        } else {
            debug!("Input path is a file but not an MP3: {}", path_or_dir);
            return Err(anyhow!(
                "The specified path is a file but not an MP3: {}",
                path_or_dir
            ));
        }
    } else if path.is_dir() {
        // If the path is a directory, search for an MP3 file
        debug!("Searching for MP3 file in directory: {}", path_or_dir);
        let mp3_file = fs::read_dir(path)
            .context(format!("Failed to read directory: {}", path_or_dir))?
            .filter_map(Result::ok)
            .find(|entry| entry.path().extension().and_then(|ext| ext.to_str()) == Some("mp3"));

        if let Some(entry) = mp3_file {
            let mp3_path = entry.path().to_string_lossy().to_string();
            debug!("Found MP3 file: {}", mp3_path);
            return Ok(mp3_path);
        } else {
            debug!("No MP3 file found in directory: {}", path_or_dir);
            return Err(anyhow!("No MP3 file found in directory: {}", path_or_dir));
        }
    } else {
        debug!("Path does not exist or is not accessible: {}", path_or_dir);
        return Err(anyhow!(
            "Path does not exist or is not accessible: {}",
            path_or_dir
        ));
    }
}
