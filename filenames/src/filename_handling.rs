// use anyhow::{Context, Result};
use anyhow::Result;
use log::{debug, error};
use regex::Regex;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
// use tempfile::TempDir;
use thiserror::Error;

// Trait definition
pub trait FilenameValidator {
    fn validate_and_fix_image_filenames(
        &self,
        images: &[PathBuf],
    ) -> Result<BTreeMap<u32, PathBuf>, ImageMappingError>;
}

// Validator that does not create a temporary directory
pub struct SimpleValidator;

#[derive(Error, Debug)]
pub enum ImageMappingError {
    #[error("Duplicate numerical identifier {0} found in files: {1:?} and {2:?}")]
    DuplicateIdentifier(u32, PathBuf, PathBuf),

    #[error("Failed to rename image {0} to {1}: {2}")]
    RenameError(PathBuf, PathBuf, String),

    #[error("Failed to copy or rename images: {0}")]
    CopyRenameError(String),
}

impl FilenameValidator for SimpleValidator {
    fn validate_and_fix_image_filenames(
        &self,
        images: &[PathBuf],
    ) -> Result<BTreeMap<u32, PathBuf>, ImageMappingError> {
        debug!("Starting validation of image filenames...");

        let mut fixed_images: Vec<PathBuf> = Vec::with_capacity(images.len());
        for image in images {
            // Use the helper function to rename the image if needed.
            let fixed_image = rename_image_if_malformed(image)?;
            fixed_images.push(fixed_image);
        }

        map_files_by_number(fixed_images)
    }
}

/// Renames the image if its filename is malformed.
/// Returns the new path if renamed, or the original path otherwise.
fn rename_image_if_malformed(image: &PathBuf) -> Result<PathBuf, ImageMappingError> {
    // Extract the file stem (filename without extension).
    let filename: String = image
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.to_owned())
        .unwrap_or_else(|| image.to_string_lossy().into_owned());

    if is_malformed(&filename) {
        debug!("Filename '{}' is malformed, removing suffix.", filename);
        // Remove the unwanted suffix.
        let fixed_name = remove_suffix(&filename);
        // Rebuild the path with the fixed filename.
        let mut new_path = image.with_file_name(&fixed_name);
        if let Some(ext) = image.extension() {
            new_path.set_extension(ext);
        }
        // Only perform renaming if the new path is different.
        if new_path != *image {
            fs::rename(&image, &new_path).map_err(|e| {
                ImageMappingError::RenameError(image.clone(), new_path.clone(), e.to_string())
            })?;
        }
        Ok(new_path)
    } else {
        debug!("Filename '{}' is not malformed; skipping fix.", filename);
        Ok(image.clone())
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

fn extract_correct_number(filename: &str) -> Option<u32> {
    let re = Regex::new(r"_(\d+)").ok()?;
    re.captures(filename)
        .and_then(|caps| caps.get(1))
        .and_then(|m| m.as_str().parse::<u32>().ok())
}

/// Determines if an image filename is malformed based on a specific pattern.
///
/// This function checks whether a filename adheres to the following pattern:
/// - Starts with any alphanumeric characters
/// - Followed by an underscore and four digits
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
/// The valid format is: "[alphanumerics]_XXXX" or "[alphanumerics]_XXXX_000Y" where X is a digit and Y is 1-4.
/// Examples:
/// - "img_1234" → valid
/// - "sample_5678_0003" → valid
/// - "image_abc" → invalid
/// - "test_12345" → invalid
fn is_malformed(filename: &str) -> bool {
    debug!("Checking if filename '{}' is malformed", filename);

    let re = Regex::new(r"^[A-Za-z]+_\d+_\d+$").expect("Failed to compile regex");

    debug!("Regex compiled successfully");

    // If the regex matches, it indicates two underscores with numbers after both.
    let is_match = re.is_match(filename);
    debug!("Regex match result for '{}': {}", filename, is_match);

    // Return true if it is malformed.
    is_match
}

/// Removes the suffix "_000[1-6]" from the filename if present.
///
/// If the suffix is found at the end of the filename, this function returns a new
/// string without it. Otherwise, the original filename is returned.
fn remove_suffix(filename: &str) -> String {
    debug!("Removing suffix from filename: '{}'", filename);

    // Find the last occurrence of '_'
    if let Some(idx) = filename.rfind('_') {
        debug!("Found '_' at index: {}", idx);

        // Extract the suffix after '_'
        let suffix = &filename[idx + 1..];
        debug!("Extracted suffix: '{}'", suffix);

        // Check if the suffix is non-empty and consists only of ASCII digits
        if !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit()) {
            debug!("Suffix '{}' is valid and will be removed", suffix);

            // Return the filename without the suffix
            let result = filename[..idx].to_string();
            debug!("Filename after removing suffix: '{}'", result);
            return result;
        } else {
            debug!(
                "Suffix '{}' is invalid (not all digits or empty), keeping original filename",
                suffix
            );
        }
    } else {
        debug!("No '_' found in filename, keeping original filename");
    }

    // Return the original filename if no valid suffix is found
    debug!("Returning original filename: '{}'", filename);
    filename.to_string()
}
