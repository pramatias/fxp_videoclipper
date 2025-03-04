use anyhow::{anyhow, Context, Result};
use clap::{ArgAction, Parser};
use clap_verbosity_flag::Verbosity;
use console::style;
use log::error;
use log::{debug, warn};
use std::path::Path;

use init::get_audio_file;
use init::{get_audio_dir, get_audio_duration, get_multiple_opacities};
use init::{get_duration, get_fps, get_opacity, get_pixel_upper_limit, get_sampling_number};
use init::{initialize_configuration, initialize_logger, load_default_configuration, Config};

use clipper::Clipper;
use clutter::Clutter;
use exporter::Exporter;
use gmicer::Gmicer;
use merger::Merger;
use sampler::Sampler;

use modes::Modes;

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

#[derive(Parser, Debug)]
#[command(
    author = "emporas",
    version = "0.5",
    about = "Easy videoclip creation with optional GMIC mode",
    long_about = None
)]
struct Cli {
    #[command(flatten)]
    verbose: Verbosity,

    /// Initialize configuration
    #[arg(long, help = "Initialize configuration", action = ArgAction::SetTrue)]
    init: bool,

    /// Input for video or directory (used for both video processing and GMIC mode)
    #[arg(
        short = 'i',
        long,
        help = "Input for video or directory. All modes",
        required = true
    )]
    input: Option<String>,
    // input: String,
    /// Output for directory or video (used for both video processing and GMIC mode)
    #[arg(short = 'o', long, help = "Output for directory or video. All modes")]
    output: Option<String>,

    // ===== GMIC Mode =====
    /// GMICER: Enable GMIC mode (applies a GMIC command to all images in the input directory)
    #[arg(short = 'g', long = "gmic", help = "Enable GMIC mode, GMICER", action = ArgAction::SetTrue)]
    gmic: bool,

    /// Arguments for GMIC command (only used if --gmic is enabled)
    #[arg(
        help = "Arguments for GMIC command (only used if --gmic is enabled)",
        num_args = 0..,
        allow_hyphen_values = true
    )]
    gmic_args: Option<Vec<String>>,

    // ===== Clipper Mode Options =====
    /// CLIPPER: Make the Videoclip
    #[arg(short = 'j', long, help = "Run in clutter mode", action = ArgAction::SetTrue)]
    clipper: bool,

    // ===== Clutter (Transfer Colors) Mode Options =====
    /// CLUTTER: Transfer colors using clut file
    #[arg(short = 'c', long, help = "Run in clutter mode", action = ArgAction::SetTrue)]
    clutter: bool,

    /// Path to the source image used for CLUT (Clutter mode)
    #[arg(
        short = 'l',
        long = "clut",
        help = "Path to the source image used for CLUT (Clutter)"
    )]
    clut_image: Option<String>,

    /// Opacity level for merging in clutter mode (0.0 - 1.0)
    #[arg(long = "clut-opacity", help = "Opacity level for merging in clutter mode (0.0 - 1.0) (Clutter)", value_parser = clap::value_parser!(f32))]
    clut_opacity: Option<f32>,

    /// Merge clutted images with original with many opacities (e.g. 0.25, 0.5, 0.75)
    #[arg(long = "clut-multiple", help = "Merge clutted images with original with many opacities (Clutter)", action = ArgAction::SetTrue)]
    clut_multiple: bool,

    /// Run the merging process after applying CLUT (Clutter mode)
    #[arg(long = "clut-merge", help = "Run the merging process after applying CLUT (Clutter)", action = ArgAction::SetTrue)]
    clut_merge: bool,

    // ===== Sampler Parameters =====
    /// SAMPLER: Sample frames evenly spaced across the video (Sampler)
    #[arg(short = 's', long = "sampler", help = "Sample frames evenly spaced across the video (Sampler)", action = ArgAction::SetTrue)]
    sampler: bool,

    /// Extract multiple frames evenly across the video (requires --sampler) (Sampler)
    #[arg(short = 'u', long = "multiple", help = "Extract multiple frames (requires --sampler) (Sampler)", action = ArgAction::SetTrue)]
    multiple: bool,

    /// Number of frames to extract (requires --multiple) (Sampler)
    #[arg(short = 'n', long = "number", help = "Number of frames to extract (requires --multiple) (Sampler)", value_parser = clap::value_parser!(usize))]
    number: Option<usize>,

    // ===== Exporter Parameters =====
    /// Exporter: Export frames based on duration and resolution
    #[arg(short = 'x', long = "exporter", help = "Export frames based on duration and resolution", action = ArgAction::SetTrue)]
    exporter: bool,

    /// Optional path to the MP3 file (Exporter, Sampler)
    #[arg(
        short = 'a',
        long = "audio",
        help = "Optional path to the MP3 file (Exporter, Sampler)"
    )]
    mp3: Option<String>,

    /// Duration in milliseconds to cut the video (Exporter, Sampler)
    #[arg(
        short,
        long,
        help = "Duration in milliseconds to cut the video (Exporter, Sampler)"
    )]
    duration: Option<String>,

    /// Frames per second to extract (Exporter)
    #[arg(short, long, help = "Frames per second to extract (Exporter)")]
    fps: Option<String>,

    /// Maximum upper limit for pixel resolution; video will be scaled relative to original (Exporter)
    #[arg(short, long = "pixel-limit", help = "Maximum upper limit for pixel resolution (Exporter)", value_parser = clap::value_parser!(u32))]
    pixel_upper_limit: Option<u32>,

    // ===== Merger Parameters =====
    /// Merger: Merge two directories of images
    #[arg(short = 'm', long = "merger", help = "Merge two directories of images", action = ArgAction::SetTrue)]
    merger: bool,

    /// Path to the first image directory (Merger)
    #[arg(short = '1', long, help = "Path to the first image directory (Merger)")]
    directory1: Option<String>,

    /// Path to the second image directory (Merger)
    #[arg(
        short = 'r',
        long = "second-directory",
        help = "Path to the second image directory (Merger)"
    )]
    directory2: Option<String>,

    /// Opacity level for merging (0.0 - 1.0) (Merger)
    #[arg(
        short = 't',
        long,
        help = "Opacity level for merging (Merger)",
        default_value = "0.5"
    )]
    opacity: Option<f32>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let verbosity_level = cli.verbose.log_level_filter();
    initialize_logger(verbosity_level).context("Failed to initialize logger")?;
    debug!(
        "{} {:?}",
        style("Logger initialized with verbosity:").cyan(),
        verbosity_level
    );

    if cli.init {
        debug!("{}", style("Initializing configuration...").yellow());
        initialize_configuration().context("Failed to initialize configuration")?;
        return Ok(());
    }

    let config = load_default_configuration().context("Failed to load default configuration")?;
    debug!("{}", style("Default configuration loaded").green());

    // Determine the mode based on CLI arguments
    let mode = if cli.exporter {
        Modes::Exporter
    } else if cli.merger {
        Modes::Merger
    } else if cli.sampler {
        Modes::Sampler
    } else if cli.clutter {
        Modes::Clutter
    } else if cli.clipper {
        Modes::Clipper
    } else if cli.gmic {
        Modes::Gmicer
    } else {
        error!("{}", style("No mode selected. Please use one of --exporter, --merger, --sampler, --clutter, --clipper, or --gmic.").red());
        std::process::exit(1);
    };

    // Match the mode and run the corresponding function
    match mode {
        Modes::Exporter => {
            debug!("{}", style("Running in exporter mode").blue());
            run_exporter(&cli, &config)?;
        }
        Modes::Merger => {
            debug!("{}", style("Running in merger mode").blue());
            run_merger(&cli, &config)?;
        }
        Modes::Sampler => {
            debug!("{}", style("Running in sampler mode").blue());
            run_sampler(&cli, &config)?;
        }
        Modes::Clutter => {
            debug!("{}", style("Running in clutter mode").blue());
            run_clutter(&cli, &config)?;
        }
        Modes::Clipper => {
            debug!("{}", style("Running in clipper mode").blue());
            run_clipper(&cli, &config)?;
        }
        Modes::Gmicer => {
            debug!("{}", style("Running in GMIC mode").blue());
            run_gmicer(&cli)?;
        }
    }

    debug!(
        "{}",
        style("Main function execution completed successfully").green()
    );
    Ok(())
}

fn run_clipper(cli: &Cli, config: &Config) -> Result<()> {
    // Ensure an input directory is provided.
    let input_dir = cli
        .input
        .as_ref()
        .ok_or_else(|| anyhow!("Input directory is required"))?;
    debug!("Input directory: {}", input_dir);

    // Extract the optional output path.
    let output_path = cli.output.clone();
    debug!("Output path: {:?}", output_path);

    // Get the audio file path using get_audio_file.
    let mp3_path = get_audio_file(cli.mp3.clone(), config).context("Failed to get audio file")?;
    debug!("Resolved MP3 path: {:?}", mp3_path);

    // Convert the Option<PathBuf> into an Option<String>.
    let mp3_path_str = mp3_path.as_ref().map(|p| p.to_string_lossy().into_owned());

    // Resolve the frames-per-second value.
    let cli_fps = cli
        .fps
        .clone()
        .map(|s| s.parse::<u32>().context("Invalid FPS value"))
        .transpose()?;
    let fps = get_fps(cli_fps, config).context("Failed to resolve FPS")?;
    debug!("Resolved FPS value: {}", fps);

    // let duration_arg = cli.duration.clone();

    // Get the audio duration using the mp3_path from get_audio_file.
    let duration =
        get_audio_duration(mp3_path_str.clone(), config).context("Failed to resolve duration")?;
    match duration {
        Some(d) => debug!("Final duration to use: {} milliseconds", d),
        None => debug!("Final duration to use: None"),
    }

    // Initialize the Clipper with the resolved parameters, including the mp3_path from get_audio_file.
    let clipper = Clipper::new(
        input_dir.to_string(),
        mp3_path_str,
        output_path,
        fps,
        duration,
    )?;
    debug!("Initialized Clipper: {:?}", clipper);

    // Run the clip process.
    clipper.clip()?;
    debug!("Clip process completed successfully");

    Ok(())
}

fn run_exporter(cli: &Cli, config: &Config) -> Result<()> {
    // Ensure an input video path is provided.
    let video_path = cli
        .input
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Input video path is required"))?;
    debug!("Video path: {}", video_path);

    // Extract the optional output path.
    let output_path = cli.output.clone();
    debug!("Output path: {:?}", output_path);

    // Resolve duration either from the provided mp3 or duration argument.
    let mp3_provided = cli.mp3.is_some();
    let duration_provided = cli.duration.is_some();
    let mp3_path = cli.mp3.clone();
    let duration_arg = cli.duration.clone();

    let duration = get_duration(
        mp3_provided,
        duration_provided,
        video_path,
        mp3_path,
        duration_arg,
        config,
    )
    .context("Failed to resolve duration")?;
    debug!("Final duration to use: {} milliseconds", duration);

    // Resolve the frames-per-second value.
    let cli_fps = cli
        .fps
        .clone()
        .map(|s| s.parse::<u32>().context("Invalid FPS value"))
        .transpose()?;
    let fps = get_fps(cli_fps, config).context("Failed to resolve FPS")?;
    debug!("Resolved FPS value: {}", fps);

    // Resolve the pixel upper limit.
    let pixel_upper_limit = cli.pixel_upper_limit.unwrap_or_else(|| {
        get_pixel_upper_limit(None, config).unwrap_or_else(|e| {
            eprintln!("Error resolving pixel upper limit: {}", e);
            std::process::exit(1);
        })
    });
    debug!("Resolved pixel upper limit: {}", pixel_upper_limit);

    // Initialize the exporter directly with the resolved parameters,
    // including the optional output path.
    let exporter = Exporter::new(
        video_path.to_string(),
        output_path,
        duration,
        fps,
        pixel_upper_limit,
    )?;
    exporter.export_images()?;
    debug!("Initialized exporter: {:?}", exporter);

    Ok(())
}

/// Runs the sampler mode using only sampler-specific CLI arguments.
fn run_sampler(cli: &Cli, config: &Config) -> Result<()> {
    // Ensure a video path is provided.
    let video_path = cli
        .input
        .clone()
        .ok_or_else(|| anyhow::anyhow!("Video path must be provided."))?;

    // Determine the output directory using get_audio_dir.
    let output_dir = get_audio_dir(cli.output.clone(), config)
        .context("Failed to resolve audio directory for sampler mode")?;

    // Convert PathBuf to String
    let output_path = Some(output_dir.to_string_lossy().to_string());
    debug!("Video path: {}, Output path: {:?}", video_path, output_path);

    // Resolve duration either from cli.mp3 or cli.duration.
    let mp3_provided = cli.mp3.is_some();
    let duration_provided = cli.duration.is_some();
    let mp3_path = cli.mp3.clone();
    let duration_arg = cli.duration.clone();

    let duration = get_duration(
        mp3_provided,
        duration_provided,
        &video_path,
        mp3_path,
        duration_arg,
        config,
    )
    .context("Failed to resolve duration for sampler mode")?;
    debug!("Final duration to use: {} milliseconds", duration);

    let sampling_number = get_sampling_number(cli.multiple, cli.number, config);
    debug!("Using resolved sampling number: {}", sampling_number);

    // Create sampler arguments.
    let sampler_args = Sampler::new(video_path, output_path, duration, sampling_number);
    debug!("Sampler CLI Arguments: {:?}", sampler_args);

    // Set up a Ctrl+C handler.
    let running = Arc::new(AtomicBool::new(true));
    {
        let running_clone = running.clone();
        ctrlc::set_handler(move || {
            eprintln!("\nReceived Ctrl+C, terminating...");
            running_clone.store(false, Ordering::SeqCst);
        })
        .context("Error setting Ctrl+C handler")?;
    }

    // Execute the sampling process.
    sampler_args?
        .sample_images(running)
        .context("An error occurred during sample image processing")?;

    Ok(())
}

fn run_gmicer(cli: &Cli) -> Result<()> {
    debug!("Running in GMIC mode");

    // Ensure at least one GMIC argument is provided.
    let gmic_args = match &cli.gmic_args {
        Some(args) if !args.is_empty() => args.clone(),
        _ => {
            return Err(anyhow::anyhow!(
                "GMIC mode requires at least one GMIC argument."
            ))
        }
    };

    // Validate that the input is provided and is a directory.
    let input = cli
        .input
        .clone()
        .ok_or_else(|| anyhow::anyhow!("For GMIC mode, the input directory must be provided."))?;
    let input_path = Path::new(&input);
    if !input_path.is_dir() {
        return Err(anyhow::anyhow!(
            "For GMIC mode, the input must be a directory: {}",
            input
        ));
    }
    debug!("GMIC input directory: {:?}", input_path);

    // Filter out verbosity flags from the GMIC arguments.
    let mut filtered_args = gmic_args.clone();
    debug!("GMIC arguments before filtering: {:?}", filtered_args);
    filtered_args.retain(|arg| !matches!(arg.as_str(), "-v" | "-vv" | "-vvv" | "-vvvv"));
    debug!("GMIC arguments after filtering: {:?}", filtered_args);

    // Create the GMIC processor instance.
    let gmicer = Gmicer::new(&input, cli.output.clone().as_deref(), filtered_args)
        .context("Failed to initialize GMIC processor")?;
    gmicer
        .gmic_images()
        .context("Failed to process images using GMIC")?;

    Ok(())
}

/// Runs the merger mode by resolving merger parameters and merging the images.
fn run_merger(cli: &Cli, config: &Config) -> Result<()> {
    let opacity = get_opacity(cli.opacity, config).context("Failed to resolve opacity")?;
    debug!("Resolved opacity: {}", opacity);

    // Unwrap input since it's now an Option<String>
    let input = cli.input.clone().expect("Input is required in merger mode");

    let directory2 = cli
        .directory2
        .clone()
        .expect("Directory 2 is required in merger mode");

    let merger = Merger::new(input, directory2, opacity, cli.output.clone());

    merger?.merge_images().context("Failed to merge images")?;
    Ok(())
}

fn run_clutter(cli: &Cli, config: &Config) -> Result<()> {
    // Ensure that the input directory is provided.
    let input_dir = cli
        .input
        .as_ref()
        .context("Input directory is required in clutter mode")?;
    debug!("Input directory: {:?}", input_dir);

    // Ensure the CLUT image is provided.
    let clut_image = cli
        .clut_image
        .as_ref()
        .context("CLUT image is required in clutter mode")?;
    debug!("CLUT image: {:?}", clut_image);

    // Create a Clutter instance using the input directory, CLUT image,
    // and optionally the output directory.
    let clutter = Clutter::new(input_dir.clone(), clut_image.clone(), cli.output.clone());
    debug!(
        "Clutter instance created with input_dir: {:?} and clut_image: {:?}",
        input_dir, clut_image
    );

    // Generate CLUT images.
    let clut_dir = clutter?
        .create_clut_images()
        .context("Failed to create CLUT images")?;
    debug!(
        "CLUT images created successfully in directory: {:?}",
        clut_dir
    );

    // Determine if merging is enabled via the clutter-specific flags.
    let merger_enabled = cli.clut_merge || cli.clut_multiple || cli.clut_opacity.is_some();
    debug!("Clutter merger mode enabled: {}", merger_enabled);

    if merger_enabled {
        debug!("Clutter merger mode activated.");

        if cli.clut_opacity.is_some() && cli.clut_multiple {
            warn!("Both --clut-opacity and --clut-multiple are selected. The single opacity value will take priority.");
        }

        let opacities = if cli.clut_opacity.is_some() {
            debug!("Single opacity mode selected, ignoring --clut-multiple.");
            vec![get_opacity(cli.clut_opacity, config)
                .context("Failed to retrieve opacity from configuration")?]
        } else if cli.clut_multiple {
            debug!("Retrieving multiple opacities from configuration");
            get_multiple_opacities(None, config)
                .context("Failed to retrieve multiple opacities from configuration")?
                .to_vec()
        } else {
            debug!("No opacity options selected, skipping merging.");
            return Ok(());
        };

        // Call the merging function.
        // (Note: Depending on your intended logic, you might swap the order of the directories.)
        merging(
            clut_dir.clone(),  // Represents the CLUT images directory.
            input_dir.clone(), // Represents the original images directory.
            Some(clut_dir.clone()),
            opacities,
        )?;
    } else {
        debug!("Clutter merger mode not activated. Skipping image merging.");
    }

    debug!("Clutter run completed successfully");
    Ok(())
}

/// Merges images with varying opacities from input directories.
///
/// This function handles the merging process for multiple opacity levels,
/// creating composite images based on the specified input and CLUT directories.
///
/// # Parameters
/// - `input_dir`: Path to the directory containing input images.
/// - `clut_dir`: Path to the directory containing color lookup tables.
/// - `output_dir`: Optional path for output images; defaults to input directory if not specified.
/// - `opacities`: List of opacity values to apply during the merging process.
///
/// # Returns
/// - `Result<()>`: Success if all images are merged without errors; `Err` otherwise.
///
/// # Notes
/// - Each opacity value in `opacities` triggers a separate merging operation.
/// - Output directory defaults to the input directory if not provided.
fn merging(
    input_dir: String,
    clut_dir: String,
    _output_dir: Option<String>,
    opacities: Vec<f32>,
) -> Result<Vec<String>> {
    let mut output_directories = Vec::new();

    for opacity in opacities {
        debug!("Merging images with opacity: {}", opacity);
        let merger = Merger::new(input_dir.clone(), clut_dir.clone(), opacity, None)
            .context("Failed to initialize image merger")?;

        let output_directory = merger.merge_images()?; // Apply the `?` operator here
        debug!("Images merged successfully with opacity: {}", opacity);

        output_directories.push(output_directory.to_string_lossy().into_owned());
    }

    Ok(output_directories)
}
