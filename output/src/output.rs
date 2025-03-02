use anyhow::{anyhow, Context, Result};
use log::debug;
use rand::distributions::Alphanumeric;
use rand::Rng;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use rand::thread_rng;

pub use modes::Modes; // Assuming Modes is defined in the `modes` module elsewhere

// Define a trait that creates an output directory and returns its PathBuf.
pub trait ModeOutput {
    type Input;
    fn create_output_directory(&self, input: Self::Input) -> Result<PathBuf>;
}

pub struct MergerOutput;
impl ModeOutput for MergerOutput {
    // The input is a tuple: (input_path, output_directory, merge_value)
    type Input = (PathBuf, Option<String>, f32);

    fn create_output_directory(&self, input: Self::Input) -> Result<PathBuf> {
        let (input_path, output_directory, merge_value) = input;
        // Convert Option<String> into Option<&str> using as_deref()
        self.create_output_directory_impl(&input_path, output_directory.as_deref(), merge_value)
    }
}

pub struct GmicerOutput;
impl ModeOutput for GmicerOutput {
    // The input is a tuple: (input_path, output_directory)
    type Input = (PathBuf, Option<String>);

    fn create_output_directory(&self, input: Self::Input) -> Result<PathBuf> {
        let (input_path, output_directory) = input;
        // Convert Option<String> to Option<&str> using as_deref() for consistency
        self.create_output_directory_impl(&input_path, output_directory.as_deref())
    }
}

// Enum to hold all the possible outputs.
pub enum Output {
    Exporter(ExporterOutput),
    Merger(MergerOutput),
    Sampler(SamplerOutput),
    Clutter(ClutterOutput),
    Clipper(ClipperOutput),
    Gmicer(GmicerOutput),
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

pub struct SamplerOutput;
impl ModeOutput for SamplerOutput {
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

pub struct ClutterOutput;
impl ModeOutput for ClutterOutput {
    // Adjusted Input to match the new pattern: (PathBuf, Option<String>)
    type Input = (PathBuf, Option<String>);

    fn create_output_directory(&self, input: Self::Input) -> Result<PathBuf> {
        let (dir, maybe_str) = input;
        // Convert Option<String> to Option<&str> using as_deref() for consistency.
        self.create_output_directory_impl(&dir, maybe_str.as_deref())
    }
}

pub struct ClipperOutput;
impl ModeOutput for ClipperOutput {
    // Input: (input_path, optional custom directory as PathBuf, optional string parameter)
    type Input = (PathBuf, Option<PathBuf>, Option<String>);

    fn create_output_directory(&self, input: Self::Input) -> Result<PathBuf> {
        let (input_path, maybe_path, maybe_str) = input;
        // Convert owned Option<PathBuf> to Option<&Path> and Option<String> to Option<&str>
        self.create_output_directory_impl(
            &input_path,
            maybe_path.as_deref(),
            maybe_str.as_deref(),
        )
    }
}

//rest of functions
impl MergerOutput {
    /// Internal implementation that calls the appropriate method based on the provided output directory.
    fn create_output_directory_impl(
        &self,
        input_path: &Path,
        output_directory: Option<&str>,
        merge_value: f32,
    ) -> Result<PathBuf> {
        match output_directory {
            Some(dir) => self.create_explicit_output_directory(dir),
            None => self.output_directory_auto_generated(input_path, merge_value),
        }
    }

    /// Creates an output directory using the explicitly provided path.
    fn create_explicit_output_directory(&self, output_dir: &str) -> Result<PathBuf> {
        log::debug!("Output directory provided: {:?}", output_dir);
        let output_path = Path::new(output_dir);
        if output_path.exists() {
            log::debug!("Output directory exists, removing it: {:?}", output_path);
            fs::remove_dir_all(output_path)
                .with_context(|| "Failed to remove existing output directory")?;
        }
        log::debug!("Creating output directory: {:?}", output_path);
        fs::create_dir_all(output_path).with_context(|| "Failed to create output directory")?;
        Ok(output_path.to_path_buf())
    }

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

    /// Generates a random directory name by appending two random alphanumeric characters.
    fn generate_random_name(&self, base: &OsStr) -> String {
        let mut rng = rand::thread_rng();
        let random_suffix: String = (0..2).map(|_| rng.sample(Alphanumeric) as char).collect();
        format!("{}{}", base.to_string_lossy(), random_suffix)
    }
}

impl ExporterOutput {
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
        log::debug!("Output directory provided: {:?}", output_dir);
        let output_path = Path::new(output_dir);
        if output_path.exists() {
            log::debug!("Output directory exists, removing it: {:?}", output_path);
            fs::remove_dir_all(output_path)
                .context("Failed to remove existing output directory")?;
        }
        log::debug!("Creating output directory: {:?}", output_path);
        fs::create_dir_all(output_path).context("Failed to create output directory")?;
        Ok(output_path.to_path_buf())
    }
}

impl SamplerOutput {
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
        fs::create_dir_all(output_path)
            .context("Failed to create output directory")?;
        Ok(output_path.to_path_buf())
    }
}
impl ClutterOutput {
    /// Creates the output directory either explicitly (if provided) or auto-generates one.
    pub fn create_output_directory_impl(&self, input_path: &Path, output_directory: Option<&str>) -> Result<PathBuf> {
        match output_directory {
            Some(dir) => self.create_explicit_output_directory(dir),
            None => self.output_directory_auto_generated(input_path),
        }
    }

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
        let random_suffix: String = (0..2)
            .map(|_| rng.sample(Alphanumeric) as char)
            .collect();
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
        fs::create_dir_all(output_path)
            .context("Failed to create output directory")?;
        Ok(output_path.to_path_buf())
    }
}
impl ClipperOutput {
    /// Creates the output directory or file path based on provided parameters.
    ///
    /// - If `output_path` is provided, it is returned (converted to a PathBuf).
    /// - If an MP3 path is provided, its parent directory and file stem are used.
    /// - Otherwise, the parent of the input directory and its file name are used.
    pub fn create_output_directory_impl(
        &self,
        input_dir: &Path,
        mp3_path: Option<&Path>,
        output_path: Option<&str>,
    ) -> Result<PathBuf> {
        debug!("Starting create_output_directory_impl function");

        // If an output path is explicitly provided, convert it to PathBuf and return.
        if let Some(out_path) = output_path {
            let path = PathBuf::from(out_path);
            debug!("Output path provided: {:?}", path);
            return Ok(path);
        }

        // If an MP3 path is provided, use its parent directory and file stem.
        if let Some(mp3) = mp3_path {
            debug!("MP3 path provided: {:?}", mp3);
            let parent = mp3.parent().unwrap_or_else(|| Path::new("."));
            debug!("Using parent directory: {:?}", parent);
            let stem = mp3.file_stem().unwrap_or_else(|| OsStr::new("input"));
            debug!("Using file stem: {:?}", stem);
            return Ok(self.build_output_file(parent, stem));
        }

        debug!(
            "No MP3 path provided, using input directory: {:?}",
            input_dir
        );
        // Use the input directory's parent and its file name.
        let parent = input_dir.parent().unwrap_or_else(|| Path::new("."));
        debug!("Using parent directory: {:?}", parent);
        let stem = input_dir.file_name().unwrap_or_else(|| OsStr::new("input"));
        debug!("Using file stem: {:?}", stem);
        Ok(self.build_output_file(parent, stem))
    }

    /// Generates a random name based on the given base string by appending two random alphanumeric characters.
    fn generate_random_name(&self, base: &OsStr) -> String {
        let mut rng = rand::thread_rng();
        let random_suffix: String = (0..2)
            .map(|_| rng.sample(Alphanumeric) as char)
            .collect();
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
    /// A public method that chooses between using an explicit output directory
    /// or auto-generating one (which requires GMIC arguments).
    pub fn create_output_directory_with_args(
        &self,
        input_path: &Path,
        gmic_args: &[String],
        output_directory: Option<&str>,
    ) -> Result<PathBuf> {
        match output_directory {
            Some(dir) => self.create_explicit_output_directory(dir),
            None => self.output_directory_auto_generated(input_path, gmic_args),
        }
    }

    /// This method is called from the trait implementation.
    /// Here we simply use the explicit output directory if provided,
    /// otherwise we return an error since no GMIC arguments are available.
    fn create_output_directory_impl(
        &self,
        input_path: &Path,
        output_directory: Option<&str>,
    ) -> Result<PathBuf> {
        match output_directory {
            Some(dir) => self.create_explicit_output_directory(dir),
            None => Err(anyhow!("GMIC arguments are required for auto-generated output directory")),
        }
    }

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
        fs::create_dir_all(output_path)
            .context("Failed to create output directory")?;
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
        let random_suffix: String = (0..2)
            .map(|_| rng.sample(Alphanumeric) as char)
            .collect();
        format!("{}{}", base.to_string_lossy(), random_suffix)
    }
}
