use anyhow::{Context, Result};
use console::style;
use log::{debug, error};
use std::collections::BTreeMap;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use modes::Modes;
use output::ModeOutput;
use output::Output;

use crate::image::image_processing;
use filenames::FileOperations;
use filenames::ImageMappingError;

pub struct Gmicer {
    input_path: PathBuf,
    gmic_args: Vec<String>,
    output_path: PathBuf,
    images: BTreeMap<u32, PathBuf>,
}

impl Gmicer {
    /// Creates a new instance of `Gmicer` for processing images using GMIC.
    ///
    /// This function initializes the necessary components for image processing,
    /// including input and output directories, and GMIC arguments.
    ///
    /// # Parameters
    /// - `input_directory`: The path to the directory containing input images.
    /// - `output_directory`: Optional path for output images; defaults to input directory if not provided.
    /// - `gmic_args`: Vector of GMIC arguments to apply during processing.
    ///
    /// # Returns
    /// - `Result<Self>`: Returns a new `Gmicer` instance on success, or an error if initialization fails.
    ///
    /// # Notes
    /// - If `output_directory` is not provided, output files will be placed in the input directory.
    /// - The function validates the input directory and GMIC arguments before initializing.
    pub fn new(
        input_directory: &str,
        output_directory: Option<&str>,
        gmic_args: Vec<String>,
    ) -> Result<Self> {
        debug!("Initializing new Gmicer instance");
        debug!("Input directory: {}", input_directory);
        debug!("Output directory: {:?}", output_directory);
        debug!("GMIC arguments: {:?}", gmic_args);

        let input_path = PathBuf::from(input_directory);
        debug!("Created input PathBuf: {:?}", input_path);

        // Create the output directory via the ModeOutput trait:
        let mode: Modes = Modes::Gmicer;
        debug!("Using mode: {:?}", mode);

        let output: Output = mode.into();
        let output_path_buf = match output {
            Output::Gmicer(gmicer_output) => {
                debug!("Creating GMICer output directory");
                let path = gmicer_output.create_output((
                    input_path.clone(),
                    gmic_args.clone(),
                    output_directory.map(String::from),
                ))?;
                debug!("Output directory created at: {:?}", path);
                path
            }
            _ => {
                debug!("Unexpected output type encountered!");
                unreachable!("Expected Gmicer mode")
            }
        };

        debug!(
            "Setting up GMIC processing for directory: {}",
            input_directory
        );
        let (images, padding) = setup_gmic_processing(input_directory)?;
        debug!("Found {} images with padding: {}", images.len(), padding);

        let gmicer = Self {
            input_path: input_path.clone(),
            gmic_args: gmic_args.clone(),
            output_path: output_path_buf.clone(),
            images: images.clone(),
        };

        debug!("Successfully created Gmicer instance:");
        debug!("- Input path: {:?}", gmicer.input_path);
        debug!("- Output path: {:?}", gmicer.output_path);
        debug!("- Number of images: {}", gmicer.images.len());
        debug!("- GMIC arguments: {:?}", gmicer.gmic_args);

        Ok(gmicer)
    }
}

/// Sets up and processes G'MIC image files from a specified directory.
///
/// This function reads image files from the input directory, processes them,
/// and returns a mapped collection of the images along with their count.
///
/// # Parameters
/// - `input_directory`: The file path to the directory containing G'MIC images to process.
///
/// # Returns
/// - `Result<(BTreeMap<u32, PathBuf>, usize)>`: A tuple containing:
///   - A `BTreeMap` mapping image IDs to their file paths.
///   - The total number of images processed.
///
/// # Notes
/// - The function reads all image files from the specified directory.
/// - Uses `FileOperations` for processing images in "Gmicer" mode.
fn setup_gmic_processing(input_directory: &str) -> Result<(BTreeMap<u32, PathBuf>, usize)> {
    debug!("Starting setup_gmic_processing function");

    let dir_path = Path::new(input_directory);
    debug!("Input directory path: {:?}", dir_path);

    // Read all image paths from the input directory.
    let images: Vec<PathBuf> = fs::read_dir(dir_path)
        .context("Failed to read input directory")?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .collect();
    debug!("Found {} images in input directory", images.len());

    // Use FileOperations implemented for Modes::Clipper to process images.
    debug!("Loading files using FileOperations for Clipper mode");
    let image_map = Modes::Gmicer
        .load_files(&images)
        .map_err(|e| ImageMappingError::RenameError(e.to_string()))?;
    debug!("Total images after processing: {}", image_map.len());

    Ok((image_map.clone(), image_map.len()))
}

impl Gmicer {
    /// Processes images using GMIC arguments and handles output warnings.
    ///
    /// This function manages the image processing pipeline, including logging,
    /// image handling, and output warnings.
    ///
    /// # Parameters
    /// - `&self`: Reference to the current instance containing processing data
    ///
    /// # Returns
    /// - `Result<()>`: Indicates success or failure of the image processing operation
    ///
    /// # Notes
    /// - Logs debug and error messages for visibility into processing flow
    /// - Processes images with GMIC arguments and handles output directory warnings
    /// - Returns early with success if no images are found
    pub fn gmic_images(&self) -> Result<()> {
        debug!(
            "Processing images from '{}' with GMIC arguments: {:?}",
            self.input_path.display(),
            self.gmic_args
        );

        if self.images.is_empty() {
            error!("No images found in the input directory.");
            return Ok(());
        }

        image_processing(&self.images, &self.gmic_args, &self.output_path)
            .context("Failed to process images")?;

        warn_on_multiple_image_output(&self.output_path)
            .context("Failed to warn on multiple image output")?;

        Ok(())
    }
}

/// Warns about multiple image output files with the same numeric identifier.
///
/// This function checks for image files in the specified directory that follow a specific
/// naming pattern. It identifies files with multiple outputs that share the same numeric
/// segment and issues a warning.
///
/// # Parameters
/// - `output_path`: The directory path to check for image files.
///
/// # Returns
/// - `Result<()>`: Indicates whether the operation completed successfully.
///
/// # Notes
/// - The function scans filenames for a format containing numeric identifiers between
///   the second underscore and the file extension.
/// - It collects unique numeric identifiers and warns if multiple files share the
///   same identifier, indicating potential conflicts.
/// - Files that do not match the expected pattern are ignored.
fn warn_on_multiple_image_output(output_path: &Path) -> Result<()> {
    let mut unique_numbers = HashSet::new();

    // Iterate over all entries in the directory
    for entry in fs::read_dir(output_path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
                // Find all underscore positions in the filename.
                let underscore_positions: Vec<_> = filename.match_indices('_').collect();

                // Ensure there is a second underscore.
                if underscore_positions.len() >= 2 {
                    // Get the index of the second underscore.
                    let second_uscore_idx = underscore_positions[1].0;

                    // Find the position of the last dot to exclude the file extension.
                    if let Some(dot_idx) = filename.rfind('.') {
                        // Extract the substring after the second underscore until the dot.
                        let candidate = &filename[second_uscore_idx + 1..dot_idx];

                        // Check if the candidate consists only of digits.
                        if !candidate.is_empty() && candidate.chars().all(|c| c.is_ascii_digit()) {
                            unique_numbers.insert(candidate.to_string());
                        }
                    }
                }
            }
        }
    }

    // Convert the HashSet into a sorted Vec.
    let mut unique_list: Vec<_> = unique_numbers.into_iter().collect();
    unique_list.sort();

    // Log the warning message with the unique numbers.
    display_warn_message(&unique_list);

    Ok(())
}

/// Displays a warning message listing unique numbers found in output files.
///
/// This function prints a styled message to the console when unique numbers are detected.
///
/// # Parameters
/// - `numbers`: A slice of `String` containing the unique numbers to display.
///
/// # Notes
/// - If the `numbers` slice is empty, no message will be displayed.
pub fn display_warn_message(numbers: &[String]) {
    if !numbers.is_empty() {
        println!(
            "{}",
            style(format!(
                "Unique numbers found in output files: {}",
                numbers.join(", ")
            ))
            .yellow()
        );
    }
}
