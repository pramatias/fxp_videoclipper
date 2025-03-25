use anyhow::{anyhow, bail, Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use log::debug;
use std::fs;
use std::path::PathBuf;
use std::process::Command as StdCommand;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Extracts all frames from a video file with progress indication.
///
/// This function extracts frames from a video at specified intervals and displays a progress bar.
/// It can be interrupted, stopping the extraction process.
///
/// # Parameters
/// - `video`: Input video file path.
/// - `output_dir`: Directory to save the extracted frames.
/// - `duration`: Video duration in seconds.
/// - `fps`: Frames per second to determine the number of frames.
/// - `running`: Flag to control the extraction process continuation.
///
/// # Returns
/// - `Result<()>`: Indicates if the extraction completed successfully or encountered an error.
///
/// # Notes
/// - The extracted frames are named in the format `frame_0001.png`, `frame_0002.png`, etc.
/// - If the process is interrupted, returns an error message.
pub fn extract_all_frames_with_progress(
    video: &str,
    output_dir: PathBuf,
    duration: f64,
    fps: u32,
    running: Arc<AtomicBool>,
) -> Result<()> {
    let total_frames = (duration * fps as f64) as u64;
    debug!("Total frames to extract: {}", total_frames);

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
            debug!("Frame extraction interrupted by user.");
            return Err(anyhow!("Frame extraction interrupted by user."));
        }

        let output_file = output_dir.join(format!("frame_{:04}.png", i + 1));

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
    debug!("Frame extraction completed!");
    Ok(())
}

/// Processes a video by cutting it to a specified duration, adjusting FPS,
/// and resizing based on a pixel limit.
///
/// This function handles video processing in multiple steps:
/// - Cuts the video to the specified duration
/// - Adjusts the FPS (frames per second)
/// - Resizes the video based on pixel upper limit
/// - Uses a temporary directory for processing
/// - Monitors if the process is still running
///
/// # Parameters
/// - `video_path`: Path to the input video file
/// - `duration`: Desired duration of the output video in milliseconds
/// - `pixel_upper_limit`: Maximum allowed pixels for resizing
/// - `fps`: Target frames per second for the output video
/// - `tmp_dir_path`: Temporary directory for processing files
/// - `running`: Flag to check if processing should continue
///
/// # Returns
/// - `Result<(String, f64)>`: Tuple containing:
///   - Path to the processed video file
///   - Duration of the output video in seconds
///
/// # Notes
/// - The function uses FFmpeg under the hood for video processing
/// - Temporary files are stored in the specified temporary directory
/// - Processing stops if `running` is set to false
/// - Returns an error if video cutting or resizing fails
/// - If the requested duration is longer than the source video, it returns the original video
pub fn cut_duration_adjust_fps_resize(
    video_path: &str,
    duration: u64,
    pixel_upper_limit: u32,
    fps: u32,
    tmp_dir_path: PathBuf,
    running: Arc<AtomicBool>,
) -> Result<(String, f64)> {
    debug!("Processing video cut for: {}", video_path);
    debug!("Requested duration (milliseconds): {} ms", duration);

    // Convert duration from milliseconds to seconds.
    let duration_in_seconds = (duration as f64) / 1000.0;
    debug!(
        "Converted duration to seconds: {:.2} seconds",
        duration_in_seconds
    );

    let cut_duration = duration_in_seconds;
    debug!("Calculated cut duration: {:.2} seconds", cut_duration);

    // Attempt to cut and process the video with the given pixel_upper_limit.
    debug!("Attempting to cut the video...");
    let cut_video_path = cut_video(
        video_path,
        cut_duration,
        pixel_upper_limit,
        fps,
        tmp_dir_path,
        running.clone(),
    )
    .context("Failed to cut video")?;

    debug!("Video successfully cut to {:.2} seconds", cut_duration);

    Ok((cut_video_path, cut_duration))
}

/// Processes a video by cutting, resizing, and adjusting framerate.
///
/// This function handles video processing in three main steps:
/// 1. Cuts the video to the specified duration
/// 2. Resizes the video based on pixel limit
/// 3. Adjusts the video framerate
///
/// # Parameters
/// - `video_path`: Path to the input video file
/// - `duration`: Desired duration of the output video
/// - `pixel_upper_limit`: Maximum allowed pixel size for resizing
/// - `fps`: Frames per second for the output video
/// - `tmp_dir_path`: Temporary directory for processing files
/// - `running`: Atomic boolean to track if process should continue
///
/// # Returns
/// - `Result<String>`: Path to the processed video file or error
fn cut_video(
    video_path: &str,
    duration: f64,
    pixel_upper_limit: u32,
    fps: u32, // new fps parameter added here
    tmp_dir_path: PathBuf,
    running: Arc<AtomicBool>,
) -> Result<String> {
    // Create the temporary directory if it doesn't exist.
    fs::create_dir_all(&tmp_dir_path).context("Failed to create temporary directory")?;

    let temp_cut_path = tmp_dir_path.join("video_cut.mp4");
    let temp_resized_path = tmp_dir_path.join("video_resized.mp4");

    let output_path_buf = tmp_dir_path.join(format!("{}.mp4", video_path.trim_end_matches(".mp4")));

    // Convert the output path (PathBuf) to a String.
    let output_path = output_path_buf
        .to_str()
        .expect("Output path contains invalid UTF-8")
        .to_string();

    debug!("Starting video processing for: {}", video_path);
    debug!("Temporary cut path: {:?}", temp_cut_path);
    debug!("Temporary resized path: {:?}", temp_resized_path);
    debug!("Output path: {}", output_path);

    // Step 1: Cut the video to the desired duration.
    cut_video_to_duration(
        video_path,
        temp_cut_path
            .to_str()
            .expect("Temporary cut path contains invalid UTF-8"),
        duration,
        running.clone(),
    )?;

    // Check if the process is still running.
    if !running.load(Ordering::SeqCst) {
        bail!("Process interrupted by user");
    }

    // Step 2: Resize the video.
    resize_video(
        temp_cut_path
            .to_str()
            .expect("Temporary cut path contains invalid UTF-8"),
        temp_resized_path
            .to_str()
            .expect("Temporary resized path contains invalid UTF-8"),
        pixel_upper_limit,
        running.clone(),
    )?;

    // Step 3: Adjust the framerate using the provided fps value.
    adjust_framerate(
        temp_resized_path
            .to_str()
            .expect("Temporary resized path contains invalid UTF-8"),
        &output_path,
        fps,
        running.clone(),
    )?;

    debug!("Video processing completed successfully");
    Ok(output_path)
}

/// Fetches the dimensions (width and height) of a video file using ffprobe.
///
/// This function executes an ffprobe command to extract video stream information
/// and parse the dimensions from the output.
///
/// # Parameters
/// - `input_path`: Path to the video file as a string.
/// - `running`: A flag to check if the process should continue running.
///
/// # Returns
/// - `Result<(u32, u32)>`: A tuple containing the video width and height in pixels.
///                            Returns an error if dimensions cannot be parsed.
///
/// # Notes
/// - The function will bail if the process has been interrupted by the user.
/// - Relies on ffprobe being available in the system PATH.
fn get_video_dimensions(input_path: &str, running: Arc<AtomicBool>) -> Result<(u32, u32)> {
    if !running.load(Ordering::SeqCst) {
        bail!("Process interrupted by user");
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
        bail!("Failed to get video dimensions: {}", stderr);
    }

    // Parse the output to extract dimensions
    let dimensions = String::from_utf8_lossy(&output.stdout);
    debug!("Raw dimensions output from ffprobe: {}", dimensions.trim());

    let dimensions: Vec<&str> = dimensions.trim().split('x').collect();
    if dimensions.len() != 2 {
        debug!("Unexpected dimensions format: {}", dimensions.join("x"));
        bail!("Unexpected dimensions format: {}", dimensions.join("x"));
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
        bail!("Process interrupted by user");
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
        bail!("Failed to resize video: {}", stderr);
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
    let new_duration = duration + 1.0;
    debug!("Cutting video to {} seconds", new_duration);

    // Check if the process is still running
    if !running.load(Ordering::SeqCst) {
        bail!("Process interrupted by user");
    }

    StdCommand::new("ffmpeg")
        .args(&[
            "-y", // Automatically overwrite existing files
            "-i",
            input_path,
            "-t",
            &new_duration.to_string(),
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
        bail!("Process interrupted by user");
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
        bail!("Failed to change framerate");
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
