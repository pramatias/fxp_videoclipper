use anyhow::Result;
use log::{debug, error};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Holds the parts of a filename: a prefix, a suffix, the file path, the file extension, and a modified flag.
#[derive(Debug)]
pub struct FilenameParts {
    pub prefix: String,
    pub suffix: String,
    pub path: PathBuf,
    pub file_extension: String,
    pub modified: bool, // New field added
}

impl FilenameParts {
    /// Checks the `suffix` for an underscore after the first digit.
    /// If such an underscore exists, it removes the underscore and all subsequent characters,
    /// and marks the struct as modified.
    pub fn check_suffix(&mut self) -> Result<(), ImageMappingError> {
        // First, check for an underscore after the first digit.
        if let Some(first_digit_index) = self.suffix.chars().position(|c| c.is_ascii_digit()) {
            debug!("First digit found at index: {}", first_digit_index);

            if let Some(underscore_relative_index) = self.suffix[first_digit_index + 1..].find('_')
            {
                let underscore_index = first_digit_index + 1 + underscore_relative_index;
                debug!("Underscore found at index: {}", underscore_index);

                let new_suffix = &self.suffix[..underscore_index];
                debug!("New suffix after trimming at underscore: {}", new_suffix);

                if new_suffix != self.suffix {
                    self.suffix = new_suffix.to_string();
                    self.modified = true;
                    debug!("Suffix updated to: {}", self.suffix);
                }
            }
        }

        // Next, ensure the suffix contains only digits.
        let digits_only: String = self.suffix.chars().filter(|c| c.is_ascii_digit()).collect();
        debug!("Suffix after filtering for digits only: {}", digits_only);

        if digits_only != self.suffix {
            self.suffix = digits_only;
            self.modified = true;
            debug!("Suffix updated to digits only: {}", self.suffix);
        }

        // Finally, ensure the suffix is padded to a length that is a multiple of 4.
        let len = self.suffix.len();
        debug!("Current suffix length: {}", len);

        let remainder = len % 4;
        debug!("Remainder when divided by 4: {}", remainder);

        if remainder != 0 {
            // Calculate how many zeros to add.
            let padding = 4 - remainder;
            debug!("Padding required: {}", padding);

            // Left-pad the suffix with zeros.
            let padded = format!("{:0>width$}", self.suffix, width = len + padding);
            debug!("Padded suffix: {}", padded);

            if padded != self.suffix {
                self.suffix = padded;
                self.modified = true;
                debug!("Suffix updated after padding: {}", self.suffix);
            }
        }

        Ok(())
    }

    /// Checks if the provided new prefix is different from the current one (as extracted from the filename).
    /// If they differ, the function only updates the struct’s prefix field and marks it as modified.
    pub fn check_prefix(&mut self, new_prefix: &str) -> Result<(), ImageMappingError> {
        // Extract the current prefix from the file's path.
        let current_prefix = extract_prefix(&self.path)?;
        // If the prefixes differ, update the prefix field and mark as modified.
        if current_prefix != new_prefix {
            self.prefix = new_prefix.to_string();
            self.modified = true;
        }
        Ok(())
    }

    /// Returns whether the filename has been modified (i.e. its prefix changed).
    pub fn is_modified(&self) -> bool {
        self.modified
    }

    /// If the filename has been marked as modified, this function constructs a new filename using
    /// `construct_new_filename` and renames the file accordingly. After renaming, the modified flag is reset.
    pub fn save_file(&mut self) -> Result<(), ImageMappingError> {
        if self.modified {
            debug!("Filename is marked as modified. Proceeding to rename the file.");

            // Construct the new filename using the current prefix, suffix, and file extension.
            let new_filename = self.construct_new_filename(&self.prefix);
            debug!("New filename constructed: {}", new_filename);

            let new_path = self.path.with_file_name(new_filename);
            debug!("New file path: {:?}", new_path);

            if new_path != self.path {
                debug!("Renaming file from {:?} to {:?}", self.path, new_path);
                fs::rename(&self.path, &new_path).map_err(|e| {
                    debug!("Error renaming file: {}", e);
                    ImageMappingError::RenameError(e.to_string())
                })?;

                self.path = new_path;
                debug!("File renamed successfully. Updated path: {:?}", self.path);
            } else {
                debug!("New path is the same as the current path. No renaming needed.");
            }

            // Reset the modified flag after saving.
            self.modified = false;
            debug!("Modified flag reset to false.");
        } else {
            debug!("Filename is not modified. No action taken.");
        }

        Ok(())
    }

    /// Constructs the full filename using the given prefix, the existing suffix,
    /// and file extension.
    fn construct_new_filename(&self, new_prefix: &str) -> String {
        debug!("Constructing new filename with prefix: {}", new_prefix);
        debug!("Current suffix: {}", self.suffix);
        debug!("Current file extension: {}", self.file_extension);

        let new_filename = format!("{}_{}.{}", new_prefix, self.suffix, self.file_extension);
        debug!("New filename constructed: {}", new_filename);

        new_filename
    }
}

impl FilenameParts {
    /// Constructs a new `FilenameParts` by splitting the filename on the first underscore
    /// and extracting the file extension.
    /// Returns an error if the filename is not valid UTF-8, if no underscore is found,
    /// or if the file extension is missing or not valid UTF-8.
    pub fn new(file: &Path) -> Result<Self, ImageMappingError> {
        debug!("Attempting to create FilenameParts for file: {:?}", file);

        // Extract the file name as a &str.
        let filename_str = file
            .file_name()
            .and_then(|os_str| os_str.to_str())
            .ok_or_else(|| {
                let error = ImageMappingError::InvalidFilename(
                    file.to_path_buf(),
                    "Filename is not valid UTF-8".into(),
                );
                debug!("Error extracting filename: {:?}", error);
                error
            })?;
        debug!("Extracted filename as string: {}", filename_str);

        // Extract the file extension as a &str.
        let extension = file
            .extension()
            .and_then(|os_str| os_str.to_str())
            .ok_or_else(|| {
                let error = ImageMappingError::InvalidFilename(
                    file.to_path_buf(),
                    "Filename does not have a valid extension".into(),
                );
                debug!("Error extracting file extension: {:?}", error);
                error
            })?;
        debug!("Extracted file extension: {}", extension);

        // Split the filename on the first underscore.
        if let Some(pos) = filename_str.find('_') {
            debug!("Found underscore at position: {}", pos);

            let prefix = filename_str[..pos].to_string();
            let suffix = filename_str[pos + 1..].to_string();
            debug!("Extracted prefix: {}", prefix);
            debug!("Extracted suffix: {}", suffix);

            Ok(Self {
                prefix,
                suffix,
                path: file.to_path_buf(),
                file_extension: extension.to_string(),
                modified: false, // Initialize as false
            })
        } else {
            let error = ImageMappingError::InvalidFilename(
                file.to_path_buf(),
                "Filename does not contain '_' to split into prefix and suffix".into(),
            );
            debug!("Error splitting filename: {:?}", error);
            Err(error)
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

/// Extracts the prefix from the image filename.
/// The prefix is defined as everything in the file's stem before the first underscore ('_').
fn extract_prefix(path: &PathBuf) -> Result<String, ImageMappingError> {
    debug!("Attempting to extract prefix from path: {:?}", path);

    let filename = path.file_stem().and_then(|s| s.to_str()).ok_or_else(|| {
        let error = ImageMappingError::InvalidFilename(
            path.clone(),
            "Could not extract valid file stem".to_string(),
        );
        debug!("Error extracting file stem: {:?}", error);
        error
    })?;
    debug!("Extracted file stem: {}", filename);

    // Split at the first underscore and return the prefix.
    match filename.split_once('_') {
        Some((prefix, _)) => {
            debug!("Found underscore in filename. Extracted prefix: {}", prefix);
            Ok(prefix.to_string())
        }
        None => {
            debug!(
                "No underscore found in filename. Using entire stem as prefix: {}",
                filename
            );
            Ok(filename.to_string())
        }
    }
}
