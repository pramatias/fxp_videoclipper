use anyhow::{Context, Result};
use clap::{ArgAction, Args, Parser, Subcommand};
use clap_verbosity_flag::log::LevelFilter;
use console::style;
use log::{debug, warn};
use std::path::Path;

use init::get_audio_file;
use init::{get_audio_dir, get_audio_duration, get_multiple_opacities};
use init::{get_duration, get_fps, get_opacity, get_pixel_upper_limit, get_sampling_number};
use init::{initialize_configuration, initialize_logger, load_default_configuration, Config};

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

#[derive(clap::Args, Debug)]
pub struct Verbosity {
    #[arg(short = 'v', long, action = clap::ArgAction::Count, display_order = 99)]
    pub verbose: u8,

    #[arg(short = 'q', long, action = clap::ArgAction::Count, display_order = 100)]
    pub quiet: u8,
}
impl Verbosity {
    pub fn log_level_filter(&self) -> LevelFilter {
        if self.quiet > 0 {
            LevelFilter::Warn
        } else {
            match self.verbose {
                0 => LevelFilter::Info,
                1 => LevelFilter::Debug,
                _ => LevelFilter::Trace,
            }
        }
    }
}

#[derive(Args, Debug)]
struct ClipperCommonOptions {
    /// Optional path to the MP3 file (Exporter, Sampler)
    #[arg(short = 'a', long = "audio", help = "Optional path to the MP3 file ")]
    mp3: Option<String>,
    /// Frames per second to extract (Exporter)
    #[arg(short, long, help = "Frames per second to extract \n")]
    fps: Option<String>,
}

#[derive(Args, Debug)]
struct CommonOptions {
    /// Optional path to the MP3 file (Exporter, Sampler)
    #[arg(short = 'a', long = "audio", help = "Optional path to the MP3 file ")]
    mp3: Option<String>,
    /// Duration in milliseconds to cut the video (Exporter, Sampler)
    #[arg(short, long, help = "Duration in milliseconds to cut the video ")]
    duration: Option<String>,
    /// Frames per second to extract (Exporter)
    #[arg(short, long, help = "Frames per second to extract \n")]
    fps: Option<String>,
}

#[derive(Args, Debug)]
struct SamplerCommonOptions {
    /// Optional path to the MP3 file (Exporter, Sampler)
    #[arg(short = 'a', long = "audio", help = "Optional path to the MP3 file ")]
    mp3: Option<String>,
    /// Duration in milliseconds to cut the video (Exporter, Sampler)
    #[arg(short, long, help = "Duration in milliseconds to cut the video \n")]
    duration: Option<String>,
}

#[derive(Args, Debug)]
struct ClipperOptions {
    #[command(flatten)]
    io: ClipperInputOutput,
    #[command(flatten)]
    common_options: ClipperCommonOptions,
}

#[derive(Args, Debug)]
struct GmicerOptions {
    #[command(flatten)]
    io: InputOutput,

    /// Arguments for GMIC command
    #[arg(
        help = "Arguments for GMIC command ",
        num_args = 0..,
        allow_hyphen_values = true
    )]
    gmic_args: Option<Vec<String>>,
}

#[derive(Args, Debug)]
pub struct ClutterOptions {
    #[command(flatten)]
    io: InputOutput,
    /// Path to the source image used for CLUT (Clutter mode)
    #[arg(
        short = 'l',
        long = "clut",
        help = "Path to the source image used for CLUT"
    )]
    pub clut_image: String,
    /// Opacity level for merging in clutter mode (0.0 - 1.0)
    #[arg(long = "clut-opacity", help = "Opacity level for merging in clutter mode ", value_parser = clap::value_parser!(f32))]
    pub clut_opacity: Option<f32>,
    /// Merge clutted images with original with multiple opacities
    #[arg(long = "clut-multiple", help = "Merge clutted images with original ", action = ArgAction::SetTrue)]
    pub clut_multiple: bool,
    /// Run the merging process after applying CLUT
    pub clut_merge: bool,
}

#[derive(Args, Debug)]
struct SamplerOptions {
    #[command(flatten)]
    io: ExporterInputOutput,

    /// Extract multiple frames (Sampler)
    #[arg(short = 'u', long = "multiple", help = "Extract multiple frames", action = ArgAction::SetTrue)]
    multiple: bool,

    /// Number of frames to extract (Sampler)
    #[arg(short = 'n', long = "number", help = "Number of frames to extract", value_parser = clap::value_parser!(usize))]
    number: Option<usize>,

    #[command(flatten)]
    common_options: SamplerCommonOptions,
}

#[derive(Args, Debug)]
struct MergerOptions {
    #[command(flatten)]
    io: InputOutput,
    /// Path to the second image directory (Merger)
    #[arg(
        short = 'r',
        long = "second-directory",
        help = "Path to the second image directory (Merger)"
    )]
    directory2: String,
    /// Opacity level for merging (Merger)
    #[arg(
        short = 't',
        long,
        help = "Opacity level for merging \n",
        default_value = "0.5"
    )]
    opacity: f32,
}

#[derive(Args, Debug)]
struct ExporterOptions {
    #[command(flatten)]
    io: ExporterInputOutput,

    /// Maximum upper limit for pixel resolution (Exporter only)
    #[arg(short, long = "pixel-limit", help = "Maximum upper limit for pixel resolution", value_parser = clap::value_parser!(u32))]
    pixel_upper_limit: Option<u32>,

    #[command(flatten)]
    common: CommonOptions,
}

#[derive(Args, Debug)]
struct ClipperInputOutput {
    /// Input for video or directory. Applies to all modes.
    #[arg(short = 'i', long, help = "Input directory ")]
    input: String,
    /// Output for directory or video. Applies to all modes.
    #[arg(short = 'o', long, help = "Output video \n")]
    output: Option<String>,
}

#[derive(Args, Debug)]
struct InputOutput {
    /// Input for video or directory. Applies to all modes.
    #[arg(short = 'i', long, help = "Input directory ")]
    input: String,
    /// Output for directory or video. Applies to all modes.
    #[arg(short = 'o', long, help = "Output directory \n")]
    output: Option<String>,
}

#[derive(Args, Debug)]
struct ExporterInputOutput {
    /// Input for video or directory. Applies to all modes.
    #[arg(short = 'i', long, help = "Input video")]
    input: String,
    /// Output for directory or video. Applies to all modes.
    #[arg(short = 'o', long, help = "Output directory \n")]
    output: Option<String>,
}

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
    #[command(subcommand)]
    mode: Mode,
}

#[derive(Subcommand, Debug)]
enum Mode {
    /// Initialize configuration
    Init,
    /// Export frames based on duration and resolution
    Exporter(ExporterOptions),
    /// Sample frames evenly across the video
    Sampler(SamplerOptions),
    /// Merger: Merge two directories of images. Uses the IO input as the first directory.
    Merger(MergerOptions),
    /// Apply a GMIC command to all images in the input directory
    Gmicer(GmicerOptions),
    /// Transfer colors using a CLUT file
    Clutter(ClutterOptions),
    /// Create the videoclip
    Clipper(ClipperOptions),
}

/// Main entry point for the application, handling command-line argument parsing and dispatching.
///
/// This function initializes the application, sets up logging, loads configuration, and dispatches execution
/// based on the specified command-line mode.
///
/// # Parameters
/// - None
///
/// # Returns
/// - `Result<()>`: Indicates success or failure of the application execution
///
/// # Notes
/// - This is the primary entry point of the application
/// - Dispatches to different runtime modes based on the command-line arguments provided
/// - Upon successful execution, returns `Ok(())`
fn main() -> Result<()> {
    let cli = Cli::parse();

    let verbosity_level = cli.verbose.log_level_filter();
    initialize_logger(verbosity_level).context("Failed to initialize logger")?;
    debug!(
        "{} {:?}",
        style("Logger initialized with verbosity:").cyan(),
        verbosity_level
    );

    let config = load_default_configuration().context("Failed to load default configuration")?;
    debug!("{}", style("Default configuration loaded").green());

    // Dispatch based on the subcommand variant
    match &cli.mode {
        Mode::Init => {
            debug!("{}", style("Initializing configuration...").yellow());
            initialize_configuration().context("Failed to initialize configuration")?;
            return Ok(());
        }

        Mode::Gmicer(options) => {
            debug!("{}", style("Running in GMIC mode").blue());
            run_gmicer(options, &config)?;
        }
        Mode::Clipper(options) => {
            debug!("{}", style("Running in clipper mode").blue());
            run_clipper(options, &config)?;
        }
        Mode::Clutter(options) => {
            debug!("{}", style("Running in clutter mode").blue());
            run_clutter(options, &config)?;
        }
        Mode::Sampler(options) => {
            debug!("{}", style("Running in sampler mode").blue());
            run_sampler(options, &config)?;
        }
        Mode::Exporter(options) => {
            debug!("{}", style("Running in exporter mode").blue());
            run_exporter(options, &config)?;
        }
        Mode::Merger(options) => {
            debug!("{}", style("Running in merger mode").blue());
            run_merger(options, &config)?;
        }
    }

    debug!(
        "{}",
        style("Main function execution completed successfully").green()
    );
    Ok(())
}

/// Processes images using the GMIC tool with specified options and configuration.
///
/// This function runs in GMIC mode, handling input validation, argument filtering,
/// and output directory management.
///
/// # Parameters
/// - `options`: Contains input, output, and GMIC arguments.
/// - `config`: Configuration settings for the application.
///
/// # Returns
/// - `Result<()>`: Indicates success or failure of image processing.
///
/// # Notes
/// - The input must be a directory.
/// - At least one GMIC argument is required.
/// - Handles the `-o` flag for explicit output directories.
fn run_gmicer(options: &GmicerOptions, _config: &Config) -> Result<()> {
    debug!("Running in GMIC mode");

    // Validate that the input is provided and is a directory.
    let input = &options.io.input;
    let input_path = Path::new(input);
    if !input_path.is_dir() {
        return Err(anyhow::anyhow!(
            "For GMIC mode, the input must be a directory: {}",
            input
        ));
    }
    debug!("GMIC input directory: {:?}", input_path);

    // Ensure that at least one GMIC argument is provided.
    let args = options.gmic_args.clone().unwrap_or_default();
    if args.is_empty() {
        return Err(anyhow::anyhow!(
            "GMIC mode requires at least one GMIC argument."
        ));
    }
    debug!("GMIC arguments before filtering: {:?}", args);

    // Filter out any "-o" flag and capture its associated argument.
    let (filtered_args, explicit_output) = filter_output_flag(args);
    debug!("GMIC arguments after filtering: {:?}", filtered_args);
    debug!("Captured output flag value: {:?}", explicit_output);

    // Determine the output regardless of it's position to the command
    // Contained amongst args is different than be put somewhere else
    let output = explicit_output.or_else(|| options.io.output.clone());
    debug!("Final GMIC output directory: {:?}", output);

    // Create the GMIC processor instance using the input, output, and filtered GMIC args.
    let gmicer = gmicer::Gmicer::new(input, output.as_deref(), filtered_args)
        .context("Failed to initialize GMIC processor")?;
    gmicer
        .gmic_images()
        .context("Failed to process images using GMIC")?;

    Ok(())
}

/// Merges images from two directories based on the provided options and configuration.
///
/// This function takes two directories of images, applies the specified opacity,
/// and merges them into the output directory.
///
/// # Parameters
/// - `options`: A struct containing input/output paths and opacity value.
/// - `config`: Configuration containing default settings.
///
/// # Returns
/// - `Result<()>`: Indicates success or failure of the merge operation.
///
/// # Notes
/// - Extracts directories from the provided options and uses them for merging.
/// - Returns an error if opacity resolution or image merging fails.
fn run_merger(options: &MergerOptions, config: &Config) -> Result<()> {
    // Resolve the opacity using the value provided in the merger options.
    let opacity =
        get_opacity(Some(options.opacity), config).context("Failed to resolve opacity")?;
    debug!("Resolved opacity: {}", opacity);

    // Use the embedded InputOutput field for directories.
    let directory1 = options.io.input.clone();
    let directory2 = options.directory2.clone();
    let output = options.io.output.clone();

    // Initialize the merger with the provided directories, opacity, and output.
    let merger = merger::Merger::new(directory1, directory2, opacity, output);
    merger?.merge_images().context("Failed to merge images")?;
    Ok(())
}

/// Processes video clips with synchronized audio using specified options and configuration.
///
/// This function handles the entire workflow of clipping video based on the provided parameters.
/// It resolves input/output paths, audio files, FPS, and duration, then executes the clipping process.
///
/// # Parameters
/// - `options`: Struct containing clipper-specific options, including input/output paths and FPS.
/// - `config`: Configuration struct providing default values and settings.
///
/// # Returns
/// - `Result<()>`: Indicates success or failure of the clipping process.
fn run_clipper(options: &ClipperOptions, config: &Config) -> Result<()> {
    // Get input and output from the embedded I/O field.
    let input_dir = &options.io.input;
    debug!("Input directory: {}", input_dir);

    let output_path = options.io.output.clone();
    debug!("Output path: {:?}", output_path);

    // Use the common options from the ClipperOptions.
    let mp3_path = get_audio_file(options.common_options.mp3.clone(), config)
        .context("Failed to get audio file")?;
    debug!("Resolved MP3 path: {:?}", mp3_path);

    let mp3_path_str = mp3_path.as_ref().map(|p| p.to_string_lossy().into_owned());

    // Resolve the FPS value using the common options.
    let cli_fps = options
        .common_options
        .fps
        .clone()
        .map(|s| s.parse::<u32>().context("Invalid FPS value"))
        .transpose()?;
    let fps_val = get_fps(cli_fps, config).context("Failed to resolve FPS")?;
    debug!("Resolved FPS value: {}", fps_val);

    // Get the audio duration using the mp3_path.
    let duration =
        get_audio_duration(mp3_path_str.clone(), config).context("Failed to resolve duration")?;
    match duration {
        Some(d) => debug!("Final duration to use: {} milliseconds", d),
        None => debug!("Final duration to use: None"),
    }

    // Initialize the Clipper with the resolved parameters.
    let clipper = clipper::Clipper::new(
        input_dir.clone(),
        mp3_path_str,
        output_path,
        fps_val,
        duration,
    )?;
    debug!("Initialized Clipper: {:?}", clipper);

    // Run the clip process.
    clipper.clip()?;
    debug!("Clip process completed successfully");

    Ok(())
}

/// Executes the CLUT process, generating and merging images based on specified options.
///
/// This function initializes the CLUT process, creates CLUT images, and optionally merges
/// them with opacity adjustments if enabled.
///
/// # Parameters
/// - `options`: Configuration options for the CLUT process.
/// - `config`: Application configuration containing additional settings.
///
/// # Returns
/// - `Result<()>`: Indicates success or failure of the CLUT operation.
///
/// # Notes
/// - Merging of CLUT images is performed only if the corresponding flags are enabled.
fn run_clutter(options: &ClutterOptions, config: &Config) -> Result<()> {
    // Access input and output from the flattened InputOutput field
    let input_dir = &options.io.input;
    let output = options.io.output.clone();

    debug!("Input directory: {:?}", input_dir);

    // Ensure the CLUT image is provided.
    let clut_image = &options.clut_image;
    debug!("CLUT image: {:?}", clut_image);

    // Create a Clutter instance using the input directory, CLUT image, and output.
    let clutter = clutter::Clutter::new(input_dir.clone(), clut_image.clone(), output);
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
    let merger_enabled =
        options.clut_merge || options.clut_multiple || options.clut_opacity.is_some();
    debug!("Clutter merger mode enabled: {}", merger_enabled);

    if merger_enabled {
        debug!("Clutter merger mode activated.");

        if options.clut_opacity.is_some() && options.clut_multiple {
            warn!("Both --clut-opacity and --clut-multiple are selected. The single opacity value will take priority.");
        }

        let opacities = if options.clut_opacity.is_some() {
            debug!("Single opacity mode selected, ignoring --clut-multiple.");
            vec![get_opacity(options.clut_opacity, config)
                .context("Failed to retrieve opacity from configuration")?]
        } else if options.clut_multiple {
            debug!("Retrieving multiple opacities from configuration");
            get_multiple_opacities(None, config)
                .context("Failed to retrieve multiple opacities from configuration")?
                .to_vec()
        } else {
            debug!("No opacity options selected, skipping merging.");
            return Ok(());
        };

        // Call the merging function.
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

/// Processes video and audio to generate samples according to specified parameters.
///
/// This function manages the sampling process, including input validation, duration calculation,
/// and signal handling for interruptible operation.
///
/// # Parameters
/// - `options`: Contains input/output paths, duration, and sampling configuration.
/// - `config`: Application-level settings that may override or extend options.
///
/// # Returns
/// - `Result<()>`: Indicates success or failure of the sampling process.
///
/// # Notes
/// - Requires a valid video input path to proceed with sampling.
/// - Supports interruptible operation through Ctrl+C handler.
/// - Calculates appropriate duration and sampling number based on inputs.
fn run_sampler(options: &SamplerOptions, config: &Config) -> Result<()> {
    // Ensure an input path is provided.
    let video_path = options.io.input.clone();
    if video_path.is_empty() {
        return Err(anyhow::anyhow!("Video path must be provided."));
    }

    let output_dir = get_audio_dir(options.io.output.clone(), config)
        .context("Failed to resolve audio directory for sampler mode")?;
    let output_path = Some(output_dir.to_string_lossy().to_string());
    debug!("Video path: {}, Output path: {:?}", video_path, output_path);

    // Resolve duration using the exporter options embedded in common_options.
    let mp3_path = options.common_options.mp3.clone();
    let duration_arg = options.common_options.duration.clone();

    let duration = get_duration(&video_path, mp3_path, duration_arg, config)
        .context("Failed to resolve duration for sampler mode")?;
    debug!("Final duration to use: {} milliseconds", duration);

    // Get the sampling number based on the 'multiple' flag and 'number' option.
    let sampling_number = get_sampling_number(options.multiple, options.number, config);
    debug!("Using resolved sampling number: {}", sampling_number);

    // Create sampler arguments.
    let sampler_args = sampler::Sampler::new(video_path, output_path, duration, sampling_number);
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

/// Executes the image exporting process based on the provided options and configuration.
///
/// This function handles the core logic of exporting images from a video source,
/// including resolving video paths, output settings, and processing parameters.
///
/// # Parameters
/// - `options`: An `ExporterOptions` instance containing exporter-specific settings.
/// - `config`: A `Config` instance providing global configuration settings.
///
/// # Returns
/// - `Result<()>`: Indicates success or failure of the export operation.
///
/// # Notes
/// - Manages input/output paths, video duration, FPS calculation, and pixel limits.
/// - Creates and executes the exporter instance with calculated parameters.
fn run_exporter(options: &ExporterOptions, config: &Config) -> Result<()> {
    // Use the new IO field for input/output
    let video_path = &options.io.input;
    let output_path = &options.io.output;
    debug!("Video path: {}", video_path);
    debug!("Output path: {:?}", output_path);

    let mp3_path = options.common.mp3.clone();
    let duration_arg = options.common.duration.clone();

    let duration = get_duration(video_path, mp3_path, duration_arg, config)
        .context("Failed to resolve duration")?;
    debug!("Final duration to use: {} milliseconds", duration);

    let cli_fps = options
        .common
        .fps
        .clone()
        .map(|s| s.parse::<u32>().context("Invalid FPS value"))
        .transpose()?;
    let fps = get_fps(cli_fps, config).context("Failed to resolve FPS")?;
    debug!("Resolved FPS value: {}", fps);

    let pixel_upper_limit = options.pixel_upper_limit.unwrap_or_else(|| {
        get_pixel_upper_limit(None, config).unwrap_or_else(|e| {
            eprintln!("Error resolving pixel upper limit: {}", e);
            std::process::exit(1);
        })
    });
    debug!("Resolved pixel upper limit: {}", pixel_upper_limit);

    let exporter = exporter::Exporter::new(
        video_path.to_string(),
        output_path.clone(),
        duration,
        fps,
        pixel_upper_limit,
    )?;
    exporter.export_images()?;
    debug!("Finished running exporter: {:?}", exporter);

    Ok(())
}

// Updated run_clutter that accepts separate input/output and clutter options.
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
        let merger = merger::Merger::new(input_dir.clone(), clut_dir.clone(), opacity, None)
            .context("Failed to initialize image merger")?;

        let output_directory = merger.merge_images()?; // Apply the `?` operator here
        debug!("Images merged successfully with opacity: {}", opacity);

        output_directories.push(output_directory.to_string_lossy().into_owned());
    }

    Ok(output_directories)
}

/// Helper function that filters out any occurrence of "-o" and its following argument,
/// returning a tuple of (filtered arguments, Option<output_flag_value>).
fn filter_output_flag(args: Vec<String>) -> (Vec<String>, Option<String>) {
    let mut filtered_args = Vec::new();
    let mut output_flag_value = None;
    let mut skip_next = false;
    for arg in args.into_iter() {
        if skip_next {
            // Capture the argument following "-o" as the output flag value.
            output_flag_value = Some(arg);
            skip_next = false;
            continue;
        }
        if arg == "-o" {
            // Skip this argument and mark the next one for capturing.
            skip_next = true;
            continue;
        }
        filtered_args.push(arg);
    }
    (filtered_args, output_flag_value)
}
