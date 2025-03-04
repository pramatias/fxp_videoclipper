use crate::config::Config;
use log::debug;
use std::env;

/// Determines the sampling number based on provided parameters and configuration.
///
/// # Parameters
/// - `multiple`: A boolean indicating whether multiple frames are being processed.
/// - `number`: An optional number provided via CLI argument.
/// - `config`: A reference to the configuration containing sampling settings.
///
/// # Returns
/// - `usize`: The resolved sampling number.
///
/// # Behavior
/// - **Multiple Frames (`multiple == true`):**
///   - If `number` is provided, its value is used.
///   - Otherwise, defaults to **10**.
/// - **Single Frame (`multiple == false`):**
///   - If `number` is provided, its value is used.
///   - Otherwise, a `Sampling` is created from `config`.
///     - If a valid sampling number exists in the configuration, that is used.
///     - Otherwise, the default of **1** is returned.
pub fn get_sampling_number(multiple: bool, number: Option<usize>, config: &Config) -> usize {
    if multiple {
        // For multiple frame sampling, CLI argument (if provided) takes priority,
        // and if absent, we default to 10.
        number.unwrap_or(10)
    } else {
        // For single frame sampling, CLI argument (if provided) takes priority,
        // otherwise we try to extract a number from the configuration using Sampling.
        number.unwrap_or_else(|| Sampling::new(config).number)
    }
}

/// A helper struct for single-frame sampling configuration.
/// It tries to derive the sampling number from the configuration.
#[derive(Debug)]
pub struct Sampling {
    pub number: usize,
}

impl Default for Sampling {
    /// The default sampling number is 1.
    fn default() -> Self {
        Self { number: 1 }
    }
}

impl Sampling {
    /// Creates a new `Sampling` based on the provided configuration.
    ///
    /// The logic is as follows:
    /// - First, check the FRAME_EXPORTER_SAMPLING_NUMBER environment variable.
    ///   If it exists and can be parsed to a positive usize, that value is used.
    /// - Otherwise, if the configuration's `sampling_number` is greater than 0, that value is used.
    /// - If neither is provided, the default value (1) is used.
    pub fn new(config: &Config) -> Self {
        let env_number = if let Ok(env_value) = env::var("FRAME_EXPORTER_SAMPLING_NUMBER") {
            if let Ok(parsed_value) = env_value.parse::<usize>() {
                if parsed_value > 0 {
                    debug!(
                        "Using sampling number from FRAME_EXPORTER_SAMPLING_NUMBER environment variable: {}",
                        parsed_value
                    );
                    Some(parsed_value)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        let config_number = if config.sampling_number > 0 {
            debug!(
                "Using sampling number from configuration file: {}",
                config.sampling_number
            );
            Some(config.sampling_number)
        } else {
            None
        };

        // Use the value from the environment or config; fall back to default if neither is valid.
        env_number
            .or(config_number)
            .map(|number| Self { number })
            .unwrap_or_else(|| {
                debug!("No valid sampling number provided; defaulting to Sampling::default().");
                Self::default()
            })
    }
}
