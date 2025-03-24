use crate::config::Config;
use anyhow::{Context, Result};
use log::{debug, warn};
use std::env;

use crate::literals::FXP_VIDEOCLIPPER_OPACITY;

/// Enum to represent the source of the Opacity value
enum OpacitySource {
    CliArgument(f32),
    EnvironmentVariable(f32),
    FromConfigFile(f32),
    DefaultValue(f32),
}

/// Retrieves and validates the opacity value from multiple sources.
///
/// This function checks for an opacity value in the following order:
/// 1. CLI argument
/// 2. Environment variable
/// 3. Configuration file
/// 4. Default value (if all else fails)
///
/// # Parameters
/// - `cli_opacity`: An optional opacity value provided via CLI argument
/// - `config`: A reference to the configuration file containing opacity setting
///
/// # Returns
/// - `Result<f32>`: The resolved opacity value if successful, otherwise an error
///
/// # Notes
/// - The function prioritizes sources in the following order: CLI > Environment > Config > Default
/// - The opacity value must be between 0.0 and 1.0 to be considered valid
pub fn get_opacity(cli_opacity: Option<f32>, config: &Config) -> Result<f32> {
    // Log the start of the function
    debug!("Starting to resolve Opacity value...");

    // Determine the Opacity source
    let opacity_source = if let Some(opacity_value) = cli_opacity {
        debug!(
            "Using Opacity value provided via CLI argument: {}",
            opacity_value
        );
        OpacitySource::CliArgument(opacity_value)
    } else if let Ok(env_opacity_value) = env::var(FXP_VIDEOCLIPPER_OPACITY) {
        let parsed_value = env_opacity_value.parse::<f32>().context(format!(
            "Invalid Opacity value in FXP_VIDEOCLIPPER_OPACITY environment variable: '{}",
            env_opacity_value
        ))?;
        debug!(
            "Using Opacity value from FXP_VIDEOCLIPPER_OPACITY environment variable: {}",
            parsed_value
        );
        OpacitySource::EnvironmentVariable(parsed_value)
    } else if config.opacity >= 0.0 && config.opacity <= 1.0 {
        debug!(
            "Using Opacity value from configuration file: {}",
            config.opacity
        );
        OpacitySource::FromConfigFile(config.opacity)
    } else {
        warn!("No valid Opacity source provided, falling back to default value of 0.5");
        OpacitySource::DefaultValue(0.5)
    };

    // Resolve the Opacity value
    debug!("Resolving Opacity value based on the determined source...");
    resolve_opacity(opacity_source)
}

/// Resolves opacity value based on the provided source, handling different input origins.
///
/// This function determines and returns the opacity value by evaluating the source
/// of the input, which can come from various origins like CLI arguments, environment
/// variables, or configuration files.
///
/// # Parameters
/// - `opacity_source`: The source containing the opacity value to be resolved.
///
/// # Returns
/// - `Result<f32>`: A success result containing the resolved opacity value, or
///   an error if the resolution fails.
///
/// # Notes
/// - The function logs which source was used to obtain the opacity value for
///   debugging purposes.
fn resolve_opacity(opacity_source: OpacitySource) -> Result<f32> {
    debug!("Resolving Opacity value based on the provided source...");

    match opacity_source {
        OpacitySource::CliArgument(opacity) => {
            debug!("Using Opacity value provided via CLI argument: {}", opacity);
            Ok(opacity)
        }
        OpacitySource::EnvironmentVariable(opacity) => {
            debug!("Using Opacity value from environment variable: {}", opacity);
            Ok(opacity)
        }
        OpacitySource::FromConfigFile(opacity) => {
            debug!("Using Opacity value from configuration file: {}", opacity);
            Ok(opacity)
        }
        OpacitySource::DefaultValue(opacity) => {
            debug!("Using default Opacity value: {}", opacity);
            Ok(opacity)
        }
    }
}
