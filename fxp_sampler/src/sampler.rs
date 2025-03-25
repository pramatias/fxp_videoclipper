use anyhow::{anyhow, Context, Result};
// use ctrlc;
use log::debug;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use fxp_modes::Modes;
use fxp_output::ModeOutput;
use fxp_output::Output;

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
    pub output_path: PathBuf,
    pub duration: u64,
    pub sampling_number: usize,
}

impl Sampler {
    /// Creates a new Sampler instance for video processing.
    ///
    /// This function initializes a Sampler with the specified parameters and sets up the output directory.
    ///
    /// # Parameters
    /// - `video_path`: The path to the video file to process.
    /// - `output_path`: An optional path for the output directory; if not provided, a default will be used.
    /// - `duration`: The duration of the video in seconds.
    /// - `sampling_number`: The number of samples to take from the video.
    ///
    /// # Returns
    /// - `Result<Self>`: Returns `Ok` if the Sampler was created successfully, `Err` if there was an issue creating the output directory.
    ///
    /// # Notes
    /// - The output directory will be created if it does not already exist.
    /// - If `output_path` is not provided, a default output path will be generated.
    pub fn new(
        video_path: String,
        output_path: Option<String>,
        duration: u64,
        sampling_number: usize,
    ) -> Result<Self> {
        let video_path = PathBuf::from(&video_path);

        // Set up mode and convert to Output (assumes Modes and Output are defined similarly to Merger)
        let mode: Modes = Modes::Sampler;
        let output: Output = mode.into();

        // Use the trait method to create the output directory.
        let output_path = match output {
            Output::Sampler(sampler_output) => {
                sampler_output.create_output((video_path.clone(), output_path, sampling_number))?
            }
            _ => unreachable!("Expected Sampler mode"),
        };

        Ok(Self {
            video_path,
            output_path: output_path,
            duration,
            sampling_number,
        })
    }
}

impl Sampler {
    /// Processes video frames for sampling based on specified criteria.
    ///
    /// This function handles the extraction of frames from a video file, either as a single frame or multiple evenly spaced frames.
    ///
    /// # Parameters
    /// - `running`: A flag indicating whether the process should continue.
    /// - `video_path`: Path to the video file to process.
    /// - `duration`: Total duration of the video.
    /// - `output_path`: Directory where the output images will be saved.
    /// - `sampling_number`: Number of frames to extract (1 for single, >1 for multiple).
    ///
    /// # Returns
    /// - `Result<()>`: Indicates success or failure of the operation.
    ///
    /// # Notes
    /// - If `running` is false, the function exits early.
    /// - If `duration` is 0, returns an error as it's an invalid value.
    /// - Based on `sampling_number`, the function will either extract a single frame or multiple frames.
    pub fn sample_images(&self, running: Arc<AtomicBool>) -> Result<()> {
        debug!("Starting sample processing with arguments: {:?}", self);

        // Check if the running flag is true; if false, exit early.
        if !running.load(Ordering::SeqCst) {
            return Err(anyhow!("Processing interrupted before starting."));
        }

        if self.duration == 0 {
            return Err(anyhow!("Invalid video duration: must be greater than 0."));
        }

        let output_path = &self.output_path;

        match self.sampling_number {
            1 => {
                extract_single_frame(
                    &self.video_path,
                    self.duration,
                    output_path.clone(), // Convert &Path to PathBuf
                    running.clone(),
                )
                .context("Failed to extract single frame")?;
            }
            num_frames if num_frames > 1 => {
                // Extract multiple evenly spaced frames.
                extract_multiple_frames(
                    &self.video_path,
                    self.duration,
                    num_frames,
                    &output_path, // Provide the output directory
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
