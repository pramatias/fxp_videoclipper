use crate::config::Config;

use anyhow::{anyhow, Context, Result};
use log::debug;
use std::path::{Path, PathBuf};
use std::{env, fs};

use crate::literals::FXP_VIDEOCLIPPER_AUDIO;
use crate::media_duration::media_duration;

const AUDIO_EXTENSIONS: [&str; 3] = ["mp3", "wav", "flac"];

/// Enum to represent the source of the audio file
enum AudioSource {
    CliArgument(String),
    SearchInExportPath(String),
    FromConfigFile(String),
}

enum PathType {
    File(PathBuf),
    Directory(PathBuf),
}

/// Retrieves the duration of an audio audio file from various sources.
///
/// This function determines the audio file source based on the provided input,
/// checking in the following order: CLI argument, environment variable, and
/// configuration file. It then calculates and returns the audio duration.
///
/// # Parameters
/// - `cli_audio`: Optional path to the audio file provided via command line.
/// - `config`: Configuration containing fallback audio path information.
///
/// # Returns
/// - `Result<Option<u64>>`: Duration in milliseconds if successful, `None`
///   if duration calculation fails, or an error if no audio source is found.
///
/// # Notes
/// - Prioritizes CLI argument over environment variable and configuration file.
/// - Returns `Ok(None)` if unable to calculate duration without raising an error.
pub fn get_audio_duration(cli_audio: Option<String>, config: &Config) -> Result<Option<u64>> {
    debug!("Audio audio duration...");

    // Load the default configuration and capture the path
    debug!("Loading default configuration for audio...");
    let config_path = config.audio_path.clone();

    // Determine the audio source
    let audio_source = if let Some(audio_path) = cli_audio {
        debug!("Using audio path provided via CLI argument: {}", audio_path);
        AudioSource::CliArgument(audio_path)
    } else if let Ok(music_video_dir) = env::var(FXP_VIDEOCLIPPER_AUDIO) {
        debug!(
            "Searching for audio in MUSIC_VIDEO environment variable path: {}",
            music_video_dir
        );
        AudioSource::SearchInExportPath(music_video_dir)
    } else if let Some(audio_path) = &config.audio_path {
        debug!("Using audio path from configuration file: {}", audio_path);
        AudioSource::FromConfigFile(audio_path.clone())
    } else {
        debug!("No audio source provided.");
        return Err(anyhow!(
            "No audio source provided and no fallback methods available"
        ));
    };

    // Resolve the audio path, passing the config_path as an argument
    debug!("Considering all audio sources...");
    let audio_path =
        resolve_audio_path(audio_source, config_path).context("Failed to resolve audio path")?;

    // Calculate the duration of the audio file
    if let Some(ref audio_path_str) = audio_path {
        match media_duration(audio_path_str) {
            Ok(duration) => {
                debug!("Duration of the audio file: {} ms", duration);
                Ok(Some(duration))
            }
            Err(e) => {
                debug!("Failed to get audio duration: {}", e);
                Ok(None) // Keep returning `Ok(None)` if duration calculation fails
            }
        }
    } else {
        debug!("No valid audio path resolved.");
        Ok(None)
    }
}

/// Retrieves the audio file path from various sources.
///
/// This function determines the audio file source based on the provided input,
/// checking in the following order: CLI argument, environment variable, and
/// configuration file. It then resolves and returns the audio file path.
///
/// # Parameters
/// - `cli_audio`: Optional path to the audio file provided via command line.
/// - `config`: Configuration containing fallback audio path information.
///
/// # Returns
/// - `Result<Option<PathBuf>>`: The audio file path if found and resolved, or `None` if no path is found.
///
/// # Notes
/// - Prioritizes CLI argument over environment variable and configuration file.
/// - Returns `Ok(None)` if no valid audio path is found.
pub fn get_audio_file(cli_audio: Option<String>, config: &Config) -> Result<Option<PathBuf>> {
    debug!("Retrieving audio file path...");

    // Load the default configuration and capture the path
    debug!("Loading default configuration for audio...");
    let config_path = config.audio_path.clone();

    // Determine the audio source
    let audio_source = if let Some(audio_path) = cli_audio {
        debug!("Using audio path provided via CLI argument: {}", audio_path);
        AudioSource::CliArgument(audio_path)
    } else if let Ok(music_video_dir) = env::var(FXP_VIDEOCLIPPER_AUDIO) {
        debug!(
            "Searching for audio in MUSIC_VIDEO environment variable path: {}",
            music_video_dir
        );
        AudioSource::SearchInExportPath(music_video_dir)
    } else if let Some(audio_path) = &config.audio_path {
        debug!("Using audio path from configuration file: {}", audio_path);
        AudioSource::FromConfigFile(audio_path.clone())
    } else {
        debug!("No audio source provided.");
        return Ok(None);
    };

    // Resolve the audio path, passing the config_path as an argument
    debug!("Considering all audio sources...");
    let audio_path =
        resolve_audio_path(audio_source, config_path).context("Failed to resolve audio path")?;

    if let Some(ref audio_path_str) = audio_path {
        let path_buf = PathBuf::from(audio_path_str);
        if path_buf.exists() {
            debug!("Resolved audio file path: {:?}", path_buf);
            Ok(Some(path_buf))
        } else {
            debug!("Resolved audio path does not exist: {:?}", path_buf);
            Ok(None)
        }
    } else {
        debug!("No valid audio path resolved.");
        Ok(None)
    }
}

/// Resolves the path to an audio file based on the provided source.
///
/// This function attempts to locate an audio file by checking different sources
/// in a specific order: CLI argument, export path, and configuration file.
///
/// # Parameters
/// - `audio_source`: Specifies where to look for the audio file.
/// - `config_path`: Optional path to a configuration file for fallback.
///
/// # Returns
/// - `Result<Option<String>>`: Path to the found audio file if successful, `None` if not found.
///
/// # Notes
/// - Prioritizes sources in the order: CLI argument > Export directory > Configuration file.
fn resolve_audio_path(
    audio_source: AudioSource,
    config_path: Option<String>,
) -> Result<Option<String>> {
    debug!("Resolving audio path based on the provided source...");

    let mut config_audio_result: Option<Result<String>> = None;

    let result = match audio_source {
        AudioSource::CliArgument(path) => {
            debug!("Using audio path provided via CLI argument: {}", path);
            find_audio_file(&path)
                .map(Some)
                .context("Failed to find audio file from CLI argument")
        }
        AudioSource::SearchInExportPath(dir) => {
            debug!("Searching for audio file in $PATH directory: {}", dir);
            match find_audio_file(&dir) {
                Ok(audio_path) => Ok(Some(audio_path)),
                Err(err) => {
                    debug!("No audio file found in $EXPORT_PATH directory: {}", err);
                    debug!("Falling back to configuration path...");

                    if let Some(config_path) = &config_path {
                        if config_audio_result.is_none() {
                            debug!(
                                "Attempting to use configuration path for audio file resolution: {}",
                                config_path
                            );
                            config_audio_result = Some(
                                find_audio_file(config_path)
                                    .context("Failed to find audio file in configuration path"),
                            );
                        }

                        match &config_audio_result {
                            Some(Ok(audio_path)) => Ok(Some(audio_path.clone())),
                            Some(Err(err)) => {
                                Err(anyhow!("Configuration audio resolution error: {}", err))
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
        AudioSource::FromConfigFile(config_path) => {
            debug!("Using audio path from configuration file: {}", config_path);
            if config_audio_result.is_none() {
                config_audio_result = Some(
                    find_audio_file(&config_path)
                        .context("Failed to find audio file in configuration file"),
                );
            }

            match config_audio_result {
                Some(Ok(ref audio_path)) => Ok(Some(audio_path.clone())),
                Some(Err(ref err)) => Err(anyhow!("Error using audio from config file: {}", err)),
                None => Ok(None),
            }
        }
    };

    // Log and return result gracefully
    if let Ok(Some(ref path)) = result {
        debug!("Resolved audio path: {}", path);
    } else {
        debug!("Failed to resolve audio path.");
    }

    result.or_else(|err| {
        debug!("Returning None due to resolution failure: {}", err);
        Ok(None)
    })
}

impl PathType {
    /// Creates a `PathType` from a given `Path`.
    ///
    /// This function determines whether the provided path points to a file or directory,
    /// and constructs the appropriate `PathType` accordingly.
    ///
    /// # Parameters
    /// - `path`: The path to evaluate as either a file or directory.
    ///
    /// # Returns
    /// - `Result<Self>`: Returns the constructed `PathType` on success, or an error
    ///   if the path does not exist or is not accessible.
    ///
    /// # Notes
    /// - This function does not create the path on the filesystem; it merely constructs
    ///   a `PathType` based on the existing path's properties.
    fn from_path(path: &Path) -> Result<Self> {
        if path.is_file() {
            Ok(PathType::File(path.to_path_buf()))
        } else if path.is_dir() {
            Ok(PathType::Directory(path.to_path_buf()))
        } else {
            Err(anyhow!(
                "Path does not exist or is not accessible: {}",
                path.to_string_lossy()
            ))
        }
    }

    /// Discovers and returns the path of a supported audio file.
    ///
    /// This function identifies and validates audio files based on their extensions.
    ///
    /// # Parameters
    /// - `self`: The current path to evaluate for an audio file or directory.
    ///
    /// # Returns
    /// - `Result<String>`: The path to a supported audio file on success, or an error if no valid audio file is found.
    ///
    /// # Notes
    /// - If the input is a file, it checks if the file extension matches supported audio types.
    /// - If the input is a directory, it searches for the first supported audio file within the directory.
    /// - Returns an error if the input path does not resolve to a valid audio file or directory.
    fn find_audio(self) -> Result<String> {
        match self {
            PathType::File(path) => {
                if let Some(ext) = path.extension().and_then(|ext| ext.to_str()) {
                    if AUDIO_EXTENSIONS.contains(&ext) {
                        let audio_path = path.to_string_lossy().to_string();
                        debug!("Input is a valid audio file: {}", audio_path);
                        return Ok(audio_path);
                    }
                }
                debug!(
                    "Input path is a file but not a supported audio file: {}",
                    path.to_string_lossy()
                );
                Err(anyhow!(
                    "The specified file is not a supported audio file: {}",
                    path.to_string_lossy()
                ))
            }
            PathType::Directory(path) => {
                debug!(
                    "Searching for audio file in directory: {}",
                    path.to_string_lossy()
                );
                let audio_entry = fs::read_dir(&path)
                    .context(format!(
                        "Failed to read directory: {}",
                        path.to_string_lossy()
                    ))?
                    .filter_map(Result::ok)
                    .find(|entry| {
                        entry
                            .path()
                            .extension()
                            .and_then(|ext| ext.to_str())
                            .map_or(false, |ext| AUDIO_EXTENSIONS.contains(&ext))
                    });

                if let Some(entry) = audio_entry {
                    let audio_path = entry.path().to_string_lossy().to_string();
                    debug!("Found audio file: {}", audio_path);
                    Ok(audio_path)
                } else {
                    debug!(
                        "No audio file found in directory: {}",
                        path.to_string_lossy()
                    );
                    Err(anyhow!(
                        "No supported audio file found in directory: {}",
                        path.to_string_lossy()
                    ))
                }
            }
        }
    }
}

/// Finds an audio file within a given directory or path.
///
/// This function searches for an audio file either in the specified directory or
/// the provided file path.
///
/// # Parameters
/// - `path_or_dir`: A string representing the directory or file path to search for the audio file.
///
/// # Returns
/// - `Result<String>`: A `Result` containing the path to the found audio file as a `String` on success, or an error on failure.
fn find_audio_file(path_or_dir: &str) -> Result<String> {
    let path = Path::new(path_or_dir);
    let path_type = PathType::from_path(path)?;
    path_type.find_audio()
}
