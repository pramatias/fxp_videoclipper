use anyhow::{Context, Result};
use dialoguer::Input;
use log::debug;
use log::warn;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    /// Optional AUDIO path
    pub audio_path: Option<String>,
    /// Frames per second
    pub fps: u32,
    /// Upper limit for pixels
    pub pixel_upper_limit: u32,
    /// Number of frames to sample
    pub sampling_number: usize,
    /// Overall opacity value for merging images (0.0 - 1.0)
    pub opacity: f32,
}

// Manually implement Default to set custom default values
impl Default for Config {
    fn default() -> Self {
        Config {
            audio_path: None,
            fps: 60,                    // Adjust default FPS if needed
            pixel_upper_limit: 480,     // Adjust default pixel limit if needed
            sampling_number: 10,        // Adjust default sample count if needed
            opacity: 0.5,               // Default overall opacity
        }
    }
}

/// Initializes and updates the application configuration by prompting the user for settings.
///
/// This function loads the existing configuration, prompts the user to update various parameters,
/// and saves the updated configuration.
///
/// # Parameters
/// - None
///
/// # Returns
/// - `Result<()>`: Indicates whether the configuration was successfully initialized and saved.
///
/// # Notes
/// - Prompts the user to update the AUDIO path, FPS, pixel upper limit, sampling number,
///   opacity, and multiple opacity values.
/// - Handles user input gracefully, allowing empty values for the AUDIO path and validating
///   numerical inputs where necessary.
/// - Saves the updated configuration to disk upon successful user interaction.
/// - Logs debug information throughout the process.
pub fn initialize_configuration() -> Result<()> {
    debug!("Initializing configuration process started.");

    // Load the configuration using confy
    let mut config: Config =
        confy::load("fxp_videoclipper", "config").context("Failed to load configuration")?;

    // Prompt the user to update the AUDIO path
    let current_audio = config
        .audio_path
        .clone()
        .unwrap_or_else(|| String::from("none"));
    config.audio_path = Input::new()
        .with_prompt(format!(
            "Enter the default AUDIO path (current: {}) (leave empty to skip)",
            current_audio
        ))
        .default(config.audio_path.clone().unwrap_or_default())
        .allow_empty(true)
        .interact()
        .ok();

    // Prompt the user to update FPS
    config.fps = Input::new()
        .with_prompt(format!(
            "Enter the default FPS value (current: {})",
            config.fps
        ))
        .default(config.fps)
        .interact()
        .unwrap_or(config.fps);

    // Prompt the user to update Pixel Upper Limit
    config.pixel_upper_limit = Input::new()
        .with_prompt(format!(
            "Enter the default Pixel Upper Limit (current: {})",
            config.pixel_upper_limit
        ))
        .default(config.pixel_upper_limit)
        .interact()
        .unwrap_or(config.pixel_upper_limit);

    // Prompt the user to update the number of frames to sample
    config.sampling_number = Input::new()
        .with_prompt(format!(
            "Enter the default number of frames to sample (current: {})",
            config.sampling_number
        ))
        .default(config.sampling_number)
        .interact()
        .unwrap_or(config.sampling_number);

    // Prompt the user to update the overall opacity value
    config.opacity = Input::new()
        .with_prompt(format!(
            "Enter the overall opacity value (0.0 - 1.0, current: {})",
            config.opacity
        ))
        .default(config.opacity)
        .interact()
        .unwrap_or(config.opacity);

    debug!("User input received for configuration.");

    // Save the updated configuration using confy
    confy::store("fxp_videoclipper", "config", &config).context("Failed to save configuration")?;

    debug!("Configuration saved successfully.");

    Ok(())
}

/// Loads and provides default configuration settings for the application.
///
/// This function attempts to load existing configuration settings and falls
/// back to default values if none are found.
///
/// # Parameters
///
/// # Returns
/// - `Result<Config>`: The loaded or default configuration settings.
///
/// # Notes
/// - If configuration loading fails, default values will be used.
pub fn load_default_configuration() -> Result<Config> {
    debug!("Default configuration loading using confy...");

    // Attempt to load the configuration using confy
    match confy::load("fxp_videoclipper", "config") {
        Ok(config) => {
            debug!("Configuration successfully loaded.");
            Ok(config)
        }
        Err(err) => {
            warn!(
                "Failed to load configuration: {}. Using default configuration.",
                err
            );
            Ok(Config::default())
        }
    }
}
