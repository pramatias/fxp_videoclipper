use crate::config::Config;
use anyhow::{anyhow, Context, Result};
use log::{debug, warn};
use std::env;

use crate::literals::FXP_VIDEOCLIPPER_FPS;

/// Enum to represent the source of the FPS value
enum FpsSource {
    CliArgument(u32),
    EnvironmentVariable,
    FromConfigFile(u32),
}

/// Retrieves the Frames Per Second (FPS) value from multiple sources.
///
/// This function prioritizes different sources in the following order:
/// 1. Command-line argument (`cli_fps`)
/// 2. Environment variable (`FXP_VIDEOCLIPPER_FPS`)
/// 3. Configuration file (`config.fps`)
///
/// # Parameters
/// - `cli_fps`: Optional FPS value provided via the command line.
/// - `config`: Configuration struct containing the FPS value if not set elsewhere.
///
/// # Returns
/// - `Result<u32>`: The determined FPS value or an error if no sources are available.
///
/// # Notes
/// - If no FPS sources are provided, the function will return an error.
pub fn get_fps(cli_fps: Option<u32>, config: &Config) -> Result<u32> {
    // Log the start of the function
    debug!("Starting to resolve FPS...");

    // Determine the FPS source
    let fps_source = if let Some(fps_value) = cli_fps {
        debug!("Using FPS provided via CLI argument: {}", fps_value);
        FpsSource::CliArgument(fps_value)
    } else if env::var(FXP_VIDEOCLIPPER_FPS).is_ok() {
        debug!("Using FPS from FXP_VIDEOCLIPPER_FPS environment variable.");
        FpsSource::EnvironmentVariable
    } else if config.fps > 0 {
        debug!("Using FPS from configuration file: {}", config.fps);
        FpsSource::FromConfigFile(config.fps)
    } else {
        warn!("No FPS source provided and no fallback methods available");
        return Err(anyhow!(
            "No FPS source provided and no fallback methods available"
        ));
    };

    // Resolve the FPS value
    debug!("Resolving FPS value based on the determined source...");
    resolve_fps(fps_source)
}

/// Resolves Frames Per Second (FPS) value based on the provided source.
///
/// This function determines the FPS value by evaluating the given source type,
/// handling CLI arguments, environment variables, and configuration files.
///
/// # Parameters
/// - `fps_source`: The source from which to resolve the FPS value.
///
/// # Returns
/// - `Result<u32>`: The resolved FPS value as an unsigned 32-bit integer,
///                    or an error if resolution fails.
///
/// # Notes
/// - Prioritizes sources in the order: CLI argument > Environment variable > Config file.
/// - Validates and parses the FPS value to ensure it is a valid unsigned integer.
fn resolve_fps(fps_source: FpsSource) -> Result<u32> {
    debug!("Resolving FPS value based on the provided source...");

    match fps_source {
        FpsSource::CliArgument(fps) => {
            debug!("Using FPS provided via CLI argument: {}", fps);
            Ok(fps)
        }
        FpsSource::EnvironmentVariable => {
            debug!("Searching for FPS in FXP_VIDEOCLIPPER_FPS environment variable...");
            let fps_str = env::var(FXP_VIDEOCLIPPER_FPS)
                .context("Failed to read FXP_VIDEOCLIPPER_FPS environment variable")?;
            let fps = fps_str.parse::<u32>().context(format!(
                "Invalid FPS value in FXP_VIDEOCLIPPER_FPS: '{}",
                fps_str
            ))?;
            Ok(fps)
        }
        FpsSource::FromConfigFile(fps) => {
            debug!("Using FPS from configuration file: {}", fps);
            Ok(fps)
        }
    }
}
