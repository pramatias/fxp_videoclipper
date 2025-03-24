use anyhow::{anyhow, Context, Result};
use log::debug;
use std::ffi::OsStr;
use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};

pub use modes::Modes;

pub trait ModeOutput {
    type Parameters;
    fn create_output(&self, input: Self::Parameters) -> Result<PathBuf>;
}

enum OutputType {
    File,
    Directory,
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
    // Parameters is a tuple of the input path and an optional explicit output directory string.
    type Parameters = (PathBuf, Option<String>);

    fn create_output(&self, input: Self::Parameters) -> Result<PathBuf> {
        let (input_path, output_directory) = input;
        match output_directory.as_deref() {
            Some(dir) => create_explicit_output_directory(dir),
            None => self.output_directory_auto_generated(&input_path),
        }
    }
}

pub struct SamplerOutput;
impl ModeOutput for SamplerOutput {
    // Extend the Parameters tuple to include sample_number (e.g., u32)
    type Parameters = (PathBuf, Option<String>, usize);

    /// Creates the output directory either explicitly (if provided) or auto-generates one.
    /// The auto-generated directory will now take `sample_number` into account.
    fn create_output(&self, input: Self::Parameters) -> Result<PathBuf> {
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
    type Parameters = (PathBuf, Option<String>);

    /// Creates an output directory for clutter output, either explicitly or automatically.
    ///
    /// This function handles the creation of the output directory based on the provided parameters.
    ///
    /// # Parameters
    /// - `input_path`: The source path used for generating the output directory if no explicit directory is provided.
    /// - `output_directory`: An optional directory path to use for output; if `None`, the directory is generated automatically from `input_path`.
    ///
    /// # Returns
    /// - `Result<PathBuf>`: The path to the created or specified output directory.
    ///
    /// # Notes
    /// - If an explicit output directory is provided, it is used directly.
    /// - If no output directory is provided, one is automatically generated from the input path.
    fn create_output(&self, input: Self::Parameters) -> Result<PathBuf> {
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
    type Parameters = (PathBuf, Option<String>, f32);

    /// Creates output path based on input parameters.
    ///
    /// This method determines the appropriate output path by checking if an explicit output directory is provided.
    /// If not, it generates the directory automatically using the input path and merge value.
    ///
    /// # Parameters
    /// - `input_path`: The path to the input file.
    /// - `output_directory`: An optional directory to use for output.
    /// - `merge_value`: A floating-point value used in auto-generating the output directory.
    ///
    /// # Returns
    /// - `Result<PathBuf>`: The resulting output path, or an error if it fails.
    ///
    /// # Notes
    /// - If `output_directory` is provided, it is used explicitly.
    /// - If `output_directory` is not provided, the directory is auto-generated based on `input_path` and `merge_value`.
    fn create_output(&self, input: Self::Parameters) -> Result<PathBuf> {
        let (input_path, output_directory, merge_value) = input;
        match output_directory.as_deref() {
            Some(dir) => create_explicit_output_directory(&dir),
            None => self.output_directory_auto_generated(&input_path, merge_value),
        }
    }
}

pub struct GmicerOutput;
impl ModeOutput for GmicerOutput {
    type Parameters = (PathBuf, Vec<String>, Option<String>);

    /// Creates an output path for GMICer based on input parameters.
    ///
    /// This function determines the appropriate output path by evaluating the provided
    /// parameters, which include the input file path, GMIC arguments, and an optional output
    /// directory. If an output directory is specified, it is used directly; otherwise,
    /// the function generates a directory name automatically based on the input file path
    /// and GMIC arguments.
    ///
    /// # Parameters
    /// - `input_path`: Path to the input file.
    /// - `gmic_args`: Vector of arguments for GMIC.
    /// - `output_directory`: Optional output directory.
    ///
    /// # Returns
    /// - `Result<PathBuf>`: The determined output path, or an error if creation fails.
    ///
    /// # Notes
    /// - If `output_directory` is `None`, it is automatically generated from `input_path` and `gmic_args`.
    fn create_output(&self, input: Self::Parameters) -> Result<PathBuf> {
        let (input_path, gmic_args, output_directory) = input;
        match output_directory.as_deref() {
            Some(dir) => create_explicit_output_directory(dir),
            None => self.output_directory_auto_generated(&input_path, &gmic_args),
        }
    }
}

pub struct ClipperOutput;
impl ModeOutput for ClipperOutput {
    // Parameters: (input_path, optional custom directory as PathBuf, optional string parameter)
    type Parameters = (PathBuf, Option<PathBuf>, Option<String>);

    /// Creates output files based on specified parameters.
    ///
    /// This function handles both explicit output directory specification and automatic generation.
    ///
    /// # Parameters
    /// - `input_path`: The input file path as a `PathBuf`.
    /// - `mp3_path`: An optional path to an MP3 file.
    /// - `output_path`: An optional output directory as a `String`.
    ///
    /// # Returns
    /// - `Result<PathBuf>`: The path to the created output file on success.
    ///
    /// # Notes
    /// - If an explicit `output_path` is provided, the function will create the output file in that directory.
    /// - If no `output_path` is provided, the function will auto-generate the output directory based on the `input_path` and `mp3_path`.
    fn create_output(&self, input: Self::Parameters) -> Result<PathBuf> {
        let (input_path, mp3_path, output_path) = input;
        match output_path.as_deref() {
            // If an explicit output directory is provided, use it.
            Some(output_path) => {
                self.create_explicit_output_file(output_path, mp3_path, &input_path)
            }
            // Otherwise, auto-generate the output directory, passing the optional mp3_path.
            None => self.output_file_auto_generated(&input_path, mp3_path.as_deref()),
        }
    }
}

impl GmicerOutput {
    /// Automatically generates an output directory name based on the input path and GMIC arguments.
    ///
    /// # Parameters
    /// - `input_path`: The path to the input file.
    /// - `gmic_args`: The arguments provided to GMIC.
    ///
    /// # Returns
    /// - `Result<PathBuf>`: The path to the generated output directory, or an error if creation fails.
    ///
    /// # Notes
    /// - The directory name is created by combining the input filename and the first GMIC argument.
    /// - If the input filename is unavailable, it defaults to "input".
    /// - If the directory already exists, a unique suffix is appended to ensure uniqueness.
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
impl MergerOutput {
    /// Automatically generates a unique output directory name by appending a suffix.
    /// Creates a directory in the parent of the given input path with a base name
    /// formatted as `input_filename_merged_{merge_value}`.
    ///
    /// # Parameters
    /// - `input_path`: The input file path used to derive the output directory name.
    /// - `merge_value`: A float value incorporated into the directory name.
    ///
    /// # Returns
    /// - `PathBuf`: The path to the newly created directory.
    ///
    /// # Notes
    /// - The directory is created in the parent directory of `input_path`.
    /// - Ensures uniqueness by appending a random suffix if necessary.
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
    /// Auto-generates an output directory based on the input path, ensuring a unique name.
    ///
    /// This function creates an output directory by first attempting to use a base name.
    /// If that directory exists, it appends a numerical suffix to find an available name.
    ///
    /// # Parameters
    /// - `input_path`: The path used as the foundation for the output directory.
    ///
    /// # Returns
    /// - `Result<PathBuf>`: The path to the created directory, or an error if creation fails.
    ///
    /// # Notes
    /// - The base directory name is "sample_frames".
    /// - If the base name is taken, it appends a counter (e.g., "sample_frames_1", "sample_frames_2").
    fn output_directory_auto_generated(&self, input_path: &Path) -> Result<PathBuf> {
        let base_directory_name = "sample_frames";
        debug!("Base directory name: {}", base_directory_name);

        let parent = input_path.parent().unwrap_or_else(|| Path::new("."));
        debug!("Parent directory: {:?}", parent);

        let candidate_path = parent.join(base_directory_name);
        debug!("Initial candidate path: {:?}", candidate_path);

        let output_path = if candidate_path.exists() {
            debug!("Candidate path exists, looking for alternative");
            let mut counter = 1;
            loop {
                let candidate_name = format!("{}_{}", base_directory_name, counter);
                let candidate_path = parent.join(&candidate_name);
                debug!("Checking alternative path: {:?}", candidate_path);

                if !candidate_path.exists() {
                    debug!("Found available path: {:?}", candidate_path);
                    break candidate_path;
                }
                counter += 1;
            }
        } else {
            debug!("Candidate path is available");
            candidate_path
        };

        debug!("Creating directory at: {:?}", output_path);
        fs::create_dir_all(&output_path)
            .with_context(|| format!("Failed to create output directory {:?}", output_path))?;

        debug!("Successfully created output directory: {:?}", output_path);
        Ok(output_path)
    }

    /// Creates an explicit output target.
    /// - When sampling_number is 1, the target is treated as a file.
    ///   * If the provided path exists as a file, it is removed.
    ///   * If it exists as a directory, a file named "output_file" is created inside that directory.
    ///   * If it doesn't exist, an output file is created at that path (ensuring parent dirs exist).
    /// - When sampling_number is greater than 1, the target is treated as a directory.
    ///   * Any existing file or directory at that path is removed and a directory is created.
    fn create_explicit_output_directory(
        &self,
        output_dir: &str,
        sampling_number: usize,
    ) -> Result<PathBuf> {
        debug!("Output path provided: {:?}", output_dir);
        let output_path = Path::new(output_dir);

        // Map sampling number to output type.
        let output_type = match sampling_number {
            1 => OutputType::File,
            _ => OutputType::Directory,
        };

        match output_type {
            OutputType::File => {
                // Check if the path exists.
                if output_path.exists() {
                    if output_path.is_dir() {
                        // Instead of removing the directory, append a file name.
                        let file_path = output_path.join("sample_frame.png");
                        debug!(
                            "Output path is a directory; creating file inside: {:?}",
                            file_path
                        );
                        // If the file already exists in the directory, remove it.
                        if file_path.exists() {
                            if file_path.is_file() {
                                debug!(
                                    "Existing file inside directory found, removing it: {:?}",
                                    file_path
                                );
                                fs::remove_file(&file_path).context(
                                    "Failed to remove existing file in directory target",
                                )?;
                            }
                        }
                        File::create(&file_path)
                            .context("Failed to create output file inside directory")?;
                        return Ok(file_path);
                    } else if output_path.is_file() {
                        debug!("Existing file found, removing it: {:?}", output_path);
                        fs::remove_file(output_path).context("Failed to remove existing file")?;
                    }
                } else {
                    // If the file doesn't exist, ensure its parent directories exist.
                    if let Some(parent) = output_path.parent() {
                        fs::create_dir_all(parent)
                            .context("Failed to create parent directories for output file")?;
                    }
                }
                debug!("Creating output file: {:?}", output_path);
                File::create(output_path).context("Failed to create output file")?;
                Ok(output_path.to_path_buf())
            }
            OutputType::Directory => {
                if output_path.exists() {
                    if output_path.is_file() {
                        debug!(
                            "Existing file found at directory target, removing it: {:?}",
                            output_path
                        );
                        fs::remove_file(output_path)
                            .context("Failed to remove existing file at directory target")?;
                    }
                }
                debug!("Creating output directory: {:?}", output_path);
                fs::create_dir_all(output_path).context("Failed to create output directory")?;
                Ok(output_path.to_path_buf())
            }
        }
    }
}
impl ExporterOutput {
    /// Auto-generates an output directory based on the input file's path.
    ///
    /// This method creates a uniquely named directory for output in the parent
    /// directory of the input path, using the input filename without extension.
    ///
    /// # Parameters
    /// - `input_path`: The path to the input file used to determine the output directory.
    ///
    /// # Returns
    /// - `Result<PathBuf>`: The path to the created output directory.
    ///
    /// # Notes
    /// - The directory name is formatted as `<input_name>_original_frames`.
    /// - If the directory exists, a unique name is created by appending a number.
    /// - The directory is created in the parent directory of `input_path`.
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
    /// Generates a unique output directory name based on the input file's name and location.
    ///
    /// This function constructs a directory name by appending `_clutted` to the input file's name.
    /// If no file name is present, it defaults to `input_clutted`.
    /// The directory is created in the same location as the input file.
    ///
    /// # Parameters
    /// - `input_path`: The path to the input file used to generate the output directory name.
    ///
    /// # Returns
    /// - `Result<PathBuf>`: The path to the generated directory, or an error if creation fails.
    ///
    /// # Notes
    /// - If the directory already exists, a unique name is created by appending a numerical suffix.
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
impl ClipperOutput {
    /// Creates an explicit output file path, handling both file and directory cases.
    ///
    /// This function determines the appropriate output path based on whether the provided
    /// path points to a file or directory. If the output path exists and is a file, it will
    /// be removed before creating the new output file.
    ///
    /// # Parameters
    /// - `output_file_or_dir`: The desired output path, which can be a file or directory.
    /// - `mp3_path`: An optional MP3 file path used to derive the output filename.
    /// - `input_dir`: The input directory path used as a fallback when `mp3_path` is not provided.
    ///
    /// # Returns
    /// - `Result<PathBuf>`: The final output file path as a `PathBuf` on success.
    ///
    /// # Notes
    /// - If `output_file_or_dir` is a directory and `mp3_path` is provided, the output filename
    ///   will be derived from the MP3 file's stem with an `.mp4` extension.
    /// - If `output_file_or_dir` is a directory and `mp3_path` is not provided, the output filename
    ///   will be derived from the `input_dir`'s name with an `.mp4` extension.
    /// - Existing files at the output path will be removed before creating the new file.
    fn create_explicit_output_file(
        &self,
        output_file_or_dir: &str,
        mp3_path: Option<PathBuf>,
        input_dir: &Path,
    ) -> Result<PathBuf> {
        debug!("Output file provided: {:?}", output_file_or_dir);
        let output_path = std::path::Path::new(output_file_or_dir);

        // Determine the output type based on the existing file system entry.
        let output_type = if output_path.exists() {
            if output_path.is_dir() {
                OutputType::Directory
            } else {
                OutputType::File
            }
        } else {
            // Default to file if the path does not exist.
            OutputType::File
        };

        // Compute the final output path.
        let final_output_path = match output_type {
            OutputType::File => output_path.to_path_buf(),
            OutputType::Directory => {
                if let Some(mp3) = mp3_path {
                    // Extract the file stem (filename without extension) from the mp3 path.
                    let stem = mp3
                        .file_stem()
                        .ok_or_else(|| anyhow!("MP3 path does not have a valid file stem"))?;
                    // Create a new filename with .mp4 extension.
                    let mut new_filename = std::ffi::OsString::from(stem);
                    new_filename.push(".mp4");
                    output_path.join(new_filename)
                } else {
                    // Use the input directory's filename.
                    let input_dir_filename = input_dir.file_name().ok_or_else(|| {
                        anyhow!("Input directory does not have a valid file name")
                    })?;
                    let mut new_filename = std::ffi::OsString::from(input_dir_filename);
                    new_filename.push(".mp4");
                    output_path.join(new_filename)
                }
            }
        };

        // If the final output path already exists and it's a file, remove it.
        if final_output_path.exists() {
            if final_output_path.is_file() {
                debug!(
                    "Output file exists as file, removing it: {:?}",
                    final_output_path
                );
                std::fs::remove_file(&final_output_path)
                    .with_context(|| "Failed to remove existing output file")?;
            } else {
                debug!(
                    "Output path is a directory, not removing it: {:?}",
                    final_output_path
                );
            }
        }

        debug!("Creating output file: {:?}", final_output_path);
        std::fs::File::create(&final_output_path)
            .with_context(|| "Failed to create output file")?;

        Ok(final_output_path)
    }

    /// Generates an output file path based on the provided MP3 file or input directory.
    ///
    /// This function constructs the output file path by using either the MP3 file's parent
    /// directory and file stem, or the input directory's details if no MP3 path is provided.
    ///
    /// # Parameters
    /// - `input_dir`: The input directory path used as a fallback when no MP3 path is provided.
    /// - `mp3_path`: An optional MP3 file path that determines the output directory and filename.
    ///
    /// # Returns
    /// - `Result<PathBuf>`: The constructed output file path.
    ///
    /// # Notes
    /// - If an MP3 path is provided, the function uses its parent directory and file stem.
    /// - If no MP3 path is provided, the function uses the input directory's parent and name.
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

    /// Constructs an output file path with a unique name for an MP4 file.
    ///
    /// This function generates a file path by combining the provided directory and stem.
    /// If the desired file already exists, it appends an incrementing counter to the stem to ensure uniqueness.
    ///
    /// # Parameters
    /// - `dir`: The directory path where the output file will be created.
    /// - `stem`: The base name of the file without the extension.
    ///
    /// # Returns
    /// - The unique `PathBuf` representing the output file path.
    ///
    /// # Behavior
    /// 1. Creates the initial candidate path with ".mp4" extension
    /// 2. Checks if the candidate file exists:
    ///    - If it doesn't exist, returns the candidate path
    ///    - If it does exist, appends an incrementing counter to the stem until a unique name is found
    fn build_output_file(&self, dir: &Path, stem: &OsStr) -> PathBuf {
        debug!("Starting build_output_file function");
        debug!("Directory: {:?}, Stem: {:?}", dir, stem);

        // Create the initial candidate path by joining the directory and stem, then setting its extension.
        let mut candidate = dir.join(stem);
        candidate.set_extension("mp4");
        debug!("Initial candidate path: {:?}", candidate);

        // If the candidate already exists, generate a new stem by appending an incrementing counter.
        if candidate.exists() {
            debug!("Candidate path already exists: {:?}", candidate);
            let stem_str = stem.to_string_lossy();
            let mut counter = 1;
            loop {
                // Create a new candidate name by appending the counter
                let new_stem = format!("{}_{}", stem_str, counter);
                let mut new_candidate = dir.join(new_stem);
                new_candidate.set_extension("mp4");
                if !new_candidate.exists() {
                    candidate = new_candidate;
                    break;
                }
                counter += 1;
            }
            debug!("Updated candidate path: {:?}", candidate);
        } else {
            debug!("Candidate path does not exist, using: {:?}", candidate);
        }

        debug!("Final output path: {:?}", candidate);
        candidate
    }
}

/// Creates a uniquely named directory, ensuring no existing directory with the same name.
///
/// This function attempts to create a directory with the given base name. If the directory
/// already exists, it appends an incrementing counter to the base name until a unique
/// directory is found.
///
/// # Parameters
/// - `parent`: The parent directory path where the new directory should be created.
/// - `base_name`: The base name of the directory to create.
///
/// # Returns
/// - `Result<PathBuf>`: The path to the newly created directory on success.
///
/// # Notes
/// - If the directory with `base_name` already exists, a numeric suffix is added
///   (e.g., `name_1`, `name_2`, etc.) until a unique name is found.
fn create_unique_dir(parent: &Path, base_name: &str) -> Result<PathBuf> {
    // Check if the directory with the base name already exists.
    let base_path = parent.join(base_name);
    if !base_path.exists() {
        fs::create_dir_all(&base_path)
            .with_context(|| format!("Failed to create output directory {:?}", base_path))?;
        return Ok(base_path);
    }

    // Otherwise, append an incrementing number until a free directory is found.
    let mut counter = 1;
    let output_path = loop {
        let candidate_name = format!("{}_{counter}", base_name);
        let candidate_path = parent.join(&candidate_name);
        if !candidate_path.exists() {
            break candidate_path;
        }
        counter += 1;
    };

    fs::create_dir_all(&output_path)
        .with_context(|| format!("Failed to create output directory {:?}", output_path))?;
    Ok(output_path)
}

/// Creates an explicit output directory, ensuring all necessary parent directories exist.
///
/// This function validates and creates the specified output directory structure.
///
/// # Parameters
/// - `output_dir`: The path to the output directory to be created.
///
/// # Returns
/// - `Result<PathBuf>`: The created directory path on success, or an error if creation fails.
///
/// # Notes
/// - Creates parent directories if they do not already exist.
fn create_explicit_output_directory(output_dir: &str) -> Result<PathBuf> {
    debug!("Output directory provided: {:?}", output_dir);
    let output_path = Path::new(output_dir);

    debug!("Creating output directory: {:?}", output_path);
    fs::create_dir_all(output_path).context("Failed to create output directory")?;
    Ok(output_path.to_path_buf())
}
