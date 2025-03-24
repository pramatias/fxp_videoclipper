use anyhow::{anyhow, Context, Result};
use image::{DynamicImage, GenericImageView, Rgba, RgbaImage};
use indicatif::{ProgressBar, ProgressStyle};
use log::debug;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Merges images from two directories into a single output directory.
///
/// This function combines pairs of images from two directories, blending them with specified opacity.
/// It ensures consistent output formatting and handles errors gracefully.
///
/// # Parameters
/// - `directory1_files`: BTreeMap of images from the first directory
/// - `directory2_files`: BTreeMap of images from the second directory
/// - `output_directory`: Path to save the merged images
/// - `opacity`: Opacity level for blending images
/// - `total_images`: Total number of images to process
///
/// # Returns
/// - `Result<()>`: Indicates success or failure of the merge operation
///
/// # Notes
/// - Images are resized to match before blending
/// - Output filenames match the first directory's files
/// - Both directories must have matching image indices for merging
pub fn merge_all_images<P: AsRef<Path>>(
    directory1_files: &BTreeMap<u32, PathBuf>,
    directory2_files: &BTreeMap<u32, PathBuf>,
    output_directory: P,
    opacity: f32,
    total_images: usize,
) -> Result<()> {
    let output_directory = output_directory.as_ref();
    debug!("Starting image merge with opacity: {}", opacity);
    debug!("Output directory: {:?}", output_directory);
    debug!("Total images to process: {}", total_images);

    let pb = ProgressBar::new(total_images as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
            )
            .unwrap(),
    );

    debug!("Beginning image processing loop...");
    for (index, file1) in directory1_files.iter().take(total_images) {
        debug!("Processing index: {}", index);
        debug!("Directory1 file: {:?}", file1);

        if let Some(file2) = directory2_files.get(index) {
            debug!("Found matching file in directory2: {:?}", file2);

            // Load images
            debug!("Loading images...");
            let img1 = image::open(file1)
                .context("Failed to open image from directory1")
                .map_err(|e| {
                    debug!("Error opening {:?}: {}", file1, e);
                    e
                })?;

            let img2 = image::open(file2)
                .context("Failed to open image from directory2")
                .map_err(|e| {
                    debug!("Error opening {:?}: {}", file2, e);
                    e
                })?;

            // Resize and blend
            debug!("Resizing image2 to match image1 dimensions...");
            let img2_resized = img2.resize(
                img1.width(),
                img1.height(),
                image::imageops::FilterType::Lanczos3,
            );

            debug!("Blending images with opacity: {}", opacity);
            let blended = blend_images(&img1, &img2_resized, opacity);

            // Save result
            let output_path = output_directory.join(file1.file_name().ok_or_else(|| {
                debug!("Failed to get filename from {:?}", file1);
                anyhow!("Failed to get file name from directory1")
            })?);

            debug!("Saving blended image to: {:?}", output_path);
            blended
                .save(&output_path)
                .context("Failed to save blended image")
                .map_err(|e| {
                    debug!("Error saving to {:?}: {}", output_path, e);
                    e
                })?;

            pb.inc(1);
            debug!("Processed {} images", pb.position());
        } else {
            debug!("No matching file in directory2 for index {}", index);
        }
    }

    pb.finish_with_message("All images merged successfully!");
    debug!("Merge operation completed successfully");

    Ok(())
}

/// Blends two images together with the specified opacity.
///
/// The opacity parameter controls the influence of the second image, where:
/// - `0.0` means only the first image is visible.
/// - `1.0` means only the second image is visible.
/// - Values between `0.0` and `1.0` create a blend between the two images.
///
/// # Arguments
/// * `img1` - The first image to blend.
/// * `img2` - The second image to blend.
/// * `opacity` - The opacity value (between `0.0` and `1.0`).
///
/// # Returns
/// The blended image as an `RgbaImage`.
fn blend_images(img1: &DynamicImage, img2: &DynamicImage, opacity: f32) -> RgbaImage {
    // debug!("Starting blend_images function");
    // debug!("Opacity: {:.2}", opacity);

    let (width, height) = img1.dimensions();
    // debug!("Image dimensions: {}x{}", width, height);

    let mut blended = RgbaImage::new(width, height);
    // debug!("Created a new RgbaImage for the blended result");

    for y in 0..height {
        for x in 0..width {
            let px1 = img1.get_pixel(x, y);
            let px2 = img2.get_pixel(x, y);

            // debug!("Pixel at ({}, {}): img1={:?}, img2={:?}", x, y, px1, px2);

            let r = ((px1[0] as f32) * (1.0 - opacity) + (px2[0] as f32) * opacity) as u8;
            let g = ((px1[1] as f32) * (1.0 - opacity) + (px2[1] as f32) * opacity) as u8;
            let b = ((px1[2] as f32) * (1.0 - opacity) + (px2[2] as f32) * opacity) as u8;
            let a = 255;

            // debug!("Blended pixel at ({}, {}): R={}, G={}, B={}, A={}", x, y, r, g, b, a);

            blended.put_pixel(x, y, Rgba([r, g, b, a]));
        }
    }

    // debug!("Finished blending images");
    blended
}
