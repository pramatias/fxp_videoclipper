// use anyhow::{Context, Result};
use anyhow::Result;
use log::{debug, error};
use regex::Regex;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

use modes::Modes;

pub trait FileOperations {
    fn check_files(&self) -> Result<(), ImageMappingError>;

    fn load_files(&self, images: &[PathBuf]) -> Result<BTreeMap<u32, PathBuf>, ImageMappingError>;
}

impl FileOperations for Modes {
    fn load_files(&self, images: &[PathBuf]) -> Result<BTreeMap<u32, PathBuf>, ImageMappingError> {
        match self {
            Modes::Exporter | Modes::Sampler => Err(ImageMappingError::UnsupportedMode),
            Modes::Merger | Modes::Clutter | Modes::Clipper | Modes::Gmicer => {
                println!("Loading files for {:?}...", self);
                process_files(images)
            }
        }
    }
    fn check_files(&self) -> Result<(), ImageMappingError> {
        match self {
            Modes::Exporter | Modes::Sampler => Err(ImageMappingError::UnsupportedMode),
            Modes::Merger => {
                println!("Checking files for Merger...");
                Ok(())
            }
            Modes::Clutter => {
                println!("Checking files for Clutter...");
                Ok(())
            }
            Modes::Clipper => {
                println!("Checking files for Clipper...");
                Ok(())
            }
            Modes::Gmicer => {
                println!("Checking files for Gmicer...");
                Ok(())
            }
        }
    }
}

#[derive(Error, Debug)]
pub enum ImageMappingError {
    #[error("Mode not supported")]
    UnsupportedMode,

    #[error("Duplicate numerical identifier {0} found in files: {1:?} and {2:?}")]
    DuplicateIdentifier(u32, PathBuf, PathBuf),

    #[error("Failed to rename image {0}")]
    RenameError(String),

    #[error("Invalid filename {0:?}: {1}")]
    InvalidFilename(PathBuf, String),

    #[error("No images found on target folder {0}")]
    FileNotFound(String),
}

pub fn process_files(images: &[PathBuf]) -> Result<BTreeMap<u32, PathBuf>, ImageMappingError> {
    debug!("Starting validation of image filenames...");

    let mut fixed_images: Vec<PathBuf> = Vec::with_capacity(images.len());
    debug!(
        "Initialized fixed_images vector with capacity: {}",
        images.len()
    );

    for image in images {
        debug!("Processing image: {:?}", image);
        // First, fix any malformed filenames.
        let fixed_image = rename_image_if_malformed(image)?;
        debug!("Fixed malformed filename: {:?}", fixed_image);

        fixed_images.push(fixed_image);
        debug!("Added fixed image to fixed_images vector");
    }

    if let Some(first_image) = fixed_images.first() {
        debug!("First image in fixed_images: {:?}", first_image);
        let expected_prefix = extract_prefix(first_image)?;
        debug!("Extracted expected prefix: {}", expected_prefix);

        for image in &mut fixed_images {
            debug!("Validating image: {:?}", image);
            let current_prefix = extract_prefix(image)?;
            debug!("Extracted current prefix: {}", current_prefix);

            if current_prefix != expected_prefix {
                debug!(
                    "Filename stem '{}' does not match expected stem '{}'",
                    current_prefix, expected_prefix
                );
                // Delegate the renaming of the stem to a separate function.
                *image = fix_stem(image, &expected_prefix)?;
                debug!("Fixed stem for image: {:?}", image);
            } else {
                debug!("Filename stem matches expected stem, no changes needed");
            }
        }
    } else {
        debug!("No images found in fixed_images vector");
    }

    debug!("Mapping files by number...");
    let result = map_files_by_number(fixed_images)?;
    debug!("Successfully mapped files by number");

    Ok(result)
}

fn fix_stem(image: &PathBuf, expected_prefix: &str) -> Result<PathBuf, ImageMappingError> {
    // Try to get the file's stem as a &str.
    let filename =
        image
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or(ImageMappingError::InvalidFilename(
                image.clone(),
                "Could not extract valid file stem".to_string(),
            ))?;

    debug!("Original filename: {}", filename);

    // Split the filename at the first underscore.
    // This separates the current prefix from the rest of the stem.
    let new_stem = if let Some((prefix, rest)) = filename.split_once('_') {
        debug!("Found prefix: {}, rest: {}", prefix, rest);
        format!("{}_{}", expected_prefix, rest)
    } else {
        debug!(
            "No underscore found, using expected prefix: {}",
            expected_prefix
        );
        expected_prefix.to_string()
    };

    debug!("New stem: {}", new_stem);

    // Reconstruct the new file name by appending the extension if it exists.
    let new_filename = if let Some(ext) = image.extension().and_then(|s| s.to_str()) {
        debug!("File extension: {}", ext);
        format!("{}.{}", new_stem, ext)
    } else {
        debug!("No file extension found");
        new_stem
    };

    debug!("New filename: {}", new_filename);

    // Build the new path by replacing the file name.
    let mut new_path = image.clone();
    new_path.set_file_name(new_filename);

    debug!("New path: {:?}", new_path);

    // Rename the file and map any error to ImageMappingError.
    fs::rename(image, &new_path).map_err(|e| ImageMappingError::RenameError(e.to_string()))?;

    debug!("File successfully renamed to: {:?}", new_path);

    Ok(new_path)
}

/// Extracts the prefix from the image filename.
/// The prefix is defined as everything in the file's stem before the first underscore ('_').
fn extract_prefix(path: &PathBuf) -> Result<String, ImageMappingError> {
    let filename =
        path.file_stem()
            .and_then(|s| s.to_str())
            .ok_or(ImageMappingError::InvalidFilename(
                path.clone(),
                "Could not extract valid file stem".to_string(),
            ))?;

    // Split at the first underscore and return the prefix.
    match filename.split_once('_') {
        Some((prefix, _)) => Ok(prefix.to_string()),
        None => Ok(filename.to_string()),
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
            fs::rename(&image, &new_path)
                .map_err(|e| ImageMappingError::RenameError(e.to_string()))?;
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
