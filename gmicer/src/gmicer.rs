use anyhow::{Context, Result};
use log::{debug, error};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use tempfile::TempDir;


use modes::Modes;
use output::ModeOutput;
use output::Output;

use crate::image::image_processing;
use filenames::FilenameValidator;
use filenames::ImageMappingError;
use filenames::TempDirValidator;

pub struct Gmicer {
    input_path: PathBuf,
    gmic_args: Vec<String>,
    tmp_dir: Option<TempDir>,
    output_path: PathBuf,
    images: BTreeMap<u32, PathBuf>,
}

impl Gmicer {
    pub fn new(
        input_directory: &str,
        output_directory: Option<&str>,
        gmic_args: Vec<String>,
    ) -> Result<Self> {
        let input_path = PathBuf::from(input_directory);

        // Create the output directory via the ModeOutput trait:
        let mode: Modes = Modes::Gmicer;
        let output: Output = mode.into();
        let output_path_buf = match output {
            Output::Gmicer(gmicer_output) => {
                gmicer_output.create_output_directory((
                    input_path.clone(),
                    output_directory.map(String::from),
                ))?
            }
            _ => unreachable!("Expected Gmicer mode"),
        };

        let (tmp_dir, images, _padding) = setup_gmic_processing(input_directory)?;

        Ok(Self {
            input_path,
            gmic_args,
            tmp_dir: Some(tmp_dir),
            output_path: output_path_buf,
            images,
        })
    }
}

/// Sets up the processing pipeline for GMIC image processing.
///
/// This function initializes the temporary directory and validates/fixes image filenames
/// found in the given input directory.
///
/// # Parameters
/// - `input_directory`: Path to the directory containing input images.
/// - `gmic_args`: Vector of GMIC arguments for processing.
///
/// # Returns
/// - `Result<(TempDir, image_map, image_count)>`
///   - `TempDir`: Temporary directory for processing.
///   - `image_map`: Mapping of image IDs to PathBuf.
///   - `image_count`: Total number of images processed.
fn setup_gmic_processing(
    input_directory: &str,
) -> Result<(TempDir, BTreeMap<u32, PathBuf>, usize)> {
    debug!("Starting setup_gmic_processing function");

    let dir_path = Path::new(input_directory);
    debug!("Input directory path: {:?}", dir_path);

    let tmp_dir = TempDir::new()?;
    let tmp_dir_path = tmp_dir.path().to_path_buf();
    debug!("Temporary directory created at: {:?}", tmp_dir_path);

    let tmp_dir_validator = TempDirValidator::new(&tmp_dir);
    debug!("TempDirValidator initialized");

    let images: Vec<PathBuf> = fs::read_dir(dir_path)
        .context("Failed to read input directory")?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .collect();
    debug!("Found {} images in input directory", images.len());

    debug!("Validating and fixing image filenames");
    let images = tmp_dir_validator
        .validate_and_fix_image_filenames(&images)
        .map_err(|e| ImageMappingError::CopyRenameError(e.to_string()))?;
    debug!("Total images after validation: {}", images.len());

    Ok((tmp_dir, images.clone(), images.len()))
}

impl Gmicer {
    /// Processes images using a GMIC command.
    ///
    /// This function handles image processing by executing a GMIC command on the specified images.
    ///
    /// # Parameters
    /// - `self`: Contains configuration like input directory, GMIC arguments, and output directory.
    ///
    /// # Returns
    /// - `Result<()>`: Indicates whether the image processing was successful.
    ///
    /// # Notes
    /// - Logs processing details at the start of the operation.
    /// - Checks and handles cases where no images are found.
    /// - Manages temporary directories based on the build mode.
    pub fn gmic_images(&self) -> Result<()> {
        debug!(
            "Processing images from '{}' with GMIC arguments: {:?}",
            self.input_path.display(), // Use `.display()` for proper formatting.
            self.gmic_args
        );

        if self.images.is_empty() {
            error!("No images found in the input directory.");
            return Ok(());
        }

        image_processing(&self.images, &self.gmic_args, &self.output_path)
            .context("Failed to process images")?;

        // Handle temporary directories based on build mode.
        rm_tmp_dir(
            self.tmp_dir
                .as_ref()
                .map(|tmp| tmp.path().to_path_buf())
                .as_ref(),
        )?;

        Ok(())
    }
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
            fs::remove_dir_all(tmp_dir.path()).context("Failed to delete temporary directory")?;
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
