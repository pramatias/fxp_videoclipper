use anyhow::{Context, Result};
use log::debug;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use crate::merge::merge_all_images;

use fxp_modes::Modes;
use fxp_output::ModeOutput;
use fxp_output::Output;

use fxp_filenames::FileOperations;

pub struct Merger {
    opacity: f32,
    directory1_files: BTreeMap<u32, PathBuf>,
    directory2_files: BTreeMap<u32, PathBuf>,
    output_directory: PathBuf,
    total_images: usize,
}

impl Merger {
    /// Creates a new `Merger` instance and initializes the image processing environment.
    ///
    /// This function sets up the necessary file paths and parameters for merging images.
    ///
    /// # Parameters
    /// - `directory1`: The first directory containing images to process.
    /// - `directory2`: The second directory containing images to process.
    /// - `opacity`: The opacity value used for image merging (0.0 to 1.0).
    /// - `output_directory`: Optional output directory for the merged images.
    ///
    /// # Returns
    /// - `Result<Self>`: A new `Merger` instance or an error if initialization fails.
    ///
    /// # Notes
    /// - If `output_directory` is not provided, a default location is used.
    /// - The function validates and prepares image files from both input directories.
    /// - Image processing is configured with the specified opacity value.
    pub fn new(
        directory1: String,
        directory2: String,
        opacity: f32,
        output_directory: Option<String>,
    ) -> Result<Self> {
        // Convert directory strings into PathBufs.
        let directory1_path = PathBuf::from(&directory1);
        let directory2_path = PathBuf::from(&directory2);

        let mode: Modes = Modes::Merger;
        let output: Output = mode.into();

        let output_directory_path = match output {
            Output::Merger(merger_output) => {
                merger_output.create_output((
                    directory1_path.clone(), // using directory1 as base
                    output_directory,
                    opacity,
                ))?
            }
            _ => unreachable!("Expected Merger mode"),
        };

        // Set up image processing (assuming this no longer returns an output directory).
        let (directory1_files, directory2_files, total_images) =
            setup_image_processing(directory1_path.clone(), directory2_path.clone())?;

        Ok(Self {
            opacity,
            directory1_files,
            directory2_files,
            output_directory: output_directory_path,
            total_images,
        })
    }
}

impl Merger {
    /// Merges images from two directories using specified opacity and returns the output directory or an error.
    ///
    /// This function combines images from two directories, applies the given opacity, and saves the merged results to the output directory.
    ///
    /// # Parameters
    /// - `directory1_files`: The first directory containing image files to merge.
    /// - `directory2_files`: The second directory containing image files to merge.
    /// - `output_directory`: The directory where merged images will be saved.
    /// - `opacity`: The opacity level applied during the merging process.
    /// - `total_images`: The total number of images to be merged.
    ///
    /// # Returns
    /// - `Result<PathBuf>`: The path to the output directory on success, or an error if merging fails.
    ///
    /// # Notes
    /// - The function provides contextual error information if the merging process fails.
    pub fn merge_images(&self) -> Result<PathBuf> {
        merge_all_images(
            &self.directory1_files,
            &self.directory2_files,
            &self.output_directory,
            self.opacity,
            self.total_images,
        )
        .with_context(|| "Error merging images")?;

        Ok(self.output_directory.clone())
    }
}

/// Sets up image processing by reading, validating, and preparing images from two directories.
///
/// This function reads image files from two specified directories, validates them,
/// and prepares them for further processing.
///
/// # Parameters
/// - `directory1`: Path to the first directory containing images to process.
/// - `directory2`: Path to the second directory containing images to process.
///
/// # Returns
/// - `Result<(BTreeMap<u32, PathBuf>, BTreeMap<u32, PathBuf>, usize)>`:
///   - A tuple containing two maps of validated image paths (one for each directory)
///     and the total number of images to be processed.
///
/// # Notes
/// - Only processes images present in both directories.
/// - Uses the `FileOperations` trait for loading and validating image files.
/// - Logs debug information about the processing steps and image counts.
fn setup_image_processing(
    directory1: PathBuf,
    directory2: PathBuf,
) -> Result<(BTreeMap<u32, PathBuf>, BTreeMap<u32, PathBuf>, usize)> {
    debug!("Reading images from directory1: {:?}", directory1);
    debug!("Reading images from directory2: {:?}", directory2);

    let dir1_images: Vec<PathBuf> = fs::read_dir(&directory1)?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .collect();
    let dir2_images: Vec<PathBuf> = fs::read_dir(&directory2)?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .collect();

    // Debug: Print the number of images found in each directory
    debug!("Found {} images in directory1", dir1_images.len());
    debug!("Found {} images in directory2", dir2_images.len());

    let mode = Modes::Merger;

    // Debug: Load and validate files using FileOperations trait.
    debug!("Loading files for directory1 using FileOperations");
    let validated_dir1_images = mode.load_files(&dir1_images)?;
    debug!("Loading files for directory2 using FileOperations");
    let validated_dir2_images = mode.load_files(&dir2_images)?;

    // Debug: Print the number of validated images in each directory.
    debug!(
        "Validated {} images in directory1",
        validated_dir1_images.len()
    );
    debug!(
        "Validated {} images in directory2",
        validated_dir2_images.len()
    );

    // Calculate the total images to be processed.
    let total_images = std::cmp::min(validated_dir1_images.len(), validated_dir2_images.len());
    debug!("Total images to be processed: {}", total_images);

    Ok((validated_dir1_images, validated_dir2_images, total_images))
}
