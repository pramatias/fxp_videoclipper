use crate::config::Config;
use anyhow::{Context, Result};
use log::{debug, warn};
use std::env;

/// Enum to represent the source of the Opacity value
enum OpacitySource {
    CliArgument(f32),
    EnvironmentVariable(f32),
    FromConfigFile(f32),
    DefaultValue(f32),
}

/// Resolves the Opacity value based on CLI arguments, environment variables, or the configuration.
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
    } else if let Ok(env_opacity_value) = env::var("EMP_TRANSFER_COLORS_OPACITY") {
        let parsed_value = env_opacity_value.parse::<f32>().context(format!(
            "Invalid Opacity value in EMP_TRANSFER_COLORS_OPACITY environment variable: '{}",
            env_opacity_value
        ))?;
        debug!(
            "Using Opacity value from EMP_TRANSFER_COLORS_OPACITY environment variable: {}",
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
