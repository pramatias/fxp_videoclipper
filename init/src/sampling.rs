use crate::config::Config;
use log::debug;
use std::env;

/// Determines the sampling number based on provided parameters and configuration.
///
/// This function evaluates multiple sources to determine the appropriate sampling number.
/// It prioritizes values in the following order: CLI argument, environment variable,
/// and configuration file. If no valid sampling number is found and sampling is enabled,
/// it defaults to 1 for single-frame sampling or returns `None` if sampling is disabled.
///
/// # Parameters
/// - `sampling`: Boolean indicating whether sampling is enabled.
/// - `multiple`: Boolean indicating whether multiple frames are being processed.
/// - `number`: Optional number provided via CLI argument.
/// - `config`: Reference to the configuration file containing sampling settings.
///
/// # Returns
/// - `Option<usize>`: The resolved sampling number, or `None` if no valid number is found.
///
/// # Notes
/// - The function prioritizes the sampling number in this order: CLI argument > environment variable > config file.
/// - If sampling is enabled and `multiple` is true, the function will only use a number if it is greater than 0.
/// - If sampling is enabled but no valid number is found, it defaults to 1 for single-frame sampling.
/// - If sampling is disabled, the function returns `None`.
pub fn get_sampling_number(
    sampling: bool,
    multiple: bool,
    number: Option<usize>,
    config: &Config,
) -> Option<usize> {
    debug!(
        "Resolving sampling number with arguments: sampling={}, multiple={}, number={:?}",
        sampling, multiple, number
    );

    let sampling_source = if sampling && multiple {
        if let Some(n) = number {
            if n > 0 {
                debug!("Using sampling number provided via CLI argument: {}", n);
                Some(n)
            } else {
                None
            }
        } else if let Ok(env_value) = env::var("FRAME_EXPORTER_SAMPLING_NUMBER") {
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
        } else if config.sampling_number > 0 {
            debug!(
                "Using sampling number from configuration file: {}",
                config.sampling_number
            );
            Some(config.sampling_number)
        } else {
            debug!("No sampling number provided; defaulting to None.");
            None
        }
    } else if sampling && number.map_or(false, |n| n > 0) {
        debug!("Using single frame sampling with provided number.");
        number
    } else if sampling {
        debug!("Using default single frame sampling of 1.");
        Some(1)
    } else {
        debug!("Sampling is disabled. Defaulting to None.");
        None
    };

    sampling_source
}
