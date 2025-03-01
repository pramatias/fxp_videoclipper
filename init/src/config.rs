use anyhow::{Context, Result};
use dialoguer::Input;
use log::debug;
use log::warn;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    /// Optional MP3 path
    pub mp3_path: Option<String>,
    /// Frames per second
    pub fps: u32,
    /// Upper limit for pixels
    pub pixel_upper_limit: u32,
    /// Number of frames to sample
    pub sampling_number: usize,
    /// Overall opacity value for merging images (0.0 - 1.0)
    pub opacity: f32,
    /// Additional opacity value 1 (0.0 - 1.0)
    pub multiple_opacities_1: f32,
    /// Additional opacity value 2 (0.0 - 1.0)
    pub multiple_opacities_2: f32,
    /// Additional opacity value 3 (0.0 - 1.0)
    pub multiple_opacities_3: f32,
}

// Manually implement Default to set custom default values
impl Default for Config {
    fn default() -> Self {
        Config {
            mp3_path: None,
            fps: 30,                    // Adjust default FPS if needed
            pixel_upper_limit: 480,     // Adjust default pixel limit if needed
            sampling_number: 10,        // Adjust default sample count if needed
            opacity: 0.5,               // Default overall opacity
            multiple_opacities_1: 0.25, // Default additional opacity 1
            multiple_opacities_2: 0.5,  // Default additional opacity 2
            multiple_opacities_3: 0.75, // Default additional opacity 3
        }
    }
}

/// Initializes and updates the application configuration by interacting with the user.
///
/// This function loads existing configuration settings, prompts the user to update each setting,
/// including the overall opacity and additional opacity fields, and then saves the updated configuration.
pub fn initialize_configuration() -> Result<()> {
    debug!("Initializing configuration process started.");

    // Load the configuration using confy
    let mut config: Config =
        confy::load("fxp_videoclipper", "config").context("Failed to load configuration")?;

    // Prompt the user to update the MP3 path
    let current_mp3 = config
        .mp3_path
        .clone()
        .unwrap_or_else(|| String::from("none"));
    config.mp3_path = Input::new()
        .with_prompt(format!(
            "Enter the default MP3 path (current: {}) (leave empty to skip)",
            current_mp3
        ))
        .default(config.mp3_path.clone().unwrap_or_default())
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

    // Prompt the user to update multiple_opacities_1
    config.multiple_opacities_1 = Input::new()
        .with_prompt(format!(
            "Enter multiple_opacities_1 value (0.0 - 1.0, current: {})",
            config.multiple_opacities_1
        ))
        .default(config.multiple_opacities_1)
        .interact()
        .unwrap_or(config.multiple_opacities_1);

    // Prompt the user to update multiple_opacities_2
    config.multiple_opacities_2 = Input::new()
        .with_prompt(format!(
            "Enter multiple_opacities_2 value (0.0 - 1.0, current: {})",
            config.multiple_opacities_2
        ))
        .default(config.multiple_opacities_2)
        .interact()
        .unwrap_or(config.multiple_opacities_2);

    // Prompt the user to update multiple_opacities_3
    config.multiple_opacities_3 = Input::new()
        .with_prompt(format!(
            "Enter multiple_opacities_3 value (0.0 - 1.0, current: {})",
            config.multiple_opacities_3
        ))
        .default(config.multiple_opacities_3)
        .interact()
        .unwrap_or(config.multiple_opacities_3);

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
