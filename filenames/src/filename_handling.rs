use anyhow::{Context, Result};
use log::{debug, warn};
use regex::Regex;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ImageMappingError {
    #[error("Duplicate numerical identifier {0} found in files: {1:?} and {2:?}")]
    DuplicateIdentifier(u32, PathBuf, PathBuf),
    #[error("Failed to copy or rename images: {0}")]
    CopyRenameError(String),
}

impl FilenameValidator for TempDirValidator {
    /// Validates and corrects image filenames, ensuring proper formatting and structure.
    ///
    /// This function checks image filenames for consistency, corrects them if necessary,
    /// and maps them by numerical identifiers for organized access.
    ///
    /// # Parameters
    /// - `images`: A slice of `PathBuf` objects representing the image files to validate.
    /// - `&self`: Reference to the current instance.
    ///
    /// # Returns
    /// - `Result<BTreeMap<u32, PathBuf>, ImageMappingError>`: A map of image files keyed by their numerical identifiers
    ///   or an error if the operation fails.
    ///
    /// # Notes
    /// - If filenames are already correctly formatted, they are returned unchanged.
    /// - Malformed filenames are corrected by copying and renaming the files.
    /// - The returned `BTreeMap` maintains files sorted by their numerical identifiers for predictable access.
    fn validate_and_fix_image_filenames(
        &self,
        images: &[PathBuf],
    ) -> Result<BTreeMap<u32, PathBuf>, ImageMappingError> {
        debug!("Starting validation of image filenames...");

        let validated_images = if !has_malformed_images(images) {
            debug!("All images are correctly named. No need to copy or rename.");
            images.to_vec()
        } else {
            debug!("Malformed images detected. Preparing to copy and rename...");
            self.copy_and_rename_images(images, &self.tmp_dir_path)
                .map_err(|e| ImageMappingError::CopyRenameError(e.to_string()))?
        };

        debug!("Mapping files by numerical identifiers...");
        map_files_by_number(validated_images)
    }
}

// Trait definition
pub trait FilenameValidator {
    fn validate_and_fix_image_filenames(
        &self,
        images: &[PathBuf],
    ) -> Result<BTreeMap<u32, PathBuf>, ImageMappingError>;
}

// Validator that creates a temporary directory
pub struct TempDirValidator {
    tmp_dir_path: PathBuf,
}

impl TempDirValidator {
    pub fn new(tmp_dir: &TempDir) -> Self {
        Self {
            tmp_dir_path: tmp_dir.path().to_path_buf(),
        }
    }
}

// Validator that does not create a temporary directory
pub struct SimpleValidator;

impl FilenameValidator for SimpleValidator {
    fn validate_and_fix_image_filenames(
        &self,
        images: &[PathBuf],
    ) -> Result<BTreeMap<u32, PathBuf>, ImageMappingError> {
        debug!("Starting validation of image filenames...");

        map_files_by_number(images.to_vec())
    }
}

impl TempDirValidator {
    /// Copies and renames image files to a standardized format in a temporary directory.
    ///
    /// Processes each image file to ensure it follows the `image_{number}.{extension}` format,
    /// where `number` is a zero-padded four-digit number. Copies files to the specified temporary directory.
    ///
    /// # Parameters
    /// - `images`: A slice of `PathBuf` representing the image files to process.
    /// - `tmp_dir_path`: The path to the temporary directory for copying files.
    ///
    /// # Returns
    /// - `Result<Vec<PathBuf>>`: A vector of `PathBuf` pointing to the renamed image files,
    /// or an error if copying fails.
    ///
    /// # Notes
    /// - Files already matching the expected format are copied without renaming.
    /// - Filenames with additional numbers are logged as improperly named.
    /// - All processing steps are logged for debugging purposes.
    fn copy_and_rename_images(
        &self,
        images: &[PathBuf],
        tmp_dir_path: &Path,
    ) -> Result<Vec<PathBuf>> {
        let mut updated_images = Vec::new();
        let mut improperly_named_files = Vec::new();

        // Log the start of the process
        debug!("Starting to copy and rename images");

        for image in images {
            // Log each image being processed
            debug!("Processing image: {:?}", image.display());

            if let Some(filename) = image.file_stem().and_then(|s| s.to_str()) {
                if let Some(number) = extract_correct_number(filename) {
                    let padded_name = format!("image_{:04}", number);
                    let new_filename = format!(
                        "{}.{}",
                        padded_name,
                        image.extension().and_then(|ext| ext.to_str()).unwrap_or("")
                    );
                    let new_path = tmp_dir_path.join(&new_filename);

                    // Log when a file is deemed improperly named
                    if is_malformed(filename) {
                        debug!("Found improperly named file: {}", filename);
                        improperly_named_files.push(filename.to_string());
                    }

                    if filename != padded_name {
                        // Log before performing the copy operation
                        debug!(
                            "Copying and renaming {} to {}",
                            image.display(),
                            new_path.display()
                        );

                        fs::copy(image, &new_path).with_context(|| {
                            format!("Failed to copy and rename file: {:?}", image)
                        })?;
                        updated_images.push(new_path);
                        continue;
                    }
                }
            }

            // Log when using the original filename
            debug!(
                "Using original filename for: {}",
                image.file_name().unwrap().to_str().unwrap()
            );
            let new_path = tmp_dir_path.join(image.file_name().unwrap());
            fs::copy(image, &new_path)
                .with_context(|| format!("Failed to copy file: {:?}", image))?;
            updated_images.push(new_path);
        }

        // Log any improperly named files
        if !improperly_named_files.is_empty() {
            warn!("Found improperly named files: {:?}", improperly_named_files);
        }

        // Log the completion of the process
        debug!("Finished copying and rename images");
        Ok(updated_images)
    }
}

/// Maps image files to a structured format based on extracted numbers.
///
/// This function processes image files to create a mapping of numeric identifiers
/// to their corresponding file paths. It corrects filenames to a standard format and
/// handles duplicates.
///
/// # Parameters
/// - `files`: A vector of `PathBuf` objects representing image file paths.
///
/// # Returns
/// - `Result<BTreeMap<u32, PathBuf>, ImageMappingError>`: A sorted map of numeric IDs to
///   corrected file paths, or an error if duplicates are found.
///
/// # Notes
/// - Filenames are corrected to the format `image_{number}.{extension}`.
/// - If duplicate numeric identifiers are detected, an error is returned.
fn map_files_by_number(files: Vec<PathBuf>) -> Result<BTreeMap<u32, PathBuf>, ImageMappingError> {
    debug!("Starting map_files_by_number function");

    let mut map: BTreeMap<u32, PathBuf> = BTreeMap::new();
    debug!("Created an empty BTreeMap to store file mappings");

    for file in files {
        debug!("Processing file: {:?}", file);

        if let Some(filename) = file.file_stem().and_then(|f| f.to_str()) {
            debug!("Found filename: {}", filename);

            if is_malformed(filename) {
                debug!("Filename is malformed, attempting to fix: {}", filename);
            }

            if let Some(number) = extract_correct_number(filename) {
                debug!("Successfully extracted number from filename: {}", number);

                let corrected_filename = format!(
                    "image_{:04}.{}",
                    number,
                    file.extension().and_then(|ext| ext.to_str()).unwrap_or("")
                );
                let corrected_path = file.with_file_name(corrected_filename);

                // Strict duplicate check
                if let Some(existing_file) = map.get(&number) {
                    return Err(ImageMappingError::DuplicateIdentifier(
                        number,
                        existing_file.clone(), // original file path
                        file.clone(),          // current original file path
                    ));
                }

                debug!(
                    "Mapped number {} to corrected file path: {:?}",
                    number, corrected_path
                );
                map.insert(number, file.clone()); // store the original file path
            } else {
                debug!("Failed to extract number from filename: {}", filename);
            }
        } else {
            debug!("Failed to convert file name to string for file: {:?}", file);
        }
    }

    debug!(
        "Finished processing files. Total files mapped: {}",
        map.len()
    );
    Ok(map)
}

/// Checks if any image in the provided list has a malformed filename.
/// An image is considered malformed if its filename does not match the pattern:
/// `image_XXXX_000Y` where `XXXX` are four digits and `Y` is 1-4.
///
/// # Arguments
///
/// * `images` - A slice of file paths to images.
///
/// # Returns
/// `true` if any image filename is malformed, `false` otherwise.
fn has_malformed_images(images: &[PathBuf]) -> bool {
    let re = Regex::new(r"^image_\d{4}_(000[1-4])$").expect("Failed to compile regex");

    images.iter().any(|image| {
        image
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|filename| !re.is_match(filename)) // Negate to identify malformed images
            .unwrap_or(true) // Consider files with invalid names as malformed
    })
}

fn extract_correct_number(filename: &str) -> Option<u32> {
    let re = Regex::new(r"_(\d+)").ok()?;
    re.captures(filename)
        .and_then(|caps| caps.get(1))
        .and_then(|m| m.as_str().parse::<u32>().ok())
}

/// Determines if an image filename is malformed based on a specific pattern.
///
/// This function checks whether a filename adheres to the following pattern:
/// - Must start with "image_"
/// - Followed by exactly four digits
/// - May optionally end with "_000" followed by a number between 1 and 4
///
/// # Parameters
/// - `filename`: The image filename to be validated
///
/// # Returns
/// - `true` if the filename does not match the expected pattern
/// - `false` if the filename is correctly formatted
///
/// # Notes
/// The valid format is: "image_XXXX" or "image_XXXX_000Y" where X is a digit and Y is 1-4.
/// Examples:
/// - "image_1234" → valid
/// - "image_1234_0002" → valid
/// - "image_abc" → invalid
/// - "image_12345" → invalid
fn is_malformed(filename: &str) -> bool {
    let re = Regex::new(r"^image_\d{4}(_000[1-4])?$").expect("Failed to compile regex");
    !re.is_match(filename)
}
