use anyhow::{Context, Result};
use ctrlc;
use log::{debug, warn};
use std::fs;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use modes::Modes;
use output::ModeOutput;
use output::Output;

use crate::export::{cut_duration_adjust_fps_resize, extract_all_frames_with_progress};

/// Represents arguments for video export configuration.
///
/// This struct holds the necessary parameters for exporting video files.
///
/// # Parameters
/// - `video_path`: A `String` specifying the path to the video file.
/// - `duration`: A `u64` representing the duration of the video in seconds.
/// - `fps`: A `u32` specifying the frames per second for the video.
/// - `pixel_upper_limit`: A `u32` indicating the upper limit of pixels.
///
/// # Returns
/// - `Self`: An instance of `Exporter` with the provided configuration.
///
/// # Notes
/// - The `pixel_upper_limit` must be a positive value.
#[derive(Debug, Clone)]
pub struct Exporter {
    pub video_path: PathBuf,
    pub output_dir: PathBuf,
    pub duration: u64,
    pub fps: u32,
    pub pixel_upper_limit: u32,
}

impl Exporter {
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
    /// - The method cleans up temporary files in release mode but retains them in debug mode.
    /// - Provides progress tracking during frame extraction.
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

        let tmp_dir_path = std::path::PathBuf::from("/tmp/fxp_videoclipper");

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

        if let Err(e) = delete_temporary_dir(tmp_dir_path.clone()) {
            warn!("Delete of temporary directory failed: {}", e);
        }

        Ok(())
    }
}

fn delete_temporary_dir(tmp_dir: PathBuf) -> Result<()> {
    if cfg!(debug_assertions) {
        debug!(
            "Debug mode detected, not deleting temporary path: {}",
            tmp_dir.display()
        );
    } else {
        debug!(
            "Release mode detected, deleting temporary path: {}",
            tmp_dir.display()
        );
        if tmp_dir.is_dir() {
            fs::remove_dir_all(&tmp_dir).context(format!(
                "Failed to delete up directory: {}",
                tmp_dir.display()
            ))?;
        } else {
            fs::remove_file(&tmp_dir).context(format!(
                "Failed to delete temporary file: {}",
                tmp_dir.display()
            ))?;
        }
    }
    Ok(())
}
