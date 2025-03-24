use crate::config::Config;
use log::debug;
use std::env;

use crate::literals::FXP_VIDEOCLIPPER_SAMPLING_NUMBER;

enum SamplingSource {
    Env(usize),
    Config(usize),
    Default,
}

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
        // and if absent, we use MultiSampling::new(config) to derive the number.
        number.unwrap_or_else(|| MultiSampling::default().number)
    } else {
        // For single frame sampling, CLI argument (if provided) takes priority,
        // otherwise we extract a number from the configuration using Sampling.
        number.unwrap_or_else(|| Sampling::new(config).number)
    }
}

/// A helper struct for single-frame sampling configuration.
/// It tries to derive the sampling number from the configuration.
#[derive(Debug)]
pub struct Sampling {
    pub number: usize,
}

/// A helper struct for multiple-frame sampling configuration.
/// It defaults the sampling number to 10.
#[derive(Debug)]
pub struct MultiSampling {
    pub number: usize,
}

impl Default for MultiSampling {
    /// The default sampling number for multiple frames is 10.
    fn default() -> Self {
        Self { number: 10 }
    }
}

impl Default for Sampling {
    /// The default sampling number is 1.
    fn default() -> Self {
        Self { number: 1 }
    }
}

impl Sampling {
    /// Creates a new `Sampling` instance based on configuration and environment settings.
    ///
    /// The function determines the sampling number through the following priority order:
    /// 1. The FXP_VIDEOCLIPPER_SAMPLING_NUMBER environment variable (if valid)
    /// 2. The `sampling_number` from the provided Config (if greater than 0)
    /// 3. Defaults to 1 if no valid sources are found
    ///
    /// # Parameters
    /// - `config`: A reference to the configuration containing sampling settings
    ///
    /// # Returns
    /// - `Self`: The newly created `Sampling` instance with the determined number
    ///
    /// # Notes
    /// - The environment variable must be a positive integer to be considered valid
    /// - The default sampling number is 1 when no valid sources are provided
    pub fn new(config: &Config) -> Self {
        // Determine the sampling number source using the enum.
        let sampling_source = match env::var(FXP_VIDEOCLIPPER_SAMPLING_NUMBER) {
            Ok(env_value) => match env_value.parse::<usize>() {
                Ok(val) if val > 0 => SamplingSource::Env(val),
                _ => {
                    if config.sampling_number > 0 {
                        SamplingSource::Config(config.sampling_number)
                    } else {
                        SamplingSource::Default
                    }
                }
            },
            Err(_) => {
                if config.sampling_number > 0 {
                    SamplingSource::Config(config.sampling_number)
                } else {
                    SamplingSource::Default
                }
            }
        };

        // Use a match statement to choose the appropriate branch based on the source.
        match sampling_source {
            SamplingSource::Env(num) => {
                debug!("Using sampling number from FXP_VIDEOCLIPPER_SAMPLING_NUMBER environment variable: {}", num);
                Self { number: num }
            }
            SamplingSource::Config(num) => {
                debug!("Using sampling number from configuration file: {}", num);
                Self { number: num }
            }
            SamplingSource::Default => {
                debug!("No valid sampling number provided; defaulting to Sampling::default().");
                Self::default()
            }
        }
    }
}
