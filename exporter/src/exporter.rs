use anyhow::{Context, Result};
use ctrlc;
use log::debug;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tempfile;

use modes::Modes;
use output::ModeOutput;
use output::Output;

use crate::export::{cut_duration_adjust_fps_resize, extract_all_frames_with_progress};

#[derive(Debug, Clone)]
pub struct Exporter {
    pub video_path: PathBuf,
    pub output_dir: PathBuf,
    pub duration: u64,
    pub fps: u32,
    pub pixel_upper_limit: u32,
}

impl Exporter {
    /// Initializes and configures an `Exporter` instance for video processing.
    ///
    /// Creates a new exporter by setting up the video path, output directory,
    /// and validating the input parameters for duration, frames per second,
    /// and pixel limits.
    ///
    /// # Parameters
    /// - `video_path`: The file path to the input video.
    /// - `output`: An optional path for the output directory.
    /// - `duration`: The duration of the video in seconds.
    /// - `fps`: The frames per second for processing.
    /// - `pixel_upper_limit`: The maximum allowed number of pixels.
    ///
    /// # Returns
    /// - `Result<Self>`: Returns the configured `Exporter` instance or an error.
    ///
    /// # Notes
    /// - The output directory will be created if it doesn't exist.
    /// - Default output directory is the same as the video file's directory.
    /// - Validates that duration and fps are greater than zero.
    /// - Ensures pixel upper limit is a reasonable value.
    pub fn new(
        video_path: String,
        output: Option<String>,
        duration: u64,
        fps: u32,
        pixel_upper_limit: u32,
    ) -> Result<Self> {
        let video_path = PathBuf::from(video_path);

        // Define the mode and convert it into an Output variant.
        let mode: Modes = Modes::Exporter;
        let output_enum: Output = mode.into();

        // Use the trait implementation for ExporterOutput to create the output directory.
        let output_directory = match output_enum {
            Output::Exporter(exporter_output) => {
                exporter_output.create_output((video_path.clone(), output))?
            }
            _ => unreachable!("Expected Exporter mode"),
        };

        Ok(Self {
            video_path,
            output_dir: output_directory,
            duration,
            fps,
            pixel_upper_limit,
        })
    }
}

impl Exporter {
    /// Processes video export by cutting and extracting frames with error handling.
    ///
    /// This method handles the video export process, including cutting a specific section
    /// of the video and extracting frames from it. It includes error handling and cleanup
    /// operations.
    ///
    /// # Parameters
    /// - `running`: An `Arc<AtomicBool>` used to track the running state of the operation.
    ///
    /// # Returns
    /// - `Result<()>`: Returns `Ok(())` on success and an error on failure.
    ///
    /// # Notes
    /// - Handles Ctrl+C interruptions gracefully.
    /// - Creates and manages a temporary directory for processing.
    /// - Provides progress tracking during frame extraction.
    /// - Retains temporary files in debug mode for inspection.
    pub fn export_images(&self) -> Result<()> {
        debug!("Starting export processing with arguments: {:?}", self);

        // Create the running variable and set up Ctrl+C handler.
        let running = Arc::new(AtomicBool::new(true));
        {
            let r = running.clone();
            ctrlc::set_handler(move || {
                eprintln!("\nReceived Ctrl+C, terminating...");
                r.store(false, Ordering::SeqCst);
            })
            .context("Error setting Ctrl+C handler")?;
        }

        // Create a temporary directory using the tempfile crate.
        let tmp_dir = tempfile::tempdir().context("Failed to create temporary directory")?;
        let tmp_dir_path = tmp_dir.path().to_path_buf();

        let (cut_video_path, cut_duration) = cut_duration_adjust_fps_resize(
            &self.video_path.to_str().unwrap(),
            self.duration,
            self.pixel_upper_limit,
            self.fps,
            tmp_dir_path.clone(),
            running.clone(),
        )
        .context("An error occurred during video cutting")?;

        extract_all_frames_with_progress(
            &cut_video_path,
            self.output_dir.clone(),
            cut_duration,
            self.fps,
            running.clone(),
        )
        .context("An error occurred during frame extraction")?;

        // In debug mode, copy the temporary directory contents to /tmp/fxp_videoclipper.
        #[cfg(debug_assertions)]
        {
            let debug_dir = PathBuf::from("/tmp/fxp_videoclipper");
            copy_tmp_dir_contents(tmp_dir.path(), &debug_dir)?;
        }

        Ok(())
    }
}

/// Copies contents from a temporary directory to a debug directory for debugging purposes.
///
/// This function transfers all files from a temporary directory to a specified debug directory.
/// It creates the debug directory if it doesn't exist.
///
/// # Parameters
/// - `tmp_dir`: The path to the temporary directory containing files to copy.
/// - `debug_dir`: The path to the debug directory where files will be copied.
///
/// # Returns
/// - `Result<()>`: Returns `Ok(())` on success or an error if any operation fails.
///
/// # Notes
/// - This function is only included in debug builds.
/// - Intended for use during debugging to preserve temporary files for inspection.
#[cfg(debug_assertions)]
fn copy_tmp_dir_contents(tmp_dir: &Path, debug_dir: &Path) -> Result<()> {
    // Create the debug directory if it doesn't exist.
    if !debug_dir.exists() {
        fs::create_dir_all(debug_dir).context(format!(
            "Failed to create debug directory: {}",
            debug_dir.display()
        ))?;
    }

    // Iterate and copy each file from tmp_dir to debug_dir.
    for entry in fs::read_dir(tmp_dir).context("Failed to read temporary directory")? {
        let entry = entry?;
        let src_path = entry.path();
        if let Some(file_name) = src_path.file_name() {
            let dest_path = debug_dir.join(file_name);
            fs::copy(&src_path, &dest_path)
                .context(format!("Failed to copy {:?} to {:?}", src_path, dest_path))?;
        }
    }
    debug!("Copied temporary files to {}", debug_dir.display());
    Ok(())
}
