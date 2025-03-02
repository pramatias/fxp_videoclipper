use anyhow::{Context, Result};
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

pub struct ClipperOutput;
impl ModeOutput for ClipperOutput {
    // Changed Option<&Path> to Option<PathBuf> to avoid lifetime issues.
    type Input = (PathBuf, Option<PathBuf>, Option<String>);
    fn create_output_directory(&self, input: Self::Input) -> Result<PathBuf> {
        let (_dir, _maybe_path, maybe_str) = input;
        println!("Running Clipper with string: {:?}", maybe_str);
        Ok(PathBuf::from(format!("clipper_output_{:?}", maybe_str)))
    }
}

pub struct GmicerOutput;
impl ModeOutput for GmicerOutput {
    type Input = (PathBuf, Option<String>);
    fn create_output_directory(&self, input: Self::Input) -> Result<PathBuf> {
        let (_dir, maybe_str) = input;
        println!("Running Gmicer with string: {:?}", maybe_str);
        Ok(PathBuf::from(format!("gmicer_output_{:?}", maybe_str)))
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
