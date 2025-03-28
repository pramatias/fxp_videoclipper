use anyhow::{anyhow, Context, Result};
use ctrlc;
use log::debug;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tempfile;

use fxp_modes::Modes;
use fxp_output::ModeOutput;
use fxp_output::Output;

use crate::clip::make_clip;

use fxp_filenames::FileOperations;
use fxp_filenames::ImageMappingError;

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
    /// Clips and processes a video file, handling interruptions gracefully.
///
/// This function manages the video clipping process, including temporary file handling
/// and cleanup. It also supports Ctrl-C interruption and debug logging.
///
/// # Parameters
/// - `images`: A slice of `PathBuf` objects representing the image files to process.
///
/// # Returns
/// - `Result<PathBuf>`: The path to the final clipped video file on success.
///
/// # Notes
/// - Creates a temporary directory for processing.
/// - Handles Ctrl-C interruptions by setting a running flag.
/// - Copies temporary directory contents to a debug directory in debug builds.
    pub fn clip(&self) -> Result<PathBuf> {
        debug!("Starting video clipping process...");

        // Create a temporary directory using the tempfile crate.
        let tmp_dir = tempfile::tempdir().context("Failed to create temporary directory")?;
        let tmp_dir_path = tmp_dir.path().to_path_buf();

        // Set up the running flag and register a Ctrl-C handler.
        let running = Arc::new(AtomicBool::new(false));
        let running_clone = running.clone();
        ctrlc::set_handler(move || {
            running_clone.store(true, Ordering::Relaxed);
        })
        .expect("Error setting Ctrl-C handler");

        // Process video using the extracted function.
        let final_video_path = make_clip(
            &self.input_dir,
            &self.output_path,
            self.mp3_path.as_deref(), // converts Option<PathBuf> to Option<&Path>
            self.fps,
            self.duration,
            running.clone(),
            &tmp_dir_path,
        )?;

        #[cfg(debug_assertions)]
        {
            let debug_dir = PathBuf::from("/tmp/fxp_videoclipper");
            copy_tmp_dir_contents(tmp_dir.path(), &debug_dir)?;
        }

        debug!(
            "Video clipping process completed successfully. Final video saved at: {:?}",
            final_video_path
        );

        Ok(final_video_path)
    }
}

impl Clipper {
    /// Creates a new Clipper instance for processing image and audio files.
    ///
    /// This function initializes a Clipper instance by validating and preparing input/output paths
    /// and configurations.
    ///
    /// # Parameters
    /// - `input_dir`: Path to the input directory containing image files (required).
    /// - `mp3_path`: Optional path to an MP3 audio file for video creation.
    /// - `output_path`: Optional custom output directory path. If not provided, a default directory
    ///   will be created inside the input directory.
    /// - `fps`: Frames per second for the output video (must be > 0).
    /// - `duration`: Optional duration in milliseconds for the video.
    ///
    /// # Returns
    /// - `Result<Self>`: A new Clipper instance on success, or an error if validation fails.
    ///
    /// # Notes
    /// - The input directory must exist and contain image files.
    /// - If an MP3 file is provided, it must exist and be a file.
    /// - The output directory will be created if it doesn't exist.
    /// - All validation errors return detailed error messages.
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

/// Sets up and prepares image files for Clipper processing.
///
/// This function reads an input directory, validates the image files, and prepares them for processing.
///
/// # Parameters
/// - `input_directory`: Path to the directory containing the input image files.
/// - `output_directory`: Path to the directory where processed files will be output.
///
/// # Returns
/// - `Result<(PathBuf, BTreeMap<u32, PathBuf>, usize)>`:
///   - `PathBuf`: Output directory path.
///   - `BTreeMap<u32, PathBuf>`: Mapping of frame IDs to their paths.
///   - `usize`: Total number of frames.
///
/// # Notes
/// - Returns an error if the input directory contains no valid image frames.
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
    debug!("Found {} image frames for processing", total_frames);

    Ok((output_directory.to_path_buf(), frames, total_frames))
}

/// Copies the contents of a temporary directory to a debug directory for inspection.
///
/// This function is used to duplicate files from a temporary directory into a debug directory,
/// which is useful for debugging purposes.
///
/// # Parameters
/// - `tmp_dir`: The path to the temporary directory containing files to copy.
/// - `debug_dir`: The path where the files will be copied for debugging.
///
/// # Returns
/// - `Result<()>`: Returns `Ok(())` if successful. Returns an error if the source directory
///   cannot be read or if there's an issue during copying.
///
/// # Notes
/// - This function is only available when `debug_assertions` are enabled.
/// - If the destination directory does not exist, it will be created before copying files.
/// - Each file copy operation will return an error if it fails, preventing further copies.
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
