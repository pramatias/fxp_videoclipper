use anyhow::{Context, Result};
use ctrlc;
use log::debug;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use modes::Modes;
use output::ModeOutput;
use output::Output;

use crate::export::{cut_video_section, extract_all_frames_with_progress};

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
                exporter_output.create_output_directory((video_path.clone(), output))?
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

        let (cut_video_path, cut_duration) = cut_video_section(
            &self.video_path.to_str().unwrap(),
            self.duration,
            self.pixel_upper_limit,
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

        debug!("Frame extraction completed for video: {}", cut_video_path);

        if cfg!(debug_assertions) {
            debug!(
                "Debug mode detected, not deleting cut video path: {}",
                cut_video_path
            );
        } else {
            debug!(
                "Release mode detected, deleting cut video path: {}",
                cut_video_path
            );
            std::fs::remove_file(&cut_video_path).context("Failed to clean up cut video file")?;
        }

        Ok(())
    }
}
