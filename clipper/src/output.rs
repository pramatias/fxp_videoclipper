use rand::{distributions::Alphanumeric, Rng};
//use anyhow::{anyhow, Context, Result};
use log::debug;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

pub fn create_output_directory(
    input_dir: &Path,
    mp3_path: Option<&Path>,
    output_path: Option<PathBuf>,
) -> PathBuf {
    debug!("Starting create_output_filename function");

    // If an output path is explicitly provided, return it as-is.
    if let Some(out_path) = output_path {
        debug!("Output path provided: {:?}", out_path);
        return out_path;
    }

    // If we have an MP3 path, use its parent directory and file stem.
    if let Some(mp3) = mp3_path {
        debug!("MP3 path provided: {:?}", mp3);
        // Use the MP3 file's parent directory, or current directory if unavailable.
        let parent = mp3.parent().unwrap_or_else(|| Path::new("."));
        debug!("Using parent directory: {:?}", parent);
        // Use the file stem (filename without extension), or "input" if missing.
        let stem = mp3.file_stem().unwrap_or_else(|| OsStr::new("input"));
        debug!("Using file stem: {:?}", stem);
        return build_output_file(parent, stem);
    }

    debug!(
        "No MP3 path provided, using input directory: {:?}",
        input_dir
    );
    // Otherwise, use the input directory's name. Place the output file
    // in the same directory as the input directory's parent.
    let parent = input_dir.parent().unwrap_or_else(|| Path::new("."));
    debug!("Using parent directory: {:?}", parent);
    // Use the input directory's final component as the base name.
    let stem = input_dir.file_name().unwrap_or_else(|| OsStr::new("input"));
    debug!("Using file stem: {:?}", stem);
    build_output_file(parent, stem)
}

fn generate_random_name(base: &OsStr) -> String {
    let mut rng = rand::thread_rng();
    let random_suffix: String = (0..2).map(|_| rng.sample(Alphanumeric) as char).collect();
    format!("{}{}", base.to_string_lossy(), random_suffix)
}

// Determines the output file path using a given directory and file stem.
fn build_output_file(dir: &Path, stem: &OsStr) -> PathBuf {
    debug!("Starting build_output_file function");
    debug!("Directory: {:?}, Stem: {:?}", dir, stem);

    // Create the initial candidate path by joining the directory and stem.
    let mut candidate = dir.join(stem);
    candidate.set_extension("mp4");
    debug!("Initial candidate path: {:?}", candidate);

    // Check if the candidate exists; if so, append two random characters.
    if candidate.exists() {
        debug!("Candidate path already exists: {:?}", candidate);
        let new_stem = generate_random_name(stem);
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
