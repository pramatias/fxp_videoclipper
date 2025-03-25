use anyhow::{Context, Result};
use indicatif::ProgressBar;
use indicatif::ProgressStyle;
use log::debug;
use std::ffi::OsStr;
use std::path::Path;
use std::path::PathBuf;
use std::process::exit;
use std::process::Command;
use std::process::Stdio;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::{fs, thread, time::Duration};

/// Creates a video from image frames without audio using ffmpeg.
///
/// This function takes a directory of image frames, processes them into a video
/// at the specified FPS, and saves the result to a temporary directory. It
/// includes progress tracking and supports cancellation.
///
/// # Parameters
/// - `input_dir`: Directory containing the image frames to process.
/// - `fps`: Frame rate for the output video.
/// - `tmp_dir`: Temporary directory to store the output video.
/// - `output_path`: Desired output filename for the video.
/// - `running`: Flag to check if the process should continue running.
///
/// # Returns
/// - `PathBuf`: Path to the created video file.
///
/// # Notes
/// - The function assumes image frames follow a zero-padded numbering format.
/// - Supports cancellation via the `running` flag.
/// - The output filename will have a `_no_audio` suffix.
pub fn create_video_without_audio(
    input_dir: &Path,
    fps: u32,
    tmp_dir: &Path,
    output_path: &Path,
    running: Arc<AtomicBool>,
) -> PathBuf {
    debug!("Starting video creation process without audio...");

    // Ensure correct frame pattern with zero-padded four-digit numbering
    let frame_pattern = input_dir
        .join("frame_%04d.png")
        .to_string_lossy()
        .to_string();
    debug!("Input frame pattern: {}", frame_pattern);

    // Convert fps to a string for ffmpeg.
    let fps_str = fps.to_string();
    debug!("Using FPS: {}", fps_str);

    // Extract the file stem from output_path and create a new filename with _no_audio suffix.
    let file_stem = output_path
        .file_stem()
        .unwrap_or_else(|| OsStr::new("output"))
        .to_string_lossy();
    let new_filename = format!("{}_no_audio.mp4", file_stem);
    let output_file = tmp_dir.join(new_filename);
    let output_filename = output_file.to_string_lossy().to_string();
    debug!("Output video file: {}", output_filename);

    // Set up a spinner-style progress bar.
    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(Duration::from_millis(100));
    let style = ProgressStyle::default_spinner()
        .template("{spinner:.green} [{elapsed_precise}] {msg}")
        .expect("Failed to set progress bar template");
    pb.set_style(style);
    pb.set_message("Creating video...");

    // Spawn the ffmpeg process.
    debug!("Spawning ffmpeg process to create video...");
    let mut child = Command::new("ffmpeg")
        .args(&[
            "-framerate",
            &fps_str,
            "-start_number",
            "1",
            "-i",
            &frame_pattern,
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            &output_filename,
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to spawn ffmpeg process");

    // Poll the process periodically, checking for interruption.
    loop {
        if running.load(Ordering::Relaxed) {
            pb.finish_with_message("Running by user!");
            // Attempt to kill the ffmpeg process.
            if let Err(e) = child.kill() {
                debug!("Failed to kill ffmpeg process: {}", e);
            }
            eprintln!("Video creation interrupted by user.");
            exit(1);
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                pb.finish_with_message("Video creation completed!");
                debug!("ffmpeg command finished with status: {}", status);
                if !status.success() {
                    eprintln!("ffmpeg command failed");
                    exit(1);
                }
                break;
            }
            Ok(None) => {
                // Process still running. Sleep a little before polling again.
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => {
                pb.finish_with_message("Error checking process status");
                eprintln!("Error while checking ffmpeg process: {}", e);
                exit(1);
            }
        }
    }

    debug!("Audio-free video saved as {}", output_filename);
    output_file
}

/// Merges a video file with an audio file using FFmpeg.
///
/// This function combines the video and audio streams into a single output file.
/// It uses FFmpeg under the hood to ensure proper encoding and formatting.
///
/// # Parameters
/// - `video_path`: The path to the video file to process.
/// - `mp3_path`: The path to the audio file to merge.
/// - `running`: A flag indicating whether the operation should continue.
///
/// # Returns
/// - `PathBuf`: The path to the merged output file.
///
/// # Notes
/// - The output file is placed in the same directory as the video file, named with "_videoclipped" appended.
/// - If an output file already exists at the target path, it will be deleted before creating a new one.
/// - FFmpeg is used with standard settings for video copying and audio re-encoding.
/// - The process can be interrupted by setting the `running` flag.
pub fn merge_video_audio(
    video_path: &PathBuf,
    mp3_path: &PathBuf,
    running: Arc<AtomicBool>,
) -> PathBuf {
    log::debug!(
        "Starting merge of video: {:?} and audio: {:?}",
        video_path,
        mp3_path
    );

    // Determine the parent directory (defaulting to the current directory if unavailable)
    let parent_dir = video_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));

    // Build the output file path: same name as video with "_videoclipped" appended and .mp4 extension
    let output_path = parent_dir
        .join({
            let mut file_stem = video_path
                .file_stem()
                .unwrap_or_else(|| std::ffi::OsStr::new("output"))
                .to_os_string();
            file_stem.push("_videoclipped");
            file_stem
        })
        .with_extension("mp4");

    // Delete the output file if it already exists
    if output_path.exists() {
        log::debug!(
            "Output file already exists at {:?}, deleting it...",
            output_path
        );
        fs::remove_file(&output_path).expect("Failed to remove existing merged video file");
        log::debug!("Existing output file deleted successfully.");
    }

    log::debug!(
        "Merging video and audio into output file: {:?}",
        output_path
    );

    // Start the ffmpeg command as a child process so that we can monitor it
    let mut child = Command::new("ffmpeg")
        .args(&[
            "-y",
            "-i",
            video_path.to_str().expect("Invalid video path"),
            "-i",
            mp3_path.to_str().expect("Invalid mp3 path"),
            "-c:v",
            "copy",
            "-c:a",
            "aac",
            output_path.to_str().expect("Invalid output path"),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to spawn ffmpeg process");

    // Periodically poll the child process while also checking for interruption
    loop {
        // Check if the process has finished
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    log::debug!("FFmpeg command failed with status: {:?}", status);
                    panic!("Failed to merge video and audio");
                }
                break;
            }
            Ok(None) => {
                // Check for interruption
                if running.load(Ordering::Relaxed) {
                    log::debug!("Interrupt flag detected. Terminating ffmpeg process.");
                    child.kill().expect("Failed to kill ffmpeg process");
                    panic!("Merge operation interrupted by user");
                }
                // Sleep for a short duration before checking again
                thread::sleep(Duration::from_millis(100));
            }
            Err(e) => {
                panic!("Error attempting to wait for ffmpeg process: {}", e);
            }
        }
    }

    debug!("Merged audio and video saved as {:?}", output_path);

    output_path
}

/// Trims a merged video using ffmpeg to a specified duration.
///
/// This function executes an ffmpeg command to trim a video to the specified duration.
/// It handles interruptions through a running flag and ensures proper file management.
///
/// # Parameters
/// - `video_path`: Path to the input video file.
/// - `duration_ms`: Desired duration of the trimmed video in milliseconds.
/// - `output_path`: Path where the trimmed video will be saved.
/// - `running`: Flag to control the execution state.
///
/// # Returns
/// - `Result<PathBuf>`: Path to the trimmed video file on success.
///
/// # Notes
/// - The function uses a temporary file to ensure proper formatting.
/// - Interrupts the process if the `running` flag is set to false.
pub fn trim_merged_video(
    video_path: std::path::PathBuf,
    duration_ms: u64,
    output_path: std::path::PathBuf,
    running: Arc<AtomicBool>,
) -> anyhow::Result<std::path::PathBuf> {
    // Preserve the original output path.
    let original_output = output_path.clone();

    // Determine the temporary output path that guarantees a .mp4 extension.
    let tmp_output = get_tmp_output_path(&output_path);

    // Convert the duration from milliseconds to seconds (ffmpeg expects seconds).
    let duration_secs = (duration_ms as f64) / 1000.0;

    log::debug!(
        "Trimming video at {} to {} seconds ({} ms)",
        video_path.display(),
        duration_secs,
        duration_ms
    );
    log::debug!("Output path for trimmed video: {}", tmp_output.display());

    // Build the ffmpeg command
    let mut child = Command::new("ffmpeg")
        .args(&[
            "-y",
            "-i",
            video_path
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Invalid video path"))?,
            "-t",
            &duration_secs.to_string(),
            "-c",
            "copy",
            tmp_output
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Invalid temporary output path"))?,
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("Failed to start ffmpeg for trimming")?;

    // Periodically check for an interruption.
    loop {
        // Check if the running flag was triggered.
        if running.load(Ordering::Relaxed) {
            log::debug!("Interruption requested; terminating ffmpeg process.");
            // Kill the ffmpeg process.
            child.kill().ok();
            return Err(anyhow::anyhow!("Operation interrupted by user"));
        }

        // Check if the child process has exited.
        match child.try_wait()? {
            Some(status) => {
                if !status.success() {
                    log::debug!("FFmpeg command failed with status: {:?}", status);
                    return Err(anyhow::anyhow!("Failed to trim merged video"));
                }
                break;
            }
            None => {
                // ffmpeg is still running; sleep briefly before checking again.
                thread::sleep(Duration::from_millis(100));
            }
        }
    }

    // If a temporary file was used, rename it to the original output path.
    rename_output_file_if_needed(&tmp_output, &original_output)?;

    log::debug!(
        "Final video trimmed and saved to {}",
        original_output.display()
    );

    Ok(original_output)
}

/// Ensures the output path ends with .mp4 extension.
///
/// This function validates and adjusts the output path to ensure it has a .mp4 extension.
///
/// # Parameters
/// - `output_path`: The input path to be validated and potentially modified.
///
/// # Returns
/// - `PathBuf`: The normalized path with a .mp4 extension.
///
/// # Notes
/// - If the original path already has a .mp4 extension, it is returned unchanged.
fn get_tmp_output_path(output_path: &Path) -> PathBuf {
    if output_path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase())
        != Some("mp4".to_string())
    {
        // Replace or append with ".mp4".
        output_path.with_extension("mp4")
    } else {
        output_path.to_owned()
    }
}

/// Renames a temporary output file to its original name if necessary.
///
/// This function checks if the temporary output path differs from the original
/// output path, and if so, renames the file accordingly.
///
/// # Parameters
/// - `tmp_output`: The temporary output file path.
/// - `original_output`: The original output file path.
///
/// # Returns
/// - `Result<()>`: Indicates success or failure of the rename operation.
///
/// # Notes
/// - No action is taken if `tmp_output` and `original_output` are the same.
fn rename_output_file_if_needed(tmp_output: &Path, original_output: &Path) -> Result<()> {
    if tmp_output != original_output {
        fs::rename(&tmp_output, &original_output).with_context(|| {
            format!(
                "Failed to rename {} to {}",
                tmp_output.display(),
                original_output.display()
            )
        })?;
    }
    Ok(())
}
