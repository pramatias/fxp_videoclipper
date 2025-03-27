use anyhow::{anyhow, Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use log::{debug, error};
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command as ShellCommand;
use std::process::Stdio;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::time::Duration;

/// Extracts a single frame from the middle of a video.
///
/// This function captures a frame at the midpoint of the video's duration.
/// If the process is interrupted, it returns an error.
///
/// # Parameters
/// - `video`: Path to the video file.
/// - `duration_ms`: Video duration in milliseconds.
/// - `output_path`: Destination path for the extracted frame.
/// - `running`: Flag indicating whether the operation should continue.
///
/// # Returns
/// - `Result<()>`: Success or failure of the frame extraction.
///
/// # Notes
/// - The function is interruptible and checks the `running` flag at multiple stages.
/// - If `output_path` is a file, the function creates a temporary file and renames it afterward.
/// - Supports both file and directory output paths, formatting filenames appropriately.
pub fn extract_single_frame<P: AsRef<Path>>(
    video: P,
    duration_ms: u64,
    output_path: PathBuf,
    running: Arc<AtomicBool>,
) -> Result<()> {
    // Initialize the progress bar with a total of 1 step (since only one frame is being extracted)
    let pb = ProgressBar::new(1);
    let style = ProgressStyle::default_bar()
        .template(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}",
        )
        .context("Failed to set progress bar template")?;
    pb.set_style(style);

    debug!("Starting to extract a single frame from the middle of the video...");

    if !running.load(Ordering::SeqCst) {
        pb.finish_and_clear();
        return Err(anyhow!("Extraction interrupted before starting."));
    }

    if duration_ms == 0 {
        pb.finish_and_clear();
        return Err(anyhow!("Failed to determine video length."));
    }

    let middle_timestamp_ms = duration_ms / 2;
    debug!("Calculated middle timestamp (ms): {}", middle_timestamp_ms);

    if !running.load(Ordering::SeqCst) {
        pb.finish_and_clear();
        return Err(anyhow!(
            "Extraction interrupted before extracting the frame."
        ));
    }

    let middle_timestamp_seconds = middle_timestamp_ms as f64 / 1000.0;

    let output_is_file = output_path.is_file();
    let temp_output_path = if output_is_file {
        let parent = output_path.parent().unwrap_or_else(|| Path::new("."));
        let filename = output_path
            .file_name()
            .ok_or_else(|| anyhow!("Invalid file name"))?;
        let filename_str = filename.to_string_lossy();
        let formatted_filename = if filename_str.to_lowercase().ends_with(".png") {
            // Remove the ".png" (4 characters) and insert %01d before it.
            let base = &filename_str[..filename_str.len() - 4];
            format!("{}%01d.png", base)
        } else {
            // Otherwise, just append %01d.png
            format!("{}%01d.png", filename_str)
        };
        parent.join(formatted_filename)
    } else {
        output_path.join("sample_frame%01d.png")
    };

    let video_str = video
        .as_ref()
        .to_str()
        .ok_or_else(|| anyhow!("Invalid video file path"))?;
    let temp_output_str = temp_output_path
        .to_str()
        .ok_or_else(|| anyhow!("Invalid output file path"))?;

    // Set a progress message and perform the frame extraction
    extract_frame(
        video_str,
        middle_timestamp_seconds,
        temp_output_str,
        running.clone(),
    )
    .with_context(|| {
        format!(
            "Failed to extract frame at {:.3} seconds from the video.",
            middle_timestamp_seconds
        )
    })?;

    // Mark progress complete
    pb.inc(1);

    if output_is_file {
        let extracted_file = temp_output_path.with_file_name(format!(
            "{}1.png",
            output_path.file_stem().unwrap().to_string_lossy()
        ));
        std::fs::rename(&extracted_file, &output_path).with_context(|| {
            format!(
                "Failed to rename {} to {}",
                extracted_file.display(),
                output_path.display()
            )
        })?;
    }

    if running.load(Ordering::SeqCst) {
        debug!(
            "Successfully extracted frame at {:.3} seconds as {:?}",
            middle_timestamp_seconds, output_path
        );
    } else {
        pb.finish_and_clear();
        return Err(anyhow!("Extraction was interrupted midway."));
    }

    pb.finish();
    Ok(())
}

/// Extracts multiple frames from a video at specified intervals.
///
/// This function captures a series of frames from a video file and saves them as images.
/// The frames are extracted at evenly spaced intervals throughout the video's duration.
///
/// # Parameters
/// - `video`: Path to the video file to extract frames from.
/// - `duration_ms`: Total duration of the video in milliseconds.
/// - `num_frames`: Number of frames to extract from the video.
/// - `output_dir`: Directory path where the extracted frames will be saved.
/// - `running`: Flag indicating whether the extraction process should continue.
///
/// # Returns
/// - `Result<()>`: Returns `Ok(())` upon success. If an error occurs, returns an
///   `Err` containing a descriptive error message.
///
/// # Notes
/// - Frames are extracted at intervals calculated by dividing the video duration
///   into `(num_frames + 1)` equal parts.
/// - The output directory will be created if it does not already exist.
/// - The process can be interrupted by setting the `running` flag to false.
pub fn extract_multiple_frames(
    video: &Path,
    duration_ms: u64,
    num_frames: usize,
    output_dir: &Path,
    running: Arc<AtomicBool>,
) -> Result<()> {
    log::debug!("Starting to extract multiple frames from the video...");

    // Ensure the output directory exists, create it if necessary.
    if !output_dir.exists() {
        fs::create_dir_all(output_dir)
            .with_context(|| format!("Failed to create output directory: {:?}", output_dir))?;
    }

    if !running.load(Ordering::SeqCst) {
        return Err(anyhow!("Extraction interrupted before starting."));
    }

    if duration_ms == 0 {
        return Err(anyhow!("Failed to determine video length."));
    }

    // Calculate frame interval by dividing the duration into (num_frames + 1) parts.
    let frame_interval_ms = duration_ms / (num_frames as u64 + 1);

    // Convert the video path to a &str for extract_frame.
    let video_str = video
        .to_str()
        .ok_or_else(|| anyhow!("Invalid video path"))?;

    // Set up a progress bar for the total number of frames.
    let pb = ProgressBar::new(num_frames as u64);
    let style = ProgressStyle::default_bar()
        .template(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}",
        )
        .context("Failed to set progress bar template")?;
    pb.set_style(style);

    for i in 0..num_frames {
        if !running.load(Ordering::SeqCst) {
            pb.finish_and_clear();
            return Err(anyhow!("Extraction interrupted during frame extraction."));
        }

        // Calculate timestamp for each frame.
        let timestamp_ms = frame_interval_ms * (i as u64 + 1);
        debug!("Extracting frame {} at {} ms", i + 1, timestamp_ms);
        // pb.set_message(format!("Extracting frame {} at {} ms", i + 1, timestamp_ms));

        // Build output file path by joining directory with a generated filename.
        let output_file_path = output_dir.join(format!("sample_frame_{}.png", i + 1));
        debug!("Output file set to: {:?}", output_file_path);

        // Convert timestamp to seconds.
        let timestamp_seconds = timestamp_ms as f64 / 1000.0;

        // Call the frame extraction function.
        extract_frame(
            video_str,
            timestamp_seconds,
            output_file_path
                .to_str()
                .ok_or_else(|| anyhow!("Invalid output file path"))?,
            running.clone(),
        )
        .with_context(|| {
            format!(
                "Failed to extract frame at {:.3} seconds from the video.",
                timestamp_seconds
            )
        })?;

        // Update the progress bar.
        pb.inc(1);
    }

    pb.finish();

    if running.load(Ordering::SeqCst) {
        debug!("Successfully extracted {} frames.", num_frames);
    } else {
        return Err(anyhow!("Extraction was interrupted midway."));
    }

    Ok(())
}

/// Extracts a single frame from a video at the specified timestamp.
///
/// This function uses FFmpeg to capture a frame at a given time and saves it as an image file.
///
/// # Parameters
/// - `video`: Path to the input video file.
/// - `timestamp_seconds`: Time in seconds (with millisecond precision) to extract the frame.
/// - `output`: Path where the extracted frame image will be saved.
/// - `running`: A flag to control the extraction process, allowing it to be interrupted.
///
/// # Returns
/// - `Result<()>`: Returns `Ok(())` on successful frame extraction, or an error if extraction fails.
///
/// # Notes
/// - The function uses FFmpeg under the hood for frame extraction.
/// - If the `running` flag becomes false, the process will be interrupted.
/// - The extraction process can be interrupted by setting the `running` flag to false.
fn extract_frame(
    video: &str,
    timestamp_seconds: f64,
    output: &str,
    running: Arc<AtomicBool>,
) -> Result<()> {
    debug!(
        "Attempting to extract frame at {:.3} seconds from video '{}' to '{}'",
        timestamp_seconds, video, output
    );

    // Construct the ffmpeg command as a string for debugging purposes
    let ffmpeg_command = format!(
        "ffmpeg -i {} -ss {:.3} -frames:v 1 {} -y",
        video, timestamp_seconds, output
    );
    // Log the final ffmpeg command
    debug!("Final ffmpeg command: {}", ffmpeg_command);

    // Spawn a child process for ffmpeg with the working directory set to output_dir.
    let mut child = ShellCommand::new("ffmpeg")
        .arg("-i")
        .arg(video)
        .arg("-ss")
        .arg(format!("{:.3}", timestamp_seconds)) // Timestamp with millisecond precision
        .arg("-frames:v")
        .arg("1") // Extract a single frame
        .arg(output) // Pass only the file name now
        .arg("-y") // Pass only the file name now
        .stdout(Stdio::null()) // Suppress stdout
        .stderr(Stdio::null()) // Suppress stderr
        .spawn()
        .with_context(|| {
            format!(
                "Failed to start ffmpeg process for frame extraction at {:.3} seconds",
                timestamp_seconds
            )
        })?;

    debug!("FFmpeg process spawned with PID: {:?}", child.id());

    // Periodically check the `running` flag.
    while running.load(Ordering::SeqCst) {
        if let Ok(Some(status)) = child.try_wait() {
            // Process finished, check its status.
            if status.success() {
                debug!("Frame extracted successfully to {}", output);
                return Ok(());
            } else {
                return Err(anyhow!("FFmpeg command failed with status: {}", status));
            }
        }

        thread::sleep(Duration::from_millis(100));
    }

    // If we exit the loop, it means `running` is false, so terminate ffmpeg.
    error!("Interrupt signal received, terminating FFmpeg process...");
    if let Err(e) = child.kill() {
        return Err(anyhow!("Failed to kill FFmpeg process: {}", e));
    }
    if let Err(e) = child.wait() {
        return Err(anyhow!(
            "Failed to wait for FFmpeg process to terminate: {}",
            e
        ));
    }

    Err(anyhow!("Extraction interrupted before completion"))
}
