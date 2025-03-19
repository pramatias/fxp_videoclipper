use anyhow::{Context, Result};
use log::debug;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use modes::Modes;
use output::ModeOutput;
use output::Output;

use crate::clut::clut_all_images;

use filenames::FileOperations;

/// Struct responsible for applying CLUT (Color Look-Up Table) to images in a directory.
pub struct Clutter {
    input_directory: PathBuf,
    clut_image: PathBuf,
    input_files: BTreeMap<u32, PathBuf>,
    output_directory: PathBuf,
}

impl Clutter {
    pub fn new(
        input_directory: String,
        clut_image: String,
        output_directory: Option<String>,
    ) -> Result<Self> {
        debug!("Initializing new Clutter instance with:");
        debug!("- Input directory: {}", input_directory);
        debug!("- CLUT image: {}", clut_image);
        debug!("- Output directory: {:?}", output_directory);

        // Process input directory: convert, check and canonicalize.
        let input_directory_path = PathBuf::from(&input_directory);
        if !input_directory_path.is_dir() {
            anyhow::bail!(
                "Input directory '{}' does not exist or is not a directory",
                input_directory_path.display()
            );
        }
        let input_directory_path = fs::canonicalize(&input_directory_path).with_context(|| {
            format!(
                "Failed to resolve input directory '{}'",
                input_directory_path.display()
            )
        })?;
        debug!("Canonicalized input directory: {:?}", input_directory_path);

        // Process CLUT image: convert, check and canonicalize.
        let clut_image_path = PathBuf::from(&clut_image);
        if !clut_image_path.is_file() {
            anyhow::bail!(
                "CLUT image '{}' does not exist or is not a file",
                clut_image_path.display()
            );
        }
        let clut_image_path = fs::canonicalize(&clut_image_path).with_context(|| {
            format!(
                "Failed to resolve CLUT image '{}'",
                clut_image_path.display()
            )
        })?;
        debug!("Canonicalized CLUT image: {:?}", clut_image_path);

        // Create output directory using the appropriate handler.
        debug!("Creating output directory...");
        let mode: Modes = Modes::Clutter;
        debug!("Using mode: {:?}", mode);
        let output: Output = mode.into();
        let output_directory_path = match output {
            Output::Clutter(clutter_output) => {
                debug!("Using Clutter output handler to create directory");
                let path = clutter_output
                    .create_output((input_directory_path.clone(), output_directory))?;
                debug!("Output directory created at: {:?}", path);
                path
            }
            _ => {
                debug!("Unexpected output variant encountered!");
                unreachable!("Expected Clutter mode")
            }
        };

        // Run setup_clut_processing to populate input_files.
        let input_directory_str = input_directory_path.to_str().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid input directory path",
            )
        })?;
        let input_files = setup_clut_processing(input_directory_str)?;
        debug!("Found {} input files for processing", input_files.len());

        debug!("Successfully initialized Clutter instance:");
        debug!("- Final input directory: {:?}", input_directory_path);
        debug!("- Final CLUT image path: {:?}", clut_image_path);
        debug!("- Output directory: {:?}", output_directory_path);

        Ok(Self {
            input_directory: input_directory_path,
            clut_image: clut_image_path,
            input_files,
            output_directory: output_directory_path,
        })
    }
}

/// Sets up CLUT (Color LookUp Table) processing by preparing input images and directories.
///
/// This function initializes the necessary directories and processes image files
/// for CLUT application. It includes temporary directory creation, image validation,
/// and output directory setup.
///
/// # Parameters
/// - `input_directory`: Path to the directory containing input images to be processed
///
/// # Returns
/// - `Result<(BTreeMap<u32, String>, String)>`: A tuple containing:
///   - A BTreeMap of input files mapped by number
///   - The path to the CLUT output directory
///
/// # Notes
/// - Creates a temporary directory for image processing
/// - Validates and corrects image filenames before processing
/// - Creates an output directory for CLUT-applied images
fn setup_clut_processing(input_directory: &str) -> Result<BTreeMap<u32, PathBuf>> {
    let input_path = Path::new(input_directory);

    // Read input images from the directory.
    let input_images: Vec<PathBuf> = fs::read_dir(input_path)?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .collect();

    let mode = Modes::Clutter;
    let validated_input_images = mode.load_files(&input_images)?;

    Ok(validated_input_images)
}

impl Clutter {
    /// Applies a Color Lookup Table (CLUT) to a set of images.
    ///
    /// This function processes images by applying a CLUT transformation using a source image
    /// as reference, creating new formatted images in a dedicated output directory.
    ///
    /// # Parameters
    /// - None
    ///
    /// # Returns
    /// - `Result<String>`: Path to the directory containing the processed CLUT images.
    ///
    /// # Notes
    /// - Creates a new directory for CLUT-processed images if it doesn't exist.
    /// - Processes all images in the input directory using the specified CLUT.
    /// - Returns an error if image processing fails.
    pub fn create_clut_images(&self) -> Result<String> {
        debug!(
            "Applying CLUT from source image '{}' to images in directory '{}'",
            self.clut_image.display(),
            self.input_directory.display()
        );

        // Now that `input_files` has been populated in `new()`, simply use it.
        clut_all_images(&self.clut_image, &self.input_files, &self.output_directory)?;

        Ok(self.output_directory.to_string_lossy().into_owned())
    }
}
