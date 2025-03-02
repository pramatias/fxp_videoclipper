use anyhow::{anyhow, Context, Result};
// use ctrlc;
use log::debug;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};


use modes::Modes;
use output::ModeOutput;
use output::Output;

use crate::sample::{extract_multiple_frames, extract_single_frame};

/// A collection of arguments for video sampling operations.
///
/// This struct holds the necessary parameters for processing video content,
/// including video file path, duration in milliseconds, and optional sampling
/// configuration.
///
/// # Parameters
/// - `video_path`: Path to the video file to process.
/// - `duration`: Duration of the video content in milliseconds.
/// - `sampling_number`: Optional number of samples to collect.
///
/// # Returns
/// - `Self`: A new instance of `Sampler` with the provided parameters.
///
/// # Notes
/// - The `sampling_number` parameter is optional and will default to a
///   calculated value if not provided.
#[derive(Debug)]
pub struct Sampler {
    pub video_path: PathBuf,
    pub output_path: Option<PathBuf>,
    pub duration: u64,
    pub sampling_number: Option<usize>,
}

impl Sampler {
    pub fn new(
        video_path: String,
        output_path: Option<String>,
        duration: u64,
        sampling_number: Option<usize>,
    ) -> Result<Self> {
        let video_path = PathBuf::from(&video_path);

        // Set up mode and convert to Output (assumes Modes and Output are defined similarly to Merger)
        let mode: Modes = Modes::Sampler;
        let output: Output = mode.into();

        // Use the trait method to create the output directory.
        let output_path = match output {
            Output::Sampler(sampler_output) => {
                sampler_output.create_output_directory((video_path.clone(), output_path))?
            }
            _ => unreachable!("Expected Sampler mode"),
        };

        Ok(Self {
            video_path,
            output_path: Some(output_path),
            duration,
            sampling_number,
        })
    }
}

impl Sampler {
    /// Processes video frames based on specified sampling parameters.
    ///
    /// This method handles video frame extraction with support for single or multiple frames.
    /// It ensures proper handling of processing interruption and validates input parameters.
    ///
    /// # Parameters
    /// - `running`: An atomic boolean flag indicating whether processing should continue.
    ///
    /// # Returns
    /// - `Result<()>`: Indicates success or failure of the processing operation.
    ///
    /// # Notes
    /// - The method exits early if the `running` flag is set to false.
    /// - Validates that video duration is greater than zero.
    /// - Supports sampling of a single frame or multiple evenly spaced frames.
    pub fn sample_images(&self, running: Arc<AtomicBool>) -> Result<()> {
        debug!("Starting sample processing with arguments: {:?}", self);

        // Check if the running flag is true; if false, exit early.
        if !running.load(Ordering::SeqCst) {
            return Err(anyhow!("Processing interrupted before starting."));
        }

        if self.duration == 0 {
            return Err(anyhow!("Invalid video duration: must be greater than 0."));
        }

        let output_path = self
            .output_path
            .as_ref()
            .ok_or_else(|| anyhow!("Output path not provided"))?;

        match self.sampling_number {
            Some(1) => {
                extract_single_frame(
                    &self.video_path,
                    self.duration,
                    Path::new(output_path).to_path_buf(), // Convert &Path to PathBuf
                    running.clone(),
                )
                .context("Failed to extract single frame")?;
            }
            Some(num_frames) if num_frames > 1 => {
                // Extract multiple evenly spaced frames.
                extract_multiple_frames(
                    &self.video_path,
                    self.duration,
                    num_frames,
                    output_path, // Provide the output directory
                    running.clone(),
                )
                .context("Failed to extract multiple frames")?;
            }
            _ => {
                return Err(anyhow!(
                    "Invalid sampling number: {:?}",
                    self.sampling_number
                ));
            }
        }

        Ok(())
    }
}
