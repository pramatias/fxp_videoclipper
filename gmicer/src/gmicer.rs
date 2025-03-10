use anyhow::{Context, Result};
use log::{debug, error};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use modes::Modes;
use output::ModeOutput;
use output::Output;

use crate::image::image_processing;
use filenames::FilenameValidator;
use filenames::ImageMappingError;
use filenames::SimpleValidator;

pub struct Gmicer {
    input_path: PathBuf,
    gmic_args: Vec<String>,
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
                gmicer_output.create_output((
                    input_path.clone(),
                    gmic_args.clone(), // Pass gmic_args here
                    output_directory.map(String::from),
                ))?
            }
            _ => unreachable!("Expected Gmicer mode"),
        };

        let (images, _padding) = setup_gmic_processing(input_directory)?;

        Ok(Self {
            input_path,
            gmic_args,
            output_path: output_path_buf,
            images,
        })
    }
}

fn setup_gmic_processing(
    input_directory: &str,
) -> Result<(BTreeMap<u32, PathBuf>, usize)> {
    debug!("Starting setup_gmic_processing function");

    let dir_path = Path::new(input_directory);
    debug!("Input directory path: {:?}", dir_path);

    // Read all image paths from the input directory.
    let images: Vec<PathBuf> = fs::read_dir(dir_path)
        .context("Failed to read input directory")?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .collect();
    debug!("Found {} images in input directory", images.len());

    // Initialize the SimpleValidator.
    let simple_validator = SimpleValidator {};
    debug!("SimpleValidator initialized");

    // Validate and fix image filenames using the simple validator.
    debug!("Validating and fixing image filenames");
    let image_map = simple_validator
        .validate_and_fix_image_filenames(&images)
        .map_err(|e| ImageMappingError::CopyRenameError(e.to_string()))?;
    debug!("Total images after validation: {}", image_map.len());

    Ok((image_map.clone(), image_map.len()))
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

        Ok(())
    }
}

