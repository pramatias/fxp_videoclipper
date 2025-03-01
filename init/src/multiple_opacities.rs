use crate::config::Config;
use anyhow::{Context, Result};
use log::{debug, warn};
use std::env;

/// Enum to represent the source of multiple Opacity values
enum MultipleOpacitySource {
    CliArgument([f32; 3]),
    EnvironmentVariable([f32; 3]),
    FromConfigFile([f32; 3]),
    DefaultValue([f32; 3]),
}

/// Resolves multiple opacity values from CLI arguments, environment variables, or configuration.
///
/// This function determines and validates opacity values by prioritizing CLI arguments,
/// then environment variables, and finally configuration file settings.
///
/// # Parameters
/// - `cli_opacities`: Optional array of 3 f32 values provided via CLI.
/// - `config`: Configuration containing fallback opacity values.
///
/// # Returns
/// - `Result<[f32; 3]>`: An array of 3 f32 opacity values or an error.
///
/// # Notes
/// - Prioritizes sources in the order: CLI arguments > environment variable > configuration file.
/// - Falls back to default values `[0.25, 0.5, 0.75]` if no valid sources are found.
pub fn get_multiple_opacities(
    cli_opacities: Option<[f32; 3]>,
    config: &Config,
) -> Result<[f32; 3]> {
    debug!("Starting to resolve multiple Opacity values...");

    let opacity_source = if let Some(opacities) = cli_opacities {
        debug!(
            "Using multiple Opacity values provided via CLI arguments: {:?}",
            opacities
        );
        MultipleOpacitySource::CliArgument(opacities)
    } else if let Ok(env_opacities) = env::var("EMP_TRANSFER_COLORS_MULTIPLE_OPACITIES") {
        let parsed_values: Vec<f32> = env_opacities
            .split(',')
            .map(|s| s.trim().parse::<f32>())
            .collect::<Result<_, _>>()
            .context(format!(
                "Invalid Opacity values in EMP_TRANSFER_COLORS_MULTIPLE_OPACITIES environment variable: '{}'",
                env_opacities
            ))?;

        if parsed_values.len() == 3 {
            debug!(
                "Using multiple Opacity values from environment variable: {:?}",
                parsed_values
            );
            MultipleOpacitySource::EnvironmentVariable([
                parsed_values[0],
                parsed_values[1],
                parsed_values[2],
            ])
        } else {
            warn!("Environment variable does not contain exactly 3 values, falling back...");
            MultipleOpacitySource::FromConfigFile([
                config.multiple_opacities_1,
                config.multiple_opacities_2,
                config.multiple_opacities_3,
            ])
        }
    } else if (0.0..=1.0).contains(&config.multiple_opacities_1)
        && (0.0..=1.0).contains(&config.multiple_opacities_2)
        && (0.0..=1.0).contains(&config.multiple_opacities_3)
    {
        debug!(
            "Using multiple Opacity values from configuration file: [{}, {}, {}]",
            config.multiple_opacities_1, config.multiple_opacities_2, config.multiple_opacities_3
        );
        MultipleOpacitySource::FromConfigFile([
            config.multiple_opacities_1,
            config.multiple_opacities_2,
            config.multiple_opacities_3,
        ])
    } else {
        warn!("No valid multiple Opacity sources provided, falling back to default values [0.5, 0.5, 0.5]");
        MultipleOpacitySource::DefaultValue([0.25, 0.5, 0.75])
    };

    debug!("Resolving multiple Opacity values based on the determined source...");
    resolve_multiple_opacities(opacity_source)
}

fn resolve_multiple_opacities(opacity_source: MultipleOpacitySource) -> Result<[f32; 3]> {
    debug!("Resolving multiple Opacity values based on the provided source...");

    match opacity_source {
        MultipleOpacitySource::CliArgument(opacities)
        | MultipleOpacitySource::EnvironmentVariable(opacities)
        | MultipleOpacitySource::FromConfigFile(opacities)
        | MultipleOpacitySource::DefaultValue(opacities) => {
            debug!("Resolved multiple Opacity values: {:?}", opacities);
            Ok(opacities)
        }
    }
}
