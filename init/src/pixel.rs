use crate::config::Config;
use anyhow::{anyhow, Context, Result};
use log::{debug, warn};
use std::env;

/// Enum to represent the source of the Pixel Upper Limit value
enum PixelLimitSource {
    CliArgument(u32),
    EnvironmentVariable(u32),
    FromConfigFile(u32),
}

/// Retrieves and determines the appropriate pixel upper limit from available sources.
///
/// This function evaluates multiple sources to establish the pixel upper limit,
/// following a defined order of precedence: CLI argument, environment variable,
/// and configuration file.
///
/// # Parameters
/// - `cli_pixel_limit`: An optional `u32` provided via command-line argument.
/// - `config`: A reference to the `Config` struct containing application settings.
///
/// # Returns
/// - `Result<u32>`: The resolved pixel upper limit value.
///   - `Ok(u32)`: Successfully determined pixel limit.
///   - `Err(anyhow::Error)`: If no valid source is found or parsing fails.
///
/// # Notes
/// - The function prioritizes sources in the following order:
///   1. Command-line argument (`--pixel-limit`)
///   2. Environment variable (`FRAME_EXPORTER_PIXEL_LIMIT`)
///   3. Configuration file setting
/// - If no valid source is available, returns an error.
pub fn get_pixel_upper_limit(cli_pixel_limit: Option<u32>, config: &Config) -> Result<u32> {
    // Log the start of the function
    debug!("Starting to resolve Pixel Upper Limit...");

    // Determine the Pixel Upper Limit source
    let pixel_limit_source = if let Some(pixel_value) = cli_pixel_limit {
        debug!(
            "Using Pixel Upper Limit provided via CLI argument: {}",
            pixel_value
        );
        PixelLimitSource::CliArgument(pixel_value)
    } else if let Ok(env_pixel_value) = env::var("FRAME_EXPORTER_PIXEL_LIMIT") {
        let parsed_value = env_pixel_value.parse::<u32>().context(format!(
            "Invalid Pixel Upper Limit in FRAME_EXPORTER_PIXEL_LIMIT environment variable: '{}",
            env_pixel_value
        ))?;
        debug!(
            "Using Pixel Upper Limit from PIXEL_LIMIT environment variable: {}",
            parsed_value
        );
        PixelLimitSource::EnvironmentVariable(parsed_value)
    } else if config.pixel_upper_limit > 0 {
        debug!(
            "Using Pixel Upper Limit from configuration file: {}",
            config.pixel_upper_limit
        );
        PixelLimitSource::FromConfigFile(config.pixel_upper_limit)
    } else {
        warn!("No Pixel Upper Limit source provided and no fallback methods available");
        return Err(anyhow!(
            "No Pixel Upper Limit source provided and no fallback methods available"
        ));
    };

    // Resolve the Pixel Upper Limit value
    debug!("Resolving Pixel Upper Limit value based on the determined source...");
    resolve_pixel_limit(pixel_limit_source)
}

/// Resolves the Pixel Upper Limit value based on the provided source.
///
/// This function retrieves and returns the pixel limit from one of three possible sources:
/// CLI argument, environment variable, or configuration file.
///
/// # Parameters
/// - `pixel_limit_source`: An enum representing the source of the pixel limit.
///
/// # Returns
/// - `Result<u32>`: The resolved pixel limit, or an error if resolution fails.
///
/// # Notes
/// - Logs the source from which the pixel limit was retrieved for debugging purposes.
fn resolve_pixel_limit(pixel_limit_source: PixelLimitSource) -> Result<u32> {
    debug!("Resolving Pixel Upper Limit value based on the provided source...");

    match pixel_limit_source {
        PixelLimitSource::CliArgument(pixel_limit) => {
            debug!(
                "Using Pixel Upper Limit provided via CLI argument: {}",
                pixel_limit
            );
            Ok(pixel_limit)
        }
        PixelLimitSource::EnvironmentVariable(pixel_limit) => {
            debug!(
                "Using Pixel Upper Limit from environment variable: {}",
                pixel_limit
            );
            Ok(pixel_limit)
        }
        PixelLimitSource::FromConfigFile(pixel_limit) => {
            debug!(
                "Using Pixel Upper Limit from configuration file: {}",
                pixel_limit
            );
            Ok(pixel_limit)
        }
    }
}
