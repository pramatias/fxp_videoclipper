use anyhow::{anyhow, Context, Result};
use log::debug;
use rand::distributions::Alphanumeric;
use rand::thread_rng;
use rand::Rng;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::fs::File;

pub use modes::Modes;

// Define a trait that creates an output directory and returns its PathBuf.
pub trait ModeOutput {
    type Input;
    fn create_output(&self, input: Self::Input) -> Result<PathBuf>;
}

// Enum to hold all the possible outputs.
pub enum Output {
    Exporter(ExporterOutput),
    Sampler(SamplerOutput),
    Merger(MergerOutput),
    Clutter(ClutterOutput),
    Gmicer(GmicerOutput),
    Clipper(ClipperOutput),
}

// Implement conversion from Modes to Output.
impl From<Modes> for Output {
    fn from(mode: Modes) -> Self {
        match mode {
            Modes::Exporter => Output::Exporter(ExporterOutput),
            Modes::Merger => Output::Merger(MergerOutput),
            Modes::Sampler => Output::Sampler(SamplerOutput),
            Modes::Clutter => Output::Clutter(ClutterOutput),
            Modes::Clipper => Output::Clipper(ClipperOutput),
            Modes::Gmicer => Output::Gmicer(GmicerOutput),
        }
    }
}

pub struct ExporterOutput;
impl ModeOutput for ExporterOutput {
    // Input is a tuple of the input path and an optional explicit output directory string.
    type Input = (PathBuf, Option<String>);

    fn create_output(&self, input: Self::Input) -> Result<PathBuf> {
        let (input_path, output_directory) = input;
        match output_directory.as_deref() {
            Some(dir) => create_explicit_output_directory(dir),
            None => self.output_directory_auto_generated(&input_path),
        }
    }
}

pub struct SamplerOutput;
impl ModeOutput for SamplerOutput {
    // Extend the Input tuple to include sample_number (e.g., u32)
    type Input = (PathBuf, Option<String>, usize);

    /// Creates the output directory either explicitly (if provided) or auto-generates one.
    /// The auto-generated directory will now take `sample_number` into account.
    fn create_output(&self, input: Self::Input) -> Result<PathBuf> {
        // Destructure the tuple into `input_path`, `output_directory`, and `sample_number`
        let (input_path, output_directory, sample_number) = input;

        match output_directory {
            Some(dir) => self.create_explicit_output_directory(&dir, sample_number),
            None => self.output_directory_auto_generated(&input_path),
        }
    }
}

pub struct ClutterOutput;
impl ModeOutput for ClutterOutput {
    // Adjusted Input to match the new pattern: (PathBuf, Option<String>)
    type Input = (PathBuf, Option<String>);
    fn create_output(&self, input: Self::Input) -> Result<PathBuf> {
        let (input_path, output_directory) = input;
        match output_directory.as_deref() {
            Some(dir) => create_explicit_output_directory(dir),
            None => self.output_directory_auto_generated(&input_path),
        }
    }
}

pub struct MergerOutput;
impl ModeOutput for MergerOutput {
    // The input is a tuple: (input_path, output_directory, merge_value)
    type Input = (PathBuf, Option<String>, f32);
    fn create_output(&self, input: Self::Input) -> Result<PathBuf> {
        let (input_path, output_directory, merge_value) = input;
        match output_directory.as_deref() {
            Some(dir) => create_explicit_output_directory(&dir),
            None => self.output_directory_auto_generated(&input_path, merge_value),
        }
    }
}

pub struct GmicerOutput;
impl ModeOutput for GmicerOutput {
    type Input = (PathBuf, Vec<String>, Option<String>);
    fn create_output(&self, input: Self::Input) -> Result<PathBuf> {
        let (input_path, gmic_args, output_directory) = input;
        match output_directory.as_deref() {
            Some(dir) => create_explicit_output_directory(dir),
            None => self.output_directory_auto_generated(&input_path, &gmic_args),
        }
    }
}

pub struct ClipperOutput;
impl ModeOutput for ClipperOutput {
    // Input: (input_path, optional custom directory as PathBuf, optional string parameter)
    type Input = (PathBuf, Option<PathBuf>, Option<String>);

    fn create_output(&self, input: Self::Input) -> Result<PathBuf> {
        let (input_path, maybe_path, maybe_explicit) = input;
        match maybe_explicit.as_deref() {
            // If an explicit output directory is provided, use it.
            Some(explicit) => self.create_explicit_output_file(explicit),
            // Otherwise, auto-generate the output directory, passing the optional mp3_path.
            None => self.output_file_auto_generated(&input_path, maybe_path.as_deref()),
        }
    }
}

/// Enum representing the output type.
enum OutputType {
    File,
    Directory,
}
impl MergerOutput {
    /// Automatically generates an output directory by appending a random suffix.
    /// The base directory name is built using the input directory name with a `_merged_{merge_value}` suffix.
    fn output_directory_auto_generated(
        &self,
        input_path: &Path,
        merge_value: f32,
    ) -> Result<PathBuf> {
        let base_directory_name = format!(
            "{}_merged_{}",
            input_path
                .file_name()
                .unwrap_or_else(|| OsStr::new("input"))
                .to_string_lossy(),
            merge_value
        );

        let parent = input_path.parent().unwrap_or_else(|| Path::new("."));
        // Use the refactored function instead of duplicating the loop.
        create_unique_dir(parent, &base_directory_name)
    }
}
impl SamplerOutput {
    /// Auto-generates an output directory based on the input path.
    fn output_directory_auto_generated(&self, input_path: &Path) -> Result<PathBuf> {
        let base_directory_name = "sample_frames";
        let candidate_path = input_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(base_directory_name);

        let output_path = if candidate_path.exists() {
            loop {
                let candidate_name = generate_random_name(OsStr::new(base_directory_name));
                let candidate_path = input_path
                    .parent()
                    .unwrap_or_else(|| Path::new("."))
                    .join(candidate_name);
                if !candidate_path.exists() {
                    break candidate_path;
                }
            }
        } else {
            candidate_path
        };

        fs::create_dir_all(&output_path)
            .with_context(|| format!("Failed to create output directory {:?}", output_path))?;
        Ok(output_path)
    }
/// Creates an explicit output target.
/// - When sampling_number == 1, the output is a file. If the file or directory already exists,
///   it is removed and replaced by an empty file.
/// - When sampling_number > 1, the output is a directory. Any existing file or directory at that path is removed.
    /// Creates an explicit output target.
    /// - When sampling_number is 1, the target is treated as a file. If a file or directory exists at the
    ///   given path, it is removed and replaced with an empty file.
    /// - When sampling_number is greater than 1, the target is treated as a directory. Any existing file or directory
    ///   at that path is removed and a directory is created.
    fn create_explicit_output_directory(&self, output_dir: &str, sampling_number: usize) -> Result<PathBuf> {
        debug!("Output path provided: {:?}", output_dir);
        let output_path = Path::new(output_dir);

        // Determine the output type based on the sampling number.
        let output_type = match sampling_number {
            1 => OutputType::File,
            _ => OutputType::Directory,
        };

        match output_type {
            OutputType::File => {
                if output_path.exists() {
                    match output_path.metadata() {
                        Ok(metadata) if metadata.is_dir() => {
                            debug!("Existing directory found at file target, removing it: {:?}", output_path);
                            fs::remove_dir_all(output_path)
                                .context("Failed to remove existing directory at file target")?;
                        }
                        Ok(metadata) if metadata.is_file() => {
                            debug!("Existing file found, removing it: {:?}", output_path);
                            fs::remove_file(output_path)
                                .context("Failed to remove existing file")?;
                        }
                        Err(e) => return Err(e.into()),
                        _ => {}
                    }
                }
                debug!("Creating output file: {:?}", output_path);
                File::create(output_path)
                    .context("Failed to create output file")?;
            }
            OutputType::Directory => {
                if output_path.exists() {
                    match output_path.metadata() {
                        Ok(metadata) if metadata.is_file() => {
                            debug!("Existing file found at directory target, removing it: {:?}", output_path);
                            fs::remove_file(output_path)
                                .context("Failed to remove existing file at directory target")?;
                        }
                        Ok(metadata) if metadata.is_dir() => {
                            debug!("Existing directory found, removing it: {:?}", output_path);
                            fs::remove_dir_all(output_path)
                                .context("Failed to remove existing directory")?;
                        }
                        Err(e) => return Err(e.into()),
                        _ => {}
                    }
                }
                debug!("Creating output directory: {:?}", output_path);
                fs::create_dir_all(output_path)
                    .context("Failed to create output directory")?;
            }
        }

        Ok(output_path.to_path_buf())
    }

}
impl ExporterOutput {
    // This method auto-generates the output directory.
    fn output_directory_auto_generated(&self, input_path: &Path) -> Result<PathBuf> {
        let base_directory_name = format!(
            "{}_original_frames",
            input_path
                .file_stem() // Strip the extension.
                .unwrap_or_else(|| OsStr::new("input"))
                .to_string_lossy()
        );

        // Determine the parent directory of the input path.
        let parent = input_path.parent().unwrap_or_else(|| Path::new("."));
        // Delegate the unique directory creation to the helper function.
        create_unique_dir(parent, &base_directory_name)
    }
}
impl ClutterOutput {
    fn output_directory_auto_generated(&self, input_path: &Path) -> Result<PathBuf> {
        let base_directory_name = format!(
            "{}_clutted",
            input_path
                .file_name()
                .unwrap_or_else(|| OsStr::new("input"))
                .to_string_lossy()
        );

        let parent = input_path.parent().unwrap_or_else(|| Path::new("."));
        create_unique_dir(parent, &base_directory_name)
    }
}
impl GmicerOutput {
    /// Automatically generates an output directory name based on the input path and GMIC arguments.
    fn output_directory_auto_generated(
        &self,
        input_path: &Path,
        gmic_args: &[String],
    ) -> Result<PathBuf> {
        let first_arg = gmic_args
            .first()
            .ok_or_else(|| anyhow!("GMIC arguments should not be empty"))?;
        debug!("First GMIC argument: {}", first_arg);
        debug!("Input path: {:?}", input_path);

        let base_directory_name = format!(
            "{}_{}",
            input_path
                .file_name()
                .unwrap_or_else(|| OsStr::new("input"))
                .to_string_lossy(),
            first_arg
        );

        // Determine the parent directory for the new directory.
        let parent_dir = input_path.parent().unwrap_or_else(|| Path::new("."));

        // Use the helper function to create a unique directory.
        let output_path = create_unique_dir(parent_dir, &base_directory_name)
            .with_context(|| format!("Failed to create output directory under {:?}", parent_dir))?;

        debug!("Output directory created successfully: {:?}", output_path);
        Ok(output_path)
    }
}
impl ClipperOutput {
    /// Creates an output file using the explicitly provided path.
    fn create_explicit_output_file(&self, output_file: &str) -> Result<PathBuf> {
        debug!("Output file provided: {:?}", output_file);
        let output_path = std::path::Path::new(output_file);
        if output_path.exists() {
            debug!("Output file exists, removing it: {:?}", output_path);
            std::fs::remove_file(output_path)
                .with_context(|| "Failed to remove existing output file")?;
        }
        debug!("Creating output file: {:?}", output_path);
        // Create the file to ensure it exists.
        std::fs::File::create(output_path).with_context(|| "Failed to create output file")?;
        // Return the output file's path as a PathBuf.
        Ok(output_path.to_path_buf())
    }

    fn output_file_auto_generated(
        &self,
        input_dir: &Path,
        mp3_path: Option<&Path>,
    ) -> Result<PathBuf> {
        if let Some(mp3) = mp3_path {
            debug!("MP3 path provided: {:?}", mp3);
            let parent = mp3.parent().unwrap_or_else(|| Path::new("."));
            debug!("Using parent directory: {:?}", parent);
            let stem = mp3.file_stem().unwrap_or_else(|| OsStr::new("input"));
            debug!("Using file stem: {:?}", stem);
            Ok(self.build_output_file(parent, stem))
        } else {
            debug!(
                "No MP3 path provided, using input directory: {:?}",
                input_dir
            );
            let parent = input_dir.parent().unwrap_or_else(|| Path::new("."));
            debug!("Using parent directory: {:?}", parent);
            let stem = input_dir.file_name().unwrap_or_else(|| OsStr::new("input"));
            debug!("Using file stem: {:?}", stem);
            Ok(self.build_output_file(parent, stem))
        }
    }

    /// Generates a random name based on the given base string by appending two random alphanumeric characters.
    fn generate_random_name(&self, base: &OsStr) -> String {
        let mut rng = rand::thread_rng();
        let random_suffix: String = (0..2).map(|_| rng.sample(Alphanumeric) as char).collect();
        format!("{}{}", base.to_string_lossy(), random_suffix)
    }

    /// Builds the output file path by combining the directory and stem.
    ///
    /// If the initial candidate (with an "mp4" extension) exists, a random suffix is appended
    /// to the stem to form a new candidate.
    fn build_output_file(&self, dir: &Path, stem: &OsStr) -> PathBuf {
        debug!("Starting build_output_file function");
        debug!("Directory: {:?}, Stem: {:?}", dir, stem);

        // Create the initial candidate path by joining the directory and stem, then setting its extension.
        let mut candidate = dir.join(stem);
        candidate.set_extension("mp4");
        debug!("Initial candidate path: {:?}", candidate);

        // If the candidate already exists, generate a new stem and update the candidate.
        if candidate.exists() {
            debug!("Candidate path already exists: {:?}", candidate);
            let new_stem = self.generate_random_name(stem);
            debug!("Generated new stem: {:?}", new_stem);
            candidate = dir.join(new_stem);
            candidate.set_extension("mp4");
            debug!("Updated candidate path: {:?}", candidate);
        } else {
            debug!("Candidate path does not exist, using: {:?}", candidate);
        }

        debug!("Final output path: {:?}", candidate);
        candidate
    }
}
fn create_unique_dir(parent: &Path, base_name: &str) -> Result<PathBuf> {
    let output_path = loop {
        // Generate a candidate name by appending two random characters.
        let candidate_name = generate_random_name(OsStr::new(base_name));
        let candidate_path = parent.join(candidate_name);

        if !candidate_path.exists() {
            break candidate_path;
        }
    };

    fs::create_dir_all(&output_path)
        .with_context(|| format!("Failed to create output directory {:?}", output_path))?;
    Ok(output_path)
}

/// Generates a random name by appending two random alphanumeric characters to the given base name.
fn generate_random_name(base: &OsStr) -> String {
    let mut rng = thread_rng();
    let random_suffix: String = (0..2).map(|_| rng.sample(Alphanumeric) as char).collect();
    format!("{}{}", base.to_string_lossy(), random_suffix)
}

// This method creates an explicit output directory.
fn create_explicit_output_directory(output_dir: &str) -> Result<PathBuf> {
    debug!("Output directory provided: {:?}", output_dir);
    let output_path = Path::new(output_dir);
    if output_path.exists() {
        debug!("Output directory exists, removing it: {:?}", output_path);
        fs::remove_dir_all(output_path).context("Failed to remove existing output directory")?;
    }
    debug!("Creating output directory: {:?}", output_path);
    fs::create_dir_all(output_path).context("Failed to create output directory")?;
    Ok(output_path.to_path_buf())
}
