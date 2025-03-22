use anyhow::{anyhow, Context, Result};
use ctrlc;
use log::{debug, info};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tempfile::TempDir;

use modes::Modes;
use output::ModeOutput;
use output::Output;

use crate::clip::create_video_without_audio;
use crate::clip::merge_video_audio;
use crate::clip::trim_merged_video;

use filenames::FileOperations;
use filenames::ImageMappingError;

/// Struct for handling video processing operations.
#[derive(Debug)]
pub struct Clipper {
    /// The input directory containing video frames or related resources.
    pub input_dir: PathBuf,

    /// Output directory where the processed video will be saved.
    pub output_path: PathBuf,

    /// Optional path to an MP3 file to overlay or process with the video.
    pub mp3_path: Option<PathBuf>,

    /// Frames per second (FPS) value for the output video.
    pub fps: u32,

    /// Duration in milliseconds to use for video processing.
    pub duration: Option<u64>,
}

impl Clipper {
    /// Creates a new Clipper instance.
    ///
    /// # Parameters
    /// - `input_dir`: Path to the input directory.
    /// - `mp3_path`: Optional MP3 file path.
    /// - `output_path`: Optional output directory; if not provided, a default directory will be created inside the input directory.
    /// - `fps`: Frames per second for the output video.
    /// - `duration`: Duration in milliseconds for the video.
    pub fn new(
        input_dir: String,
        mp3_path: Option<String>,
        output_path: Option<String>,
        fps: u32,
        duration: Option<u64>,
    ) -> Result<Self> {
        debug!("Initializing Clipper instance...");

        // Validate fps.
        if fps == 0 {
            debug!("FPS validation failed: FPS must be greater than zero");
            return Err(anyhow!("FPS must be greater than zero"));
        }
        debug!("FPS validated: {}", fps);

        // Convert and validate input_dir.
        let input_dir = PathBuf::from(input_dir);
        debug!("Input directory: {:?}", input_dir);
        if !input_dir.exists() || !input_dir.is_dir() {
            debug!(
                "Input directory validation failed: {} does not exist or is not a directory",
                input_dir.display()
            );
            return Err(anyhow!(
                "Input directory does not exist or is not a directory: {}",
                input_dir.display()
            ));
        }
        debug!("Input directory validated successfully.");

        // Validate MP3 if provided, and keep the original string for output directory creation.
        // let mp3_path_str = mp3_path.clone();
        let mp3_path = mp3_path.map(PathBuf::from);
        if let Some(ref mp3) = mp3_path {
            debug!("MP3 file provided: {:?}", mp3);
            if !mp3.exists() || !mp3.is_file() {
                debug!(
                    "MP3 file not found: {}. Continuing without a valid MP3.",
                    mp3.display()
                );
            } else {
                debug!("MP3 file validated successfully.");
            }
        } else {
            debug!("No MP3 file provided.");
        }
        debug!("Output path provided: {:?}", output_path);

        // Use the trait-based output directory creation.
        let mode: Modes = Modes::Clipper;
        let output: Output = mode.into();
        let output_directory_path = match output {
            Output::Clipper(clipper_output) => {
                clipper_output.create_output((input_dir.clone(), mp3_path.clone(), output_path))?
            }
            _ => unreachable!("Expected Clipper mode"),
        };
        debug!("Generated output directory: {:?}", output_directory_path);

        // (Optional) Log additional details from the setup.
        let (final_out_dir, _frames, total_frames) =
            setup_clipper_processing(&input_dir, &output_directory_path)?;
        debug!("Clipper setup complete: {} frames found", total_frames);

        debug!("Clipper instance created successfully.");
        Ok(Self {
            input_dir,
            mp3_path,
            output_path: final_out_dir,
            fps,
            duration,
        })
    }
}

impl Clipper {
    pub fn clip(&self) -> Result<PathBuf> {
        debug!("Starting video clipping process...");

        // Create a temporary directory for the processed frames.
        let tmp_dir = TempDir::new().context("Failed to create temporary directory")?;
        let tmp_dir_path = tmp_dir.path().to_path_buf();
        debug!("Temporary directory created at: {:?}", tmp_dir_path);

        // Set up the running flag and register a Ctrl-C handler.
        let running = Arc::new(AtomicBool::new(false));
        let running_clone = running.clone();
        ctrlc::set_handler(move || {
            running_clone.store(true, Ordering::Relaxed);
        })
        .expect("Error setting Ctrl-C handler");

        // Create the video without audio, passing the running flag.
        debug!("Creating video without audio...");
        let video_path_no_audio = create_video_without_audio(
            &self.input_dir,
            self.fps,
            &tmp_dir_path,
            &self.output_path,
            running.clone(),
        );
        debug!("Video without audio created at: {:?}", video_path_no_audio);

        // Process the video based on whether an MP3 is provided.
        let final_video_path = if let Some(mp3) = &self.mp3_path {
            debug!("MP3 file provided: {:?}. Merging video and audio...", mp3);

            // Merge the video with the audio, passing the interruption flag.
            let merged_video_path = merge_video_audio(&video_path_no_audio, mp3, running.clone());
            debug!("Video and audio merged at: {:?}", merged_video_path);

            // Unwrap duration (handle None case as needed)
            let duration = self.duration.expect("duration must be provided");
            debug!("Trimming merged video to duration: {} ms", duration);

            let trimmed_video_path = trim_merged_video(
                merged_video_path,
                duration,
                self.output_path.clone(),
                running.clone(),
            )?;
            debug!("Trimmed video saved at: {:?}", trimmed_video_path);

            self.output_path.clone()
        } else {
            debug!("No MP3 file provided. Copying video without audio to output path...");

            // If no MP3 is provided, simply copy the video without audio.
            fs::copy(&video_path_no_audio, &self.output_path)
                .context("Failed to copy video without audio to output directory")?;
            debug!(
                "Video without audio copied to output path: {:?}",
                self.output_path
            );

            self.output_path.clone()
        };

        // Clean up the temporary directory.
        debug!("Cleaning up temporary directory: {:?}", tmp_dir_path);
        rm_tmp_dir(Some(&tmp_dir_path))?;
        debug!("Temporary directory cleaned up successfully.");

        debug!(
            "Video clipping process completed successfully. Final video saved at: {:?}",
            final_video_path
        );

        Ok(final_video_path)
    }
}

fn setup_clipper_processing(
    input_directory: &Path,
    output_directory: &Path,
) -> Result<(PathBuf, BTreeMap<u32, PathBuf>, usize)> {
    debug!("Starting setup for Clipper processing");

    // Read the input directory and collect all file paths.
    let images: Vec<PathBuf> = fs::read_dir(input_directory)
        .context("Failed to read input directory")?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .collect();
    debug!("Found {} files in input directory", images.len());

    // Use FileOperations trait implemented for Modes on the Clipper mode.
    // This replaces the usage of PrefixSuffixValidator.
    let frames = Modes::Clipper
        .load_files(&images)
        .map_err(|e| ImageMappingError::RenameError(e.to_string()))?;
    debug!("Total images after validation: {}", frames.len());

    let total_frames = frames.len();
    if total_frames == 0 {
        debug!("No valid image frames found in input directory after validation");
        return Err(anyhow!(
            "No valid image frames found in input directory: {}",
            input_directory.display()
        ));
    }
    info!("Found {} image frames for processing", total_frames);

    Ok((output_directory.to_path_buf(), frames, total_frames))
}

/// Handles temporary directories based on build mode.
///
/// This function manages the cleanup or retention of temporary directories.
///
/// # Parameters
/// - =tmp_dir=: An optional reference to a =PathBuf= representing the temporary directory.
///
/// # Returns
/// - =Result<()>=: Indicates success or failure of the operation.
///
/// # Notes
/// - In release mode, the function removes the directory and logs the action.
/// - In debug mode, the directory is retained and a message is logged instead.
fn rm_tmp_dir(tmp_dir: Option<&PathBuf>) -> Result<()> {
    if let Some(tmp_dir) = tmp_dir {
        #[cfg(not(debug_assertions))]
        {
            fs::remove_dir_all(tmp_dir.as_path())
                .context("Failed to delete temporary directory")?;
            debug!("Temporary directory removed in release mode.");
        }

        #[cfg(debug_assertions)]
        {
            debug!(
                "Temporary directory retained for debugging: {:?}",
                tmp_dir.as_path()
            );
        }
    }
    Ok(())
}
