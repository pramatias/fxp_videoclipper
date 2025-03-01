use anyhow::{Context, Result};
use image::{DynamicImage, GenericImageView, Rgba, RgbaImage};
use indicatif::{ProgressBar, ProgressStyle};
// use log::{debug, info};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Merges and processes images from two directories, blending them with specified opacity.
///
/// This function takes two directories containing images and an opacity value. It processes each image pair by resizing the second image to match the first's dimensions, blending them with the specified opacity, and saving the result in an output directory.
///
/// # Parameters
/// - `directory1`: Path to the first directory containing images to process.
/// - `directory2`: Path to the second directory containing images to process.
/// - `opacity`: A float between 0.0 and 1.0 that determines the transparency of the overlay image.
///
/// # Returns
/// - `Result<()>`: Indicates success or failure of the operation.
///
/// # Notes
/// - The function creates an output directory based on `directory1` and the opacity value.
/// - Images are resized using the Lanczos3 filtering for high-quality resizing.
/// - Filenames are padded with leading zeros to maintain a consistent naming scheme.
/// - A progress bar is displayed to track the processing of images.
/// - If no matching images are found in `directory2`, a debug message is logged, and processing continues with available pairs.
/// Refactored function to merge images using parameters from setup.
pub fn merge_all_images<P: AsRef<Path>>(
    directory1_files: &BTreeMap<u32, PathBuf>,
    directory2_files: &BTreeMap<u32, PathBuf>,
    output_directory: P,
    opacity: f32,
    total_images: usize,
) -> Result<()> {
    let output_directory = output_directory.as_ref();

    let pb = ProgressBar::new(total_images as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
            )
            .unwrap(),
    );

    for (index, file1) in directory1_files.iter().take(total_images) {
        if let Some(file2) = directory2_files.get(index) {
            let img1 = image::open(file1).context("Failed to open image from directory1")?;
            let img2 = image::open(file2).context("Failed to open image from directory2")?;

            let img2_resized = img2.resize(
                img1.width(),
                img1.height(),
                image::imageops::FilterType::Lanczos3,
            );
            let blended = blend_images(&img1, &img2_resized, opacity);

            let padding = 4;
            let output_path = format!(
                "{}/image_{:0padding$}.png",
                output_directory.display(),
                index,
                padding = padding
            );
            blended
                .save(&output_path)
                .context("Failed to save blended image")?;

            pb.inc(1);
        }
    }

    pb.finish_with_message("All images merged successfully!");

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
