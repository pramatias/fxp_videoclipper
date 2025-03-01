use crate::config::Config;
use crate::media_duration::media_duration;
use crate::mp3::get_audio_duration;
use anyhow::{Context, Result};
use log::{debug, warn};

/// Determines the duration of a video or its corresponding MP3 audio.
///
/// This function calculates the duration by checking multiple sources in the following priority:
/// 1. Provided duration argument
/// 2. MP3 audio duration
/// 3. Video duration (fallback)
///
/// # Parameters
/// - `mp3_provided`: Indicates if an MP3 path is available.
/// - `duration_provided`: Indicates if a duration argument is provided.
/// - `video_path`: File path to the video file.
/// - `mp3_path`: Optional path to the corresponding MP3 audio file.
/// - `duration_arg`: Optional manually specified duration.
/// - `config`: Configuration containing necessary settings.
///
/// # Returns
/// - `Result<u64>`: The calculated duration in seconds.
///
/// # Notes
/// - Prioritizes provided duration over MP3 and video durations.
/// - If neither MP3 nor duration is provided, falls back to video duration.
/// - `mp3_path` should be provided when `mp3_provided` is `true`.
/// - `duration_arg` should be provided when `duration_provided` is `true`.
pub fn get_duration(
    mp3_provided: bool,
    duration_provided: bool,
    video_path: &str,
    mp3_path: Option<String>,
    duration_arg: Option<String>,
    config: &Config,
) -> Result<u64> {
    debug!("Getting duration with parameters:");
    debug!("  mp3_provided: {}", mp3_provided);
    debug!("  duration_provided: {}", duration_provided);
    debug!("  video_path: {}", video_path);
    debug!("  mp3_path: {:?}", mp3_path);
    debug!("  duration_arg: {:?}", duration_arg);

    let default_mp3_path = String::new();
    let mp3_path = mp3_path.unwrap_or(default_mp3_path);
    debug!("Using MP3 path: {}", mp3_path);

    let calculated_duration = match (mp3_provided, duration_provided) {
        (true, false) => {
            debug!("MP3 provided, duration not provided. Using MP3 duration.");
            let duration = get_audio_duration(Some(mp3_path.clone()), config)
                .context("Error determining MP3 duration (true, false)")?
                .ok_or_else(|| anyhow::anyhow!("MP3 duration not found"))?;
            debug!("MP3 duration: {:?}", duration);
            duration
        }
        (false, true) => {
            debug!("Duration provided, MP3 not provided. Using provided duration.");
            let duration = duration_arg
                .and_then(|d| d.parse::<u64>().ok())
                .ok_or_else(|| anyhow::anyhow!("Invalid or missing duration argument"))?;
            debug!("Provided duration: {:?}", duration);
            duration
        }
        (true, true) => {
            debug!("Both MP3 path and duration arguments provided. Defaulting to explicitly set duration.");
            let duration = duration_arg
                .and_then(|d| d.parse::<u64>().ok())
                .ok_or_else(|| anyhow::anyhow!("Invalid or missing duration argument"))?;
            debug!("Provided duration: {:?}", duration);

            duration
        }
        (false, false) => {
            debug!("Neither MP3 nor duration provided. Checking for MP3 duration first.");
            if let Some(duration) = get_audio_duration(None, config)
                .context("Error determining MP3 duration (false, false)")?
            {
                debug!("Found MP3 duration: {:?}", duration);
                duration
            } else {
                debug!("MP3 duration not found. Falling back to video duration.");

                // Fallback to video duration
                let duration =
                    media_duration(video_path).context("Error determining video duration")?;
                debug!("Video duration: {:?}", duration);
                duration
            }
        }
    };

    // Validate the calculated duration against the video duration
    validate_duration(calculated_duration, video_path)?;

    Ok(calculated_duration)
}

/// Validates the calculated duration against the video duration.
///
/// This function compares the calculated duration with the actual video duration,
/// issuing warnings if the calculated duration exceeds the video duration.
///
/// # Parameters
/// - `calculated_duration`: The duration to validate (in seconds).
/// - `video_path`: Path to the video file.
///
/// # Returns
/// - `Result<()>`: Indicates success or failure of the validation.
///
/// # Notes
/// - If the calculated duration is longer than the video duration, a warning is logged.
/// - A debug message is logged when the duration is valid.
pub fn validate_duration(calculated_duration: u64, video_path: &str) -> Result<()> {
    // Get the video duration
    let video_duration = media_duration(video_path)
        .context("Error determining video duration in validate_duration")?;

    // Compare the durations
    if video_duration < calculated_duration {
        let difference = calculated_duration - video_duration;
        warn!(
            "Duration ({}) greater than video duration ({}). Difference: {} seconds.",
            calculated_duration, video_duration, difference
        );
    } else {
        debug!(
            "Calculated duration ({}) is within video duration ({}).",
            calculated_duration, video_duration
        );
    }

    Ok(())
}
