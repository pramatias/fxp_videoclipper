use anyhow::anyhow;
use anyhow::{Context, Result};
use log::debug;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use crate::merge::merge_all_images;
// use crate::output::create_output_directory;

use modes::Modes;
// use output::MergerOutput;
use output::ModeOutput;
use output::Output;

use filenames::FilenameValidator;
use filenames::SimpleValidator;

pub struct Merger {
    opacity: f32,
    directory1: PathBuf,
    directory2: PathBuf,
    directory1_files: BTreeMap<u32, PathBuf>,
    directory2_files: BTreeMap<u32, PathBuf>,
    output_directory: PathBuf,
    total_images: usize,
}

impl Merger {
    /// Creates a new instance of `Merger` and sets up the image processing environment.
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
                merger_output.create_output_directory((
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
            directory1: directory1_path,
            directory2: directory2_path,
            directory1_files,
            directory2_files,
            output_directory: output_directory_path,
            total_images,
        })
    }
}

impl Merger {
    /// Merges images from two directories using the provided opacity and returns the output directory or an error.
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

/// Prepares image processing by validating filenames and organizing directory paths.
///
/// This function processes two directories of images, ensuring filenames are correctly formatted.
/// It prepares the necessary paths and parameters for subsequent image operations.
///
/// # Parameters
/// - `directory1`: Path to the first directory containing images.
/// - `directory2`: Path to the second directory containing images.
/// - `opacity`: The opacity level used for image processing.
/// - `output_directory`: Optional path for output; if none, a default is created.
///
/// # Returns
/// - `Result<(...)>`: A tuple containing:
///   - `BTreeMap<u32, PathBuf>`: Validated image paths for directory1.
///   - `BTreeMap<u32, PathBuf>`: Validated image paths for directory2.
///   - `String`: Output directory path.
///   - `usize`: Total number of images to process.
///   - `usize`: Padding value for image numbering.
///
/// # Notes
/// - The output directory is created if not provided.
/// - Filenames are validated and corrected automatically.
fn setup_image_processing(
    directory1: PathBuf,
    directory2: PathBuf,
) -> Result<(BTreeMap<u32, PathBuf>, BTreeMap<u32, PathBuf>, usize)> {
    // Debug: Print the input directories
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

    let validator = SimpleValidator;

    // Debug: Print a message before validating and fixing image filenames
    debug!("Validating and fixing image filenames in directory1");
    let validated_dir1_images = validator.validate_and_fix_image_filenames(&dir1_images)?;
    debug!("Validating and fixing image filenames in directory2");
    let validated_dir2_images = validator.validate_and_fix_image_filenames(&dir2_images)?;

    // Debug: Print the number of validated images in each directory
    debug!(
        "Validated {} images in directory1",
        validated_dir1_images.len()
    );
    debug!(
        "Validated {} images in directory2",
        validated_dir2_images.len()
    );

    let total_images = std::cmp::min(validated_dir1_images.len(), validated_dir2_images.len());

    // Debug: Print the total number of images to be processed
    debug!("Total images to be processed: {}", total_images);

    Ok((validated_dir1_images, validated_dir2_images, total_images))
}
