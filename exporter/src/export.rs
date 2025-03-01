use indicatif::{ProgressBar, ProgressStyle};
use log::debug;
// use std::fs;
use std::path::PathBuf;
use std::process::Command as StdCommand;

use anyhow::{Context, Result};
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;

/// Extracts multiple frames from a video at evenly spaced intervals.
///
/// This function extracts a specified number of frames from a video,
/// distributing them evenly over the video's duration.
///
/// # Parameters
/// - `video`: Path to the video file
/// - `duration_ms`: Total duration of the video in milliseconds
/// - `num_frames`: Number of frames to extract
/// - `running`: Flag to check if the operation should continue
///
/// # Returns
/// - `Result<()>`: Indicates success or failure of the operation
///
/// # Notes
/// - The video duration must be greater than 2 seconds for frame extraction
/// - Frames are extracted at uniform intervals between 1 second after start and 1 second before end
/// - The `running` flag can be used to cancel the operation prematurely
pub fn extract_all_frames_with_progress(
    video: &str,
    output_dir: PathBuf,
    duration: f64,
    fps: u32,
    running: Arc<AtomicBool>,
) -> anyhow::Result<()> {
    let total_frames = (duration * fps as f64) as u64;
    log::debug!("Total frames to extract: {}", total_frames);

    let pb = ProgressBar::new(total_frames);
    let style = ProgressStyle::default_bar()
        .template(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}",
        )
        .context("Failed to set progress bar template")?;
    pb.set_style(style);

    for i in 0..total_frames {
        if !running.load(Ordering::SeqCst) {
            pb.finish_with_message("");
            log::debug!("Frame extraction interrupted by user.");
            return Err(anyhow::anyhow!("Frame extraction interrupted by user."));
        }

        let output_file = output_dir.join(format!("image_{:04}.png", i + 1));

        StdCommand::new("ffmpeg")
            .args(&[
                "-y",
                "-i",
                video,
                "-vf",
                &format!("select=eq(n\\,{})", i),
                "-fps_mode",
                "vfr",
                output_file
                    .to_str()
                    .expect("Output file path contains invalid UTF-8"),
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .output()
            .with_context(|| {
                format!(
                    "Failed to execute ffmpeg for frame extraction at frame {}",
                    i + 1
                )
            })?;

        pb.inc(1);
    }

    pb.finish_with_message(" -> Frame extraction complete!");
    log::debug!("Frame extraction completed!");
    Ok(())
}

/// Processes a video by cutting a section based on specified duration and pixel limits.
///
/// This function handles video processing with options for duration and pixel quality.
///
/// # Parameters
/// - `video_path`: Path to the video file to process.
/// - `duration`: Desired duration in milliseconds.
/// - `pixel_upper_limit`: Maximum allowed pixel value for quality control.
/// - `running`: Atomic boolean to track if the operation should continue.
///
/// # Returns
/// - `Result<(String, f64)>`: A tuple containing the path to the processed video and the duration in seconds. Returns an error if processing fails.
///
/// # Notes
/// - Duration is converted from milliseconds to seconds for processing.
/// - The pixel upper limit ensures video quality remains within specified bounds.
pub fn cut_video_section(
    video_path: &str,
    duration: u64,
    pixel_upper_limit: u32,
    running: Arc<AtomicBool>,
) -> Result<(String, f64)> {
    debug!("Processing video cut for: {}", video_path);
    debug!("Requested duration (milliseconds): {} ms", duration);

    // Convert duration from milliseconds to seconds
    let duration_in_seconds = (duration as f64) / 1000.0;
    debug!(
        "Converted duration to seconds: {:.2} seconds",
        duration_in_seconds
    );

    let cut_duration = duration_in_seconds;
    debug!("Calculated cut duration: {:.2} seconds", cut_duration);

    // Attempt to cut and process the video with the given pixel_upper_limit
    debug!("Attempting to cut the video...");
    let cut_video_path = cut_video(video_path, cut_duration, pixel_upper_limit, running.clone())
        .context("Failed to cut video")?;

    debug!("Video successfully cut to {:.2} seconds", cut_duration);

    Ok((cut_video_path, cut_duration))
}

/// Processes a video by cutting, resizing, and adjusting framerate.
///
/// This function performs three main operations on a video file:
/// 1. Cuts the video to a specified duration
/// 2. Resizes the video based on pixel limit
/// 3. Adjusts the framerate to 31fps
///
/// # Parameters
/// - `video_path`: Path to the input video file
/// - `duration`: Desired duration of the output video in seconds
/// - `pixel_upper_limit`: Maximum pixel size for resizing
/// - `running`: Atomic flag to track if the process should continue
///
/// # Returns
/// - `Result<String>`: Path to the processed video file on success
///
/// # Notes
/// - Creates temporary files during processing
/// - Deletes temporary files unless in debug mode
fn cut_video(
    video_path: &str,
    duration: f64,
    pixel_upper_limit: u32,
    running: Arc<AtomicBool>,
) -> Result<String> {
    let temp_cut_path = "/tmp/video_cut.mp4"; // Temporary file for cut video
    let temp_resized_path = "/tmp/video_resized.mp4"; // Temporary file for resized video
    let output_path = format!("/tmp/{}.mp4", video_path.trim_end_matches(".mp4")); // Final output file

    debug!("Starting video processing for: {}", video_path);
    debug!("Temporary cut path: {}", temp_cut_path);
    debug!("Temporary resized path: {}", temp_resized_path);
    debug!("Output path: {}", output_path);

    // Step 1: Cut the video by calling the extracted function
    cut_video_to_duration(video_path, temp_cut_path, duration, running.clone())?;

    // Check if the process is still running
    if !running.load(Ordering::SeqCst) {
        anyhow::bail!("Process interrupted by user");
    }

    // Step 2: Resize the video by calling the extracted function
    resize_video(
        temp_cut_path,
        temp_resized_path,
        pixel_upper_limit,
        running.clone(),
    )?;

    // Step 3: Change the framerate to 31fps and save the final output
    adjust_framerate(&temp_resized_path, &output_path, 31, running.clone())?;

    // Step 4: Conditionally remove the temporary files
    if cfg!(not(debug_assertions)) {
        debug!("Cleaning up temporary files");
        std::fs::remove_file(temp_cut_path).context("Failed to remove temporary cut file")?;
        std::fs::remove_file(temp_resized_path)
            .context("Failed to remove temporary resized file")?;
    } else {
        debug!("Debug mode enabled, skipping cleanup of temporary files");
    }

    debug!("Video processing completed successfully");
    Ok(output_path)
}

/// Retrieves the dimensions (width and height) of a video using ffprobe.
///
/// This function executes an ffprobe command to extract video stream information
/// and parses the output to obtain width and height.
///
/// # Parameters
/// - `input_path`: The path to the video file.
/// - `running`: A flag indicating whether the process should continue.
///
/// # Returns
/// - `Result<(u32, u32)>`: The video dimensions as a tuple (width, height).
/// - Returns an error if the process is interrupted or if ffprobe execution fails.
///
/// # Notes
/// - The function uses `ffprobe` to fetch video stream details.
/// - Dimensions are parsed as unsigned 32-bit integers.
/// - Returns an error if dimension parsing fails.
fn get_video_dimensions(input_path: &str, running: Arc<AtomicBool>) -> Result<(u32, u32)> {
    if !running.load(Ordering::SeqCst) {
        anyhow::bail!("Process interrupted by user");
    }

    debug!("Fetching video dimensions for input: {}", input_path);

    // Execute ffprobe to get video dimensions
    debug!("Executing ffprobe command to retrieve video dimensions...");
    let output = StdCommand::new("ffprobe")
        .args(&[
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height",
            "-of",
            "csv=s=x:p=0",
            input_path,
        ])
        .output()
        .context("Failed to execute ffprobe to get video dimensions")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        debug!("FFprobe command failed with error: {}", stderr);
        anyhow::bail!("Failed to get video dimensions: {}", stderr);
    }

    // Parse the output to extract dimensions
    let dimensions = String::from_utf8_lossy(&output.stdout);
    debug!("Raw dimensions output from ffprobe: {}", dimensions.trim());

    let dimensions: Vec<&str> = dimensions.trim().split('x').collect();
    if dimensions.len() != 2 {
        debug!("Unexpected dimensions format: {}", dimensions.join("x"));
        anyhow::bail!("Unexpected dimensions format: {}", dimensions.join("x"));
    }

    // Parse width and height
    let width = dimensions[0]
        .parse::<u32>()
        .context("Failed to parse video width")?;
    let height = dimensions[1]
        .parse::<u32>()
        .context("Failed to parse video height")?;

    debug!("Parsed video dimensions: {}x{}", width, height);
    Ok((width, height))
}

/// Resizes a video while maintaining its aspect ratio, with a maximum pixel limit.
///
/// This function resizes a video file using FFmpeg, ensuring the new dimensions do not exceed a specified total number of pixels.
///
/// # Parameters
/// - =input_path=: Path to the input video file.
/// - =output_path=: Path where the resized video will be saved.
/// - =pixel_upper_limit=: Maximum total pixels allowed in the resized video.
/// - =running=: Flag to check if the process should continue.
///
/// # Returns
/// - =Result<()>=: Indicates success or failure of the resizing operation.
///
/// # Notes
/// - The aspect ratio of the original video is preserved.
/// - The =pixel_upper_limit= specifies the maximum number of pixels allowed in the resized video (width × height).
/// - If =running= is set to =false=, the process will be interrupted.
fn resize_video(
    input_path: &str,
    output_path: &str,
    pixel_upper_limit: u32,
    running: Arc<AtomicBool>,
) -> Result<()> {
    if !running.load(Ordering::SeqCst) {
        anyhow::bail!("Process interrupted by user");
    }

    debug!("Starting video resizing process for input: {}", input_path);

    let (width, height) = get_video_dimensions(input_path, running.clone())?;
    debug!("Original video dimensions: {}x{}", width, height);

    let (new_width, new_height) =
        calculate_aspect_ratio_dimensions(width, height, pixel_upper_limit);
    debug!("Calculated new dimensions: {}x{}", new_width, new_height);

    let vf_arg = format!("scale={}:{}", new_width, new_height);
    debug!("Using video filter argument: {}", vf_arg);

    debug!("Executing ffmpeg command to resize video...");
    let output = StdCommand::new("ffmpeg")
        .args(&["-y", "-i", input_path, "-vf", &vf_arg, output_path])
        .stderr(std::process::Stdio::null())
        .output()
        .context("Failed to execute ffmpeg for resizing video")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        debug!("FFmpeg command failed with error: {}", stderr);
        anyhow::bail!("Failed to resize video: {}", stderr);
    }

    debug!(
        "Video resizing completed successfully. Output saved to: {}",
        output_path
    );
    Ok(())
}

/// Cuts a video to the specified duration using FFmpeg.
///
/// This function trims a video file to the specified duration in seconds. It utilizes FFmpeg for the video processing.
///
/// # Parameters
/// - `input_path`: Path to the input video file
/// - `output_path`: Path where the trimmed video will be saved
/// - `duration`: Desired duration of the output video in seconds
/// - `running`: Flag to check if the process should continue running
///
/// # Returns
/// - `Result<()>`: Indicates success or failure of the video cutting operation
///
/// # Notes
/// - The function will stop execution if `running` flag becomes false
/// - Requires FFmpeg to be installed and available in system PATH
/// - Any existing file at `output_path` will be overwritten
fn cut_video_to_duration(
    input_path: &str,
    output_path: &str,
    duration: f64,
    running: Arc<AtomicBool>,
) -> Result<()> {
    debug!("Cutting video to {} seconds", duration);

    // Check if the process is still running
    if !running.load(Ordering::SeqCst) {
        anyhow::bail!("Process interrupted by user");
    }

    StdCommand::new("ffmpeg")
        .args(&[
            "-y", // Automatically overwrite existing files
            "-i",
            input_path,
            "-t",
            &duration.to_string(),
            "-c",
            "copy",
            output_path,
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .context("Failed to execute ffmpeg for cutting video")?
        .success()
        .then_some(())
        .context("Failed to cut video")?;

    debug!("Temporary cut video created at {}", output_path);
    Ok(())
}

/// Adjusts the framerate of a video using ffmpeg.
///
/// This function modifies the video's framerate to the specified value and saves the result.
///
/// # Parameters
/// - `input_path`: The path to the input video file.
/// - `output_path`: The path where the output video will be saved.
/// - `framerate`: The target frames per second.
/// - `running`: A flag to check if the process should continue running.
///
/// # Returns
/// - `Result<()>`: Returns `Ok(())` on success or an error if something goes wrong.
///
/// # Notes
/// - The function uses ffmpeg under the hood to adjust the framerate.
/// - The audio stream is copied without re-encoding.
/// - If the `running` flag is set to false, the process will be interrupted.
fn adjust_framerate(
    input_path: &str,
    output_path: &str,
    framerate: u32,
    running: Arc<AtomicBool>,
) -> Result<()> {
    debug!(
        "Adjusting framerate of video at {} to {}fps, saving to {}",
        input_path, framerate, output_path
    );

    // Check if the process is still running
    if !running.load(Ordering::SeqCst) {
        debug!("Process interrupted by user. Exiting framerate adjustment.");
        anyhow::bail!("Process interrupted by user");
    }

    debug!("Executing ffmpeg command to adjust framerate...");
    let status = StdCommand::new("ffmpeg")
        .args(&[
            "-y", // Automatically overwrite existing files
            "-i",
            input_path,
            "-filter:v",
            &format!("fps=fps={}", framerate),
            "-c:a",
            "copy", // Copy audio without re-encoding
            output_path,
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .context("Failed to execute ffmpeg for changing framerate")?;

    if !status.success() {
        debug!("FFmpeg command failed to adjust framerate.");
        anyhow::bail!("Failed to change framerate");
    }

    debug!(
        "Framerate adjustment completed successfully: {}",
        output_path
    );
    Ok(())
}

/// Ensures the pixel limit is an even number, rounding up if necessary.
///
/// This function adjusts the provided pixel limit to the nearest even number if it is odd.
///
/// # Parameters
/// - `pixel_limit`: The input pixel limit to be adjusted.
///
/// # Returns
/// - `u32`: The even pixel limit.
fn ensure_even(pixel_limit: u32) -> u32 {
    if pixel_limit % 2 != 0 {
        pixel_limit + 1 // Round up to the nearest even number
    } else {
        pixel_limit
    }
}

/// Calculates new image dimensions while maintaining aspect ratio within a pixel limit.
///
/// This function computes scaled dimensions for an image, ensuring the larger dimension does not exceed the specified pixel limit.
/// The aspect ratio of the original dimensions is preserved.
///
/// # Parameters
/// - `width`: Original width of the image
/// - `height`: Original height of the image
/// - `pixel_upper_limit`: Maximum allowed value for the larger dimension after scaling
///
/// # Returns
/// - `(u32, u32)`: A tuple containing the scaled width and height, both as even numbers
///
/// # Notes
/// - Maintains the original aspect ratio while scaling
/// - Ensures both dimensions are even numbers
fn calculate_aspect_ratio_dimensions(
    width: u32,
    height: u32,
    pixel_upper_limit: u32,
) -> (u32, u32) {
    debug!(
        "Calculating new dimensions for original size: {}x{} with pixel upper limit: {}",
        width, height, pixel_upper_limit
    );

    let original_aspect_ratio = width as f64 / height as f64;
    debug!("Original aspect ratio: {:.2}", original_aspect_ratio);

    let new_width;
    let new_height;

    if width >= height {
        debug!("Width is greater than or equal to height. Scaling based on width.");
        new_width = pixel_upper_limit;
        new_height = (pixel_upper_limit as f64 / original_aspect_ratio).round() as u32;
        debug!("Calculated new height: {}", new_height);
    } else {
        debug!("Height is greater than width. Scaling based on height.");
        new_height = pixel_upper_limit;
        new_width = (pixel_upper_limit as f64 * original_aspect_ratio).round() as u32;
        debug!("Calculated new width: {}", new_width);
    }

    let final_width = ensure_even(new_width);
    let final_height = ensure_even(new_height);
    debug!(
        "Final dimensions after ensuring even values: {}x{}",
        final_width, final_height
    );

    (final_width, final_height)
}
