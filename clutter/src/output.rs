use anyhow::{Context, Result};
use log::debug;
use rand::distributions::Alphanumeric;
use rand::Rng;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

/// Creates the output directory either explicitly (if provided) or auto-generates one.
pub fn create_output_directory(
    input_path: &Path,
    output_directory: Option<&str>,
) -> Result<PathBuf> {
    match output_directory {
        Some(dir) => create_explicit_output_directory(dir),
        None => output_directory_auto_generated(input_path),
    }
}

/// Automatically generates an output directory path by appending a random suffix to a base name
/// that uses the input directory name with a "_clutted" suffix.
fn output_directory_auto_generated(input_path: &Path) -> Result<PathBuf> {
    let base_directory_name = format!(
        "{}_clutted",
        input_path
            .file_name()
            .unwrap_or_else(|| OsStr::new("input"))
            .to_string_lossy()
    );

    let output_path = loop {
        // Generate a candidate name by appending two random characters.
        let candidate_name = generate_random_name(OsStr::new(&base_directory_name));
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

// This function generates a random name by appending two random alphanumeric characters
// to the given base name.
fn generate_random_name(base: &OsStr) -> String {
    let mut rng = rand::thread_rng();
    let random_suffix: String = (0..2).map(|_| rng.sample(Alphanumeric) as char).collect();
    format!("{}{}", base.to_string_lossy(), random_suffix)
}

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
