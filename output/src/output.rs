use anyhow::{anyhow, Context, Result};
use log::debug;
use rand::distributions::Alphanumeric;
use rand::thread_rng;
use rand::Rng;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

pub use modes::Modes;

// Define a trait that creates an output directory and returns its PathBuf.
pub trait ModeOutput {
    type Input;
    fn create_output_directory(&self, input: Self::Input) -> Result<PathBuf>;
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

    fn create_output_directory(&self, input: Self::Input) -> Result<PathBuf> {
        let (input_path, output_directory) = input;
        match output_directory.as_deref() {
            Some(dir) => self.create_explicit_output_directory(dir),
            None => self.output_directory_auto_generated(&input_path),
        }
    }
}

pub struct SamplerOutput;
impl ModeOutput for SamplerOutput {
    type Input = (PathBuf, Option<String>);

    /// Creates the output directory either explicitly (if provided) or auto-generates one.
    /// The auto-generated directory will be named "sample_frames" or if it exists, a unique name
    /// with a random suffix.
    fn create_output_directory(&self, input: Self::Input) -> Result<PathBuf> {
        // Destructure the tuple into `input_path` and `output_directory`
        let (input_path, output_directory) = input;

        match output_directory {
            Some(dir) => self.create_explicit_output_directory(&dir),
            None => self.output_directory_auto_generated(&input_path),
        }
    }
}

pub struct ClutterOutput;
impl ModeOutput for ClutterOutput {
    // Adjusted Input to match the new pattern: (PathBuf, Option<String>)
    type Input = (PathBuf, Option<String>);
    fn create_output_directory(&self, input: Self::Input) -> Result<PathBuf> {
        let (input_path, output_directory) = input;
        match output_directory.as_deref() {
            Some(dir) => self.create_explicit_output_directory(dir),
            None => self.output_directory_auto_generated(&input_path),
        }
    }
}

pub struct MergerOutput;
impl ModeOutput for MergerOutput {
    // The input is a tuple: (input_path, output_directory, merge_value)
    type Input = (PathBuf, Option<String>, f32);
    fn create_output_directory(&self, input: Self::Input) -> Result<PathBuf> {
        let (input_path, output_directory, merge_value) = input;
        match output_directory.as_deref() {
            Some(dir) => self.create_explicit_output_directory(dir),
            None => self.output_directory_auto_generated(&input_path, merge_value),
        }
    }
}

pub struct GmicerOutput;
impl ModeOutput for GmicerOutput {
    type Input = (PathBuf, Vec<String>, Option<String>);
    fn create_output_directory(&self, input: Self::Input) -> Result<PathBuf> {
        let (input_path, gmic_args, output_directory) = input;
        match output_directory.as_deref() {
            Some(dir) => self.create_explicit_output_directory(dir),
            None => self.output_directory_auto_generated(&input_path, &gmic_args),
        }
    }
}

pub struct ClipperOutput;
impl ModeOutput for ClipperOutput {
    // Input: (input_path, optional custom directory as PathBuf, optional string parameter)
    type Input = (PathBuf, Option<PathBuf>, Option<String>);

    fn create_output_directory(&self, input: Self::Input) -> Result<PathBuf> {
        let (input_path, maybe_path, maybe_explicit) = input;
        match maybe_explicit.as_deref() {
            // If an explicit output directory is provided, use it.
            Some(explicit) => self.create_explicit_output_file(explicit),
            // Otherwise, auto-generate the output directory, passing the optional mp3_path.
            None => self.output_directory_auto_generated(&input_path, maybe_path.as_deref()),
        }
    }
}

//rest of functions
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

        let output_path = loop {
            // Generate a candidate name by appending two random characters.
            let candidate_name = self.generate_random_name(OsStr::new(&base_directory_name));
            let candidate_path = input_path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(candidate_name);

            if !candidate_path.exists() {
                break candidate_path;
            }
        };

        fs::create_dir_all(&output_path)
            .with_context(|| format!("Failed to create output directory {:?}", output_path))?;
        Ok(output_path)
    }

    /// Creates an output directory using the explicitly provided path.
    fn create_explicit_output_directory(&self, output_dir: &str) -> Result<PathBuf> {
        debug!("Output directory provided: {:?}", output_dir);
        let output_path = Path::new(output_dir);
        if output_path.exists() {
            debug!("Output directory exists, removing it: {:?}", output_path);
            fs::remove_dir_all(output_path)
                .with_context(|| "Failed to remove existing output directory")?;
        }
        debug!("Creating output directory: {:?}", output_path);
        fs::create_dir_all(output_path).with_context(|| "Failed to create output directory")?;
        Ok(output_path.to_path_buf())
    }

    /// Generates a random directory name by appending two random alphanumeric characters.
    fn generate_random_name(&self, base: &OsStr) -> String {
        let mut rng = rand::thread_rng();
        let random_suffix: String = (0..2).map(|_| rng.sample(Alphanumeric) as char).collect();
        format!("{}{}", base.to_string_lossy(), random_suffix)
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
                let candidate_name = self.generate_random_name(OsStr::new(base_directory_name));
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

    /// Generates a random name by appending two random alphanumeric characters to the base name.
    fn generate_random_name(&self, base: &OsStr) -> String {
        let mut rng = rand::thread_rng();
        let random_suffix: String = (0..2).map(|_| rng.sample(Alphanumeric) as char).collect();
        format!("{}{}", base.to_string_lossy(), random_suffix)
    }

    /// Creates an explicit output directory.
    fn create_explicit_output_directory(&self, output_dir: &str) -> Result<PathBuf> {
        debug!("Output directory provided: {:?}", output_dir);
        let output_path = Path::new(output_dir);
        if output_path.exists() {
            debug!("Output directory exists, removing it: {:?}", output_path);
            fs::remove_dir_all(output_path)
                .context("Failed to remove existing output directory")?;
        }
        debug!("Creating output directory: {:?}", output_path);
        fs::create_dir_all(output_path).context("Failed to create output directory")?;
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

        let output_path = loop {
            // Generate a candidate name by appending two random characters.
            let candidate_name = self.generate_random_name(OsStr::new(&base_directory_name));
            let candidate_path = input_path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(candidate_name);

            if !candidate_path.exists() {
                break candidate_path;
            }
        };

        fs::create_dir_all(&output_path)
            .with_context(|| format!("Failed to create output directory {:?}", output_path))?;
        Ok(output_path)
    }

    // Helper method to generate a random name by appending two alphanumeric characters.
    fn generate_random_name(&self, base: &OsStr) -> String {
        let mut rng = rand::thread_rng();
        let random_suffix: String = (0..2).map(|_| rng.sample(Alphanumeric) as char).collect();
        format!("{}{}", base.to_string_lossy(), random_suffix)
    }

    // This method creates an explicit output directory.
    fn create_explicit_output_directory(&self, output_dir: &str) -> Result<PathBuf> {
        debug!("Output directory provided: {:?}", output_dir);
        let output_path = Path::new(output_dir);
        if output_path.exists() {
            debug!("Output directory exists, removing it: {:?}", output_path);
            fs::remove_dir_all(output_path)
                .context("Failed to remove existing output directory")?;
        }
        debug!("Creating output directory: {:?}", output_path);
        fs::create_dir_all(output_path).context("Failed to create output directory")?;
        Ok(output_path.to_path_buf())
    }
}
impl ClutterOutput {
    /// Automatically generates an output directory path by appending a random suffix to a base name
    /// that uses the input directory name with a "_clutted" suffix.
    fn output_directory_auto_generated(&self, input_path: &Path) -> Result<PathBuf> {
        let base_directory_name = format!(
            "{}_clutted",
            input_path
                .file_name()
                .unwrap_or_else(|| OsStr::new("input"))
                .to_string_lossy()
        );

        let output_path = loop {
            // Generate a candidate name by appending two random characters.
            let candidate_name = self.generate_random_name(OsStr::new(&base_directory_name));
            let candidate_path = input_path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(candidate_name);

            if !candidate_path.exists() {
                break candidate_path;
            }
        };

        fs::create_dir_all(&output_path)
            .with_context(|| format!("Failed to create output directory {:?}", output_path))?;
        Ok(output_path)
    }

    /// Generates a random name by appending two random alphanumeric characters to the given base name.
    fn generate_random_name(&self, base: &OsStr) -> String {
        let mut rng = thread_rng();
        let random_suffix: String = (0..2).map(|_| rng.sample(Alphanumeric) as char).collect();
        format!("{}{}", base.to_string_lossy(), random_suffix)
    }

    /// Creates the output directory explicitly. If it exists, it is removed first.
    fn create_explicit_output_directory(&self, output_dir: &str) -> Result<PathBuf> {
        debug!("Output directory provided: {:?}", output_dir);
        let output_path = Path::new(output_dir);
        if output_path.exists() {
            debug!("Output directory exists, removing it: {:?}", output_path);
            fs::remove_dir_all(output_path)
                .context("Failed to remove existing output directory")?;
        }
        debug!("Creating output directory: {:?}", output_path);
        fs::create_dir_all(output_path).context("Failed to create output directory")?;
        Ok(output_path.to_path_buf())
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

    fn output_directory_auto_generated(
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
impl GmicerOutput {
    /// Creates the output directory using an explicit path.
    fn create_explicit_output_directory(&self, output_dir: &str) -> Result<PathBuf> {
        debug!("Output directory provided: {:?}", output_dir);
        let output_path = Path::new(output_dir);
        if output_path.exists() {
            debug!("Output directory exists, removing it: {:?}", output_path);
            fs::remove_dir_all(output_path)
                .context("Failed to remove existing output directory")?;
        }
        debug!("Creating output directory: {:?}", output_path);
        fs::create_dir_all(output_path).context("Failed to create output directory")?;
        Ok(output_path.to_path_buf())
    }

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

        let output_path = loop {
            let candidate_name = self.generate_random_name(OsStr::new(&base_directory_name));
            let candidate_path = input_path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(candidate_name);

            debug!("Generated candidate output path: {:?}", candidate_path);

            if !candidate_path.exists() {
                break candidate_path;
            }
        };

        debug!("Creating output directory: {:?}", output_path);
        fs::create_dir_all(&output_path)
            .with_context(|| format!("Failed to create output directory {:?}", output_path))?;
        debug!("Output directory created successfully: {:?}", output_path);

        Ok(output_path)
    }

    /// Generates a random name by appending two random alphanumeric characters
    /// to the given base.
    fn generate_random_name(&self, base: &OsStr) -> String {
        let mut rng = rand::thread_rng();
        let random_suffix: String = (0..2).map(|_| rng.sample(Alphanumeric) as char).collect();
        format!("{}{}", base.to_string_lossy(), random_suffix)
    }
}
