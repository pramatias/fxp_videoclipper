use anyhow::Result;
use log::debug;
use regex::Regex;
use std::collections::BTreeMap;
use std::path::PathBuf;

use modes::Modes;

use crate::filename_parts::FilenameParts;
use crate::filename_parts::ImageMappingError as OtherImageMappingError;

impl FileOperations for Modes {
    fn load_files(
        &self,
        images: &[PathBuf],
    ) -> Result<BTreeMap<u32, PathBuf>, OtherImageMappingError> {
        match self {
            Modes::Exporter | Modes::Sampler => {
                debug!("Unsupported mode: {:?}. Cannot load files.", self);
                Err(OtherImageMappingError::UnsupportedMode)
            }
            Modes::Merger | Modes::Clutter | Modes::Clipper | Modes::Gmicer => {
                debug!("Loading files for mode: {:?}", self);

                // Process the first image: create a FilenameParts and check its suffix.
                debug!("Processing first image: {:?}", images[0]);
                let first_parts = FilenameParts::new(&images[0])?;
                debug!("First image parts: {:?}", first_parts);

                // Use the first image's prefix as the common prefix for all subsequent images.
                let common_prefix = first_parts.prefix.clone();
                debug!("Common prefix extracted: {}", common_prefix);

                // Create a new vector to store the updated PathBufs.
                let mut new_image_paths: Vec<PathBuf> = Vec::with_capacity(images.len());
                // Use the first image's (potentially modified) path.
                new_image_paths.push(first_parts.path.clone());

                // Process remaining images.
                for image in &images[1..] {
                    debug!("Processing image: {:?}", image);
                    let mut parts = FilenameParts::new(image)?;
                    debug!("Image parts: {:?}", parts);

                    // Check the prefix against the common prefix.
                    debug!("Checking prefix for image: {:?}", image);
                    parts.check_prefix(&common_prefix)?;
                    debug!("Prefix check completed for image: {:?}", image);

                    // Check the suffix for each image.
                    debug!("Checking suffix for image: {:?}", image);
                    parts.check_suffix()?;
                    debug!("Suffix check completed for image: {:?}", image);

                    // If the file was modified, save it.
                    if parts.is_modified() {
                        debug!("Image was modified. Saving changes for: {:?}", image);
                        parts.save_file()?;
                        debug!("Changes saved for: {:?}", image);
                    } else {
                        debug!("No modifications needed for: {:?}", image);
                    }

                    // Append the updated PathBuf from the parts.
                    new_image_paths.push(parts.path.clone());
                }

                // Map the new files by number.
                debug!("Mapping files by number...");
                let result = map_files_by_number(new_image_paths);
                debug!("Files mapped successfully.");
                result
            }
        }
    }
}

pub trait FileOperations {
    fn load_files(
        &self,
        images: &[PathBuf],
    ) -> Result<BTreeMap<u32, PathBuf>, OtherImageMappingError>;
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
/// - `Result<BTreeMap<u32, PathBuf>, OtherImageMappingError>`: A sorted map of numeric IDs to
///   corrected file paths, or an error if duplicates are found.
///
/// # Notes
/// - Filenames are corrected to the format `image_{number}.{extension}`.
/// - If duplicate numeric identifiers are detected, an error is returned.
fn map_files_by_number(
    files: Vec<PathBuf>,
) -> Result<BTreeMap<u32, PathBuf>, OtherImageMappingError> {
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
                    return Err(OtherImageMappingError::DuplicateIdentifier(
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
    debug!("Attempting to extract number from filename: {}", filename);

    let re = Regex::new(r"_(\d+)").ok()?;
    debug!("Regex compiled successfully.");

    let number = re
        .captures(filename)
        .and_then(|caps| {
            debug!("Captures found: {:?}", caps);
            caps.get(1)
        })
        .and_then(|m| {
            let matched_str = m.as_str();
            debug!("Matched number string: {}", matched_str);
            matched_str.parse::<u32>().ok()
        });

    match number {
        Some(num) => {
            debug!("Successfully extracted number: {}", num);
            Some(num)
        }
        None => {
            debug!("No number found in filename.");
            None
        }
    }
}
