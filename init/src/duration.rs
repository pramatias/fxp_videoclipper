use crate::config::Config;
use crate::media_duration::media_duration;
use crate::mp3::get_audio_duration;
use anyhow::{Context, Result};
use log::debug;

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
    video_path: &str,
    mp3_path: Option<String>,
    duration_arg: Option<String>,
    config: &Config,
) -> Result<u64> {
    debug!("Getting duration with parameters:");
    debug!("  video_path: {}", video_path);
    debug!("  mp3_path: {:?}", mp3_path);
    debug!("  duration_arg: {:?}", duration_arg);

    let calculated_duration = match (mp3_path, duration_arg) {
        (Some(mp3), None) => {
            debug!("MP3 provided, duration not provided. Using MP3 duration.");
            let duration = get_audio_duration(Some(mp3.clone()), config)
                .context("Error determining MP3 duration")?
                .ok_or_else(|| anyhow::anyhow!("MP3 duration not found"))?;
            debug!("MP3 duration: {:?}", duration);
            duration
        }
        (None, Some(dur_str)) => {
            debug!("Duration provided, MP3 not provided. Using provided duration.");
            let duration = dur_str
                .parse::<u64>()
                .map_err(|_| anyhow::anyhow!("Invalid duration argument"))?;
            debug!("Provided duration: {:?}", duration);
            duration
        }
        (Some(_), Some(dur_str)) => {
            debug!("Both MP3 path and duration argument provided. Defaulting to explicitly set duration.");
            let duration = dur_str
                .parse::<u64>()
                .map_err(|_| anyhow::anyhow!("Invalid duration argument"))?;
            debug!("Provided duration: {:?}", duration);
            duration
        }
        (None, None) => {
            debug!("Neither MP3 nor duration provided. Checking for MP3 duration first.");
            if let Some(duration) =
                get_audio_duration(None, config).context("Error determining MP3 duration")?
            {
                debug!("Found MP3 duration: {:?}", duration);
                duration
            } else {
                debug!("MP3 duration not found. Falling back to video duration.");
                let duration =
                    media_duration(video_path).context("Error determining video duration")?;
                debug!("Video duration: {:?}", duration);
                duration
            }
        }
    };

    let final_duration = minimum_duration(calculated_duration, video_path)?;
    Ok(final_duration)
}

/// Ensures the calculated duration does not exceed the actual video duration.
///
/// This function compares the calculated duration with the video's actual duration
/// and returns the smaller of the two values.
///
/// # Parameters
/// - `calculated_duration`: The calculated duration to validate
/// - `video_path`: File path to the video for duration measurement
///
/// # Returns
/// - `Result<u64>`: The minimum duration value or an error
///
/// # Notes
/// - The function returns the calculated duration if it's less than or equal to
///   the video duration, otherwise returns the video duration.
pub fn minimum_duration(calculated_duration: u64, video_path: &str) -> Result<u64> {
    let video_duration = media_duration(video_path)
        .context("Error determining video duration in minimum_duration")?;

    if calculated_duration > video_duration {
        debug!(
            "Calculated duration ({}) is greater than video duration ({}). Using video duration.",
            calculated_duration, video_duration
        );
        Ok(video_duration)
    } else {
        debug!(
            "Calculated duration ({}) is within video duration ({}).",
            calculated_duration, video_duration
        );
        Ok(calculated_duration)
    }
}
