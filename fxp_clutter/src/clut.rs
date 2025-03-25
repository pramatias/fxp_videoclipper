use anyhow::Result;
use ctrlc;
use indicatif::{ProgressBar, ProgressStyle};
use log::debug;
use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command as StdCommand;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::SystemTime;

/// Applies a Color Lookup Table (CLUT) to multiple images and saves the results.
///
/// This function processes a collection of images, applying the specified CLUT to each,
/// while providing progress tracking and supporting graceful interruption.
///
/// # Parameters
/// - `clut_path`: Path to the CLUT file to apply.
/// - `images`: A `BTreeMap` containing image IDs mapped to their file paths.
/// - `output_dir`: Directory where processed images will be saved.
///
/// # Returns
/// - `Result<()>`: Indicates success or failure of the operation.
///
/// # Notes
/// - The function displays a progress bar showing processing status.
/// - Processing can be interrupted with `Ctrl+C`, gracefully terminating the operation.
/// - Debug messages and timing information are logged during execution.
pub fn clut_all_images(
    clut_path: &PathBuf,
    images: &BTreeMap<u32, PathBuf>,
    output_dir: &Path,
) -> Result<()> {
    let pb = ProgressBar::new(images.len() as u64);
    pb.set_style(ProgressStyle::default_bar().template(
        "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta_precise})",
    )?);

    debug!("Starting to process images...");
    let start_time = SystemTime::now();

    let is_terminated = Arc::new(AtomicBool::new(false));
    let is_terminated_clone = Arc::clone(&is_terminated);

    ctrlc::set_handler(move || {
        is_terminated_clone.store(true, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl+C handler");

    for (index, input_image) in images.values().enumerate() {
        if is_terminated.load(Ordering::SeqCst) {
            debug!("Process interrupted by user. Exiting...");
            break;
        }

        debug!("Processing image {}: {:?}", index + 1, input_image);
        clut_image(input_image, clut_path, output_dir, &is_terminated);
        pb.inc(1);
        debug!("Image {} processed successfully.", index + 1);
    }

    pb.finish_with_message("Processing complete!");
    debug!(
        "All images processed successfully in {:?}.",
        start_time.elapsed()?
    );

    Ok(())
}

/// Applies a Color Lookup Table (CLUT) to an image and saves the result.
///
/// This function transforms the source image using a specified CLUT and saves
/// the output in a designated directory. It supports termination signals and
/// handles errors gracefully.
///
/// # Parameters
/// - `input_image`: Path to the source image file to process.
/// - `clut_path`: Path to the CLUT file to apply.
/// - `output_dir`: Directory where the processed image will be saved.
/// - `is_terminated`: Flag to check if processing should be stopped.
///
/// # Returns
/// - `Result<()>`: Returns `Ok(())` on success or an error if processing fails.
///
/// # Notes
/// - The function checks for a termination signal before proceeding with processing.
/// - Uses ImageMagick's `convert` command to apply the CLUT.
/// - If the command fails, an error message is printed to stderr.
fn clut_image(
    input_image: &Path,
    clut_path: &Path,
    output_dir: &Path,
    is_terminated: &Arc<AtomicBool>,
) {
    let file_name = input_image.file_name().unwrap();
    let output_path = output_dir.join(file_name);

    // If termination was requested, stop processing
    if is_terminated.load(Ordering::SeqCst) {
        debug!(
            "Skipping {} due to termination request.",
            file_name.to_string_lossy()
        );
        return;
    }

    // Apply the CLUT to the source image
    let status = StdCommand::new("convert")
        .arg(clut_path)
        .arg(input_image)
        .arg("-clut")
        .arg(&output_path)
        .status()
        .expect("Failed to run convert command");

    if !status.success() {
        eprintln!("Failed to apply CLUT: {:?}", input_image);
    }
}
