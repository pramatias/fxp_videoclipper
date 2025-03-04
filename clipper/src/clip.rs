use anyhow::{Context, Result};
use indicatif::ProgressBar;
use indicatif::ProgressStyle;
use log::debug;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::exit;
use std::process::Command;
use std::process::Stdio;
use std::time::Duration;

pub fn create_video_without_audio(
    input_dir: &Path,
    fps: u32,
    tmp_dir: &Path,
    output_path: &Path,
) -> PathBuf {
    debug!("Starting video creation process without audio...");

    // Ensure correct frame pattern with zero-padded four-digit numbering
    let frame_pattern = input_dir
        .join("image_%04d.png")
        .to_string_lossy()
        .to_string();
    debug!("Input frame pattern: {}", frame_pattern);

    // Set up a spinner-style progress bar.
    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(Duration::from_millis(100));
    let style = ProgressStyle::default_spinner()
        .template("{spinner:.green} [{elapsed_precise}] {msg}")
        .expect("Failed to set progress bar template");
    pb.set_style(style);
    pb.set_message("Creating video...");

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

    // Run ffmpeg command to generate video from the frames with correct numbering format.
    debug!("Executing ffmpeg command to create video...");
    let ffmpeg_status = Command::new("ffmpeg")
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
        .status()
        .expect("Failed to execute ffmpeg");

    pb.finish_with_message("Video creation completed!");
    debug!(
        "ffmpeg command execution completed with status: {}",
        ffmpeg_status
    );

    if !ffmpeg_status.success() {
        debug!("ffmpeg command failed with status: {}", ffmpeg_status);
        eprintln!("ffmpeg command failed");
        exit(1);
    }

    debug!("Audio-free video saved as {}", output_filename);
    output_file
}

pub fn merge_video_audio(video_path: &PathBuf, mp3_path: &PathBuf) -> PathBuf {
    debug!(
        "Starting merge of video: {:?} and audio: {:?}",
        video_path, mp3_path
    );

    // Determine the parent directory (defaulting to the current directory if unavailable)
    let parent_dir = video_path.parent().unwrap_or_else(|| Path::new("."));

    // Build the output file path: same name as video with "_merged" appended and .mp4 extension
    let output_path = parent_dir
        .join({
            let mut file_stem = video_path
                .file_stem()
                .unwrap_or_else(|| std::ffi::OsStr::new("output"))
                .to_os_string();
            file_stem.push("_merged");
            file_stem
        })
        .with_extension("mp4");

    // Delete the output file if it already exists
    if output_path.exists() {
        debug!(
            "Output file already exists at {:?}, deleting it...",
            output_path
        );
        fs::remove_file(&output_path).expect("Failed to remove existing merged video file");
        debug!("Existing output file deleted successfully.");
    }

    debug!(
        "Merging video and audio into output file: {:?}",
        output_path
    );

    // Execute ffmpeg command to merge the video and audio
    let status = Command::new("ffmpeg")
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
        .status()
        .expect("Failed to merge video and audio");

    if !status.success() {
        debug!("FFmpeg command failed with status: {}", status);
        panic!("Failed to merge video and audio");
    }

    debug!("Merged audio and video saved as {:?}", output_path);

    output_path
}

/// Trims a merged video using ffmpeg.
///
/// This function uses `get_tmp_output_path` to ensure the output file ends with `.mp4`.
/// After ffmpeg creates the trimmed file, if a temporary filename was used,
/// it is renamed back to the original output filename using `rename_output_file_if_needed`.
pub fn trim_merged_video(
    video_path: PathBuf,
    duration_ms: u64,
    output_path: PathBuf,
) -> Result<PathBuf> {
    // Preserve the original output path.
    let original_output = output_path.clone();

    // Determine the temporary output path that guarantees a .mp4 extension.
    let tmp_output = get_tmp_output_path(&output_path);

    // Convert the duration from milliseconds to seconds (ffmpeg expects seconds).
    let duration_secs = (duration_ms as f64) / 1000.0;

    debug!(
        "Trimming video at {} to {} seconds ({} ms)",
        video_path.display(),
        duration_secs,
        duration_ms
    );
    debug!("Output path for trimmed video: {}", tmp_output.display());

    // Execute ffmpeg command to trim the video.
    let status = Command::new("ffmpeg")
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
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .context("Failed to execute ffmpeg for trimming")?;

    if !status.success() {
        debug!("FFmpeg command failed with status: {}", status);
        anyhow::bail!("Failed to trim merged video");
    }

    // If a temporary file was used, rename it to the original output path.
    rename_output_file_if_needed(&tmp_output, &original_output)?;

    debug!(
        "Final video trimmed and saved to {}",
        original_output.display()
    );

    Ok(original_output)
}

/// Returns an output path that ends with `.mp4`.
/// If `output_path` does not already have a `.mp4` extension (case-insensitive),
/// a new `PathBuf` with the `.mp4` extension is returned. Otherwise, the original path is cloned.
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

/// Renames the file from `tmp_output` to `original_output` if they differ.
/// If no renaming is needed (paths are identical), this function does nothing.
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
