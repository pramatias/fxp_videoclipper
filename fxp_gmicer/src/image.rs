use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use log::{debug, warn};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

/// Processes images using GMIC with specified arguments and outputs to a directory.
///
/// This function handles image processing by validating input parameters and executing
/// GMIC operations on the provided images.
///
/// # Parameters
/// - `images`: Collection of images to process, mapped by unique identifiers.
/// - `gmic_args`: Command-line arguments for GMIC processing.
/// - `output_directory`: Path to the directory where processed images will be saved.
///
/// # Returns
/// - `Result<()>`: Indicates successful execution or returns an error if any issues occur.
///
/// # Notes
/// - The function logs debug information about the processing steps.
/// - Validates that the output directory exists and that images are provided.
pub fn image_processing(
    images: &BTreeMap<u32, PathBuf>,
    gmic_args: &[String],
    output_directory: &PathBuf,
) -> Result<()> {
    if !output_directory.exists() {
        anyhow::bail!("Error: The specified output directory does not exist.");
    }

    if images.is_empty() {
        anyhow::bail!("No valid images provided.");
    }

    debug!("Found {} images. Starting processing...", images.len());
    debug!("GMIC arguments: {:?}", gmic_args);
    debug!("Output directory: {:?}", output_directory);

    let gmic_args_ref: Vec<&str> = gmic_args.iter().map(String::as_str).collect();
    process_all_images(images, output_directory, &gmic_args_ref)
        .context("Failed to process all images")?;

    debug!("All images processed successfully!");

    Ok(())
}

/// Processes a collection of images with specified GMIC arguments and outputs them to a target directory.
///
/// This function handles a map of images, applies processing with the given GMIC arguments,
/// and saves the results to the output directory while tracking progress and handling interruptions.
///
/// # Parameters
/// - `images`: A map of image numbers to their respective file paths.
/// - `output_dir`: The directory where processed images will be saved.
/// - `gmic_args`: Command-line arguments to be used for GMIC processing.
///
/// # Returns
/// - `Result<()>`: Indicates success or failure of the entire processing operation.
///
/// # Notes
/// - The function supports handling of interrupts (Ctrl+C) to stop processing prematurely.
/// - A progress bar tracks the processing of each image.
/// - Each image is processed using the provided GMIC tool arguments.
/// - Output filenames follow the format: `image_{number}{extension}`.
/// - If an error occurs during image processing, it is logged and processing continues with the next image.
fn process_all_images(
    images: &BTreeMap<u32, PathBuf>,
    output_dir: &Path,
    gmic_args: &[&str],
) -> Result<()> {
    debug!(
        "Processing {} images to output directory: {:?}",
        images.len(),
        output_dir
    );
    debug!("GMIC arguments: {:?}", gmic_args);

    let running = Arc::new(AtomicBool::new(true));
    let r = Arc::clone(&running);

    ctrlc::set_handler(move || {
        warn!("Interrupt signal received. Stopping image processing...");
        r.store(false, Ordering::SeqCst);
    })
    .context("Error setting Ctrl+C handler")?;

    let pb = ProgressBar::new(images.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
            )
            .unwrap(),
    );

    for (index, (image_number, image_path)) in images.iter().enumerate() {
        if !running.load(Ordering::SeqCst) {
            warn!(
                "Processing interrupted by user at image {}. Exiting...",
                index + 1
            );
            break;
        }

        debug!("Processing image {}: {:?}", image_number, image_path);

        let extension = image_path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("png");

        debug!("File extension for image {}: {}", image_number, extension);

        let output_file = output_dir.join(format!("image_{:04}.{}", image_number, extension));

        debug!(
            "Output file path for image {}: {:?}",
            image_number, output_file
        );

        if let Err(e) = process_image(image_path, &output_file, gmic_args) {
            warn!("Error processing image {}: {:?}", image_number, e);
        }

        pb.inc(1);
        debug!("Finished processing image {}", image_number);
    }

    pb.finish_with_message("Processing complete!");
    debug!("All images processed successfully!");

    Ok(())
}

/// Runs GMIC command on a single image file, suppressing output.
///
/// This function executes a GMIC command with specified arguments on a given image file.
///
/// # Parameters
/// - `input`: Input image path as a `Path` reference.
/// - `output`: Output image path as a `Path` reference.
/// - `gmic_args`: Slice of GMIC command arguments as strings.
///
/// # Returns
/// - `Result<()>`: Returns `Ok(())` on successful processing, or an error if processing fails.
///
/// # Notes
/// - Suppresses both `stdout` and `stderr` during command execution.
/// - Does not handle GMIC installation or setup; assumes GMIC is already available in the system PATH.
fn process_image(input: &Path, output: &Path, gmic_args: &[&str]) -> Result<()> {
    // Debug: Print the input and output paths
    debug!(
        "Processing image: input = {:?}, output = {:?}",
        input, output
    );

    // Debug: Print the GMIC arguments being used
    debug!("GMIC arguments: {:?}", gmic_args);

    // Run the GMIC command
    let status = StdCommand::new("gmic")
        .arg(input)
        .args(gmic_args)
        .arg("-output")
        .arg(output)
        .stdout(std::process::Stdio::null()) // Suppress stdout
        .stderr(std::process::Stdio::null()) // Suppress stderr
        .status()
        .with_context(|| format!("Failed to execute GMIC command for input: {:?}", input))?;

    // Debug: Print the status of the GMIC command
    debug!("GMIC command executed with status: {}", status);

    if !status.success() {
        // Return an error if the GMIC command failed
        anyhow::bail!("GMIC command failed for input: {:?}", input);
    } else {
        // Debug: Print a success message if the GMIC command succeeded
        debug!("Successfully processed image: {:?}", input);
    }

    Ok(())
}
