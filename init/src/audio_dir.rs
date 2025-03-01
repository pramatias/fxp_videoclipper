use crate::config::Config;

use anyhow::{Context, Result};
use log::debug;
use std::env;
use std::path::PathBuf;

/// Enum to represent the source of the audio directory
enum AudioDirSource {
    CliArgument(String),
    EnvVar(String),
    FromConfigFile(String),
    Default,
}

/// Retrieves the audio directory from various sources.
///
/// This function determines the audio directory based on the provided input,
/// checking in the following order: CLI argument, environment variable, and
/// configuration file. If no audio directory is found, it defaults to the current directory.
///
/// # Parameters
/// - `cli_audio`: Optional audio directory provided via the command line.
/// - `config`: Configuration containing fallback audio directory information.
///
/// # Returns
/// - `Result<PathBuf>`: The resolved audio directory path, or an error if resolution fails.
///
/// # Notes
/// - Prioritizes CLI argument over environment variable and configuration file.
/// - Defaults to the current directory if no other audio directory is found.
pub fn get_audio_dir(cli_audio: Option<String>, config: &Config) -> Result<PathBuf> {
    debug!("Determining audio directory...");

    // Capture the audio directory from configuration (if available and not empty)
    let config_audio_dir = config.mp3_path.clone().filter(|s| !s.trim().is_empty());

    // Determine the audio directory source
    let audio_dir_source = if let Some(cli_dir) = cli_audio {
        debug!(
            "Using audio directory provided via CLI argument: {}",
            cli_dir
        );
        AudioDirSource::CliArgument(cli_dir)
    } else if let Ok(env_dir) = env::var("FXP_VIDEOCLIPPER_AUDIO") {
        debug!(
            "Using audio directory from environment variable FXP_VIDEOCLIPPER_AUDIO: {}",
            env_dir
        );
        AudioDirSource::EnvVar(env_dir)
    } else if let Some(cfg_dir) = config_audio_dir {
        debug!("Using audio directory from configuration file: {}", cfg_dir);
        AudioDirSource::FromConfigFile(cfg_dir)
    } else {
        debug!("No audio directory provided. Defaulting to current directory.");
        AudioDirSource::Default
    };

    // Resolve the audio directory without additional resolution steps
    let audio_dir = match audio_dir_source {
        AudioDirSource::CliArgument(ref path)
        | AudioDirSource::EnvVar(ref path)
        | AudioDirSource::FromConfigFile(ref path) => PathBuf::from(path),
        AudioDirSource::Default => {
            std::env::current_dir().context("Failed to get current directory")?
        }
    };

    debug!("Resolved audio directory: {:?}", audio_dir);
    Ok(audio_dir)
}
