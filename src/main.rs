use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::fs;
use std::sync::Mutex;
use std::io::Read;
use rayon::prelude::*;
use walkdir::WalkDir;
use image::{io::Reader as ImageReader, ImageFormat, DynamicImage, Rgba, RgbaImage};
use clap::{Arg, ArgAction, Command};

fn collect_image_files(source_dir: &Path) -> Vec<PathBuf> {
    let mut image_files = Vec::new();
    let allowed_extensions = ["jpg", "jpeg", "png", "gif", "bmp", "webp", "tiff"];

    for entry in WalkDir::new(source_dir)
        .into_iter()
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if allowed_extensions.contains(&ext.to_lowercase().as_str()) {
                    image_files.push(path.to_path_buf());
                }
            }
        }
    }

    image_files
}

/// Converts an image from its current format to a target format (e.g., PNG, JPEG, BMP).
/// This function will skip unsupported formats and files that cannot be decoded.
fn convert_image(input_path: &Path, output_dir: &Path, target_format: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Skip unsupported formats, such as SVG (image::guess_format will return an error for it)
    if let Some(ext) = input_path.extension() {
        let ext = ext.to_str().unwrap_or("").to_lowercase();
        if ext == "svg" {
            println!("Skipping SVG file: {:?}", input_path);
            return Ok(());  // Skip SVG files, as they're not supported
        }
    }

    // Guess the format of the image based on its contents
    let mut file = std::fs::File::open(input_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    let format = image::guess_format(&buffer)?;

    // If the format is unsupported, skip the file
    if !matches!(format, ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::Bmp) {
        println!("Skipping unsupported file format: {:?}", input_path);
        return Ok(());  // Skip unsupported file formats
    }

    // Try opening and decoding the image file
    let img_result = ImageReader::open(input_path);

    // If the image cannot be decoded, skip the file
    let img = match img_result {
        Ok(reader) => reader.decode(),
        Err(_) => {
            println!("Skipping file (could not decode): {:?}", input_path);
            return Ok(());
        }
    };

    let img = img?;

    // Create the output path by changing the file extension to the target format
    let mut output_path = output_dir.to_path_buf();
    output_path.push(input_path.file_stem().unwrap());
    output_path.set_extension(target_format);

    // Check if the output file already exists
    if output_path.exists() {
        println!("Output already exists for {:?}; skipping", input_path);
        return Ok(());  // Skip if the file already exists
    }

    // Determine the format to save the image
    let format = match target_format {
        "png" => ImageFormat::Png,
        "jpg" | "jpeg" => ImageFormat::Jpeg,
        "bmp" => ImageFormat::Bmp,
        "webp" => ImageFormat::WebP,
        _ => return Err(format!("Unsupported format: {}", target_format).into()), // Error for unsupported target formats
    };

    // Save the image in the specified format
    img.save_with_format(output_path.clone(), format)?;
    println!("Converted: {:?} -> {:?}", input_path, output_path);
    Ok(())
}

/// Traverses the source directory, processes all image files, and converts them to the specified format.
fn process_images(source_dir: &Path, output_dir: &Path, target_format: &str) -> Result<(), Box<dyn std::error::Error>> {

    let entries: Vec<_> = fs::read_dir(source_dir)?.collect();
    if entries.is_empty() {
        println!("No images found in the source directory.");
        return Ok(());
    }

    // Mutex is used to safely share the file list among threads
    let files_to_process: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());

    // Traverse the source directory and collect image files
    WalkDir::new(source_dir)
        .into_iter()
        .filter_map(Result::ok) // Ignore errors from unreadable directories
        .for_each(|entry| {
            let path = entry.path();

            if path.is_file() {
                if let Some(ext) = path.extension() {
                    let ext = ext.to_str().unwrap_or("").to_lowercase();

                    // Skip unsupported file formats like SVG
                    if ext == "svg" {
                        println!("Skipping SVG file: {:?}", path);
                    } else if ext != target_format {
                        let mut files = files_to_process.lock().unwrap();
                        files.push(path.to_path_buf()); // Add the file to the list of files to process
                        println!("Found supported image file: {:?}", path);  // Log supported image files
                    }
                }
            }
        });

    // Retrieve the list of files to process
    let files = files_to_process.into_inner().unwrap();

    // If no files were found to process, print a message and exit
    if files.is_empty() {
        println!("No files found to convert!");
    }

    // Process the image files in parallel using rayon
    files.par_iter().for_each(|file| {
        if let Err(e) = convert_image(file, output_dir, target_format) {
            eprintln!("Failed to process {:?}: {}", file, e);  // Log any errors
        }
    });

    Ok(())
}

/// Checks if two pixels are significantly different (i.e., an edge)
fn is_edge(p1: Rgba<u8>, p2: Rgba<u8>, edge_threshold: u8) -> bool {
    let diff_r = p1[0].abs_diff(p2[0]);
    let diff_g = p1[1].abs_diff(p2[1]);
    let diff_b = p1[2].abs_diff(p2[2]);

    diff_r > edge_threshold || diff_g > edge_threshold || diff_b > edge_threshold
}

/// Removes only the outer near-white background, stopping at edges.
fn remove_background(img: &DynamicImage, edge_threshold: u8) -> RgbaImage {
    let img = img.to_rgba8();
    let (width, height) = img.dimensions();
    let mut output = img.clone();
    let mut visited = vec![vec![false; width as usize]; height as usize];
    let mut queue = VecDeque::new();

    // Initialize BFS with border pixels
    for x in 0..width {
        queue.push_back((x, 0));
        queue.push_back((x, height - 1));
    }
    for y in 1..height - 1 {
        queue.push_back((0, y));
        queue.push_back((width - 1, y));
    }

    while let Some((x, y)) = queue.pop_front() {
        if x >= width || y >= height || visited[y as usize][x as usize] {
            continue;
        }
        visited[y as usize][x as usize] = true;

        let pixel = img.get_pixel(x, y);
        let [r, g, b, _] = pixel.0;

        // If pixel is near white and not an edge, continue flood-fill
        if r > 240 && g > 240 && b > 240 {
            let mut is_surrounded_by_edges = false;

            // Check neighboring pixels for strong edges
            if x > 0 && is_edge(*pixel, img.get_pixel(x - 1, y).clone(), edge_threshold) {
                is_surrounded_by_edges = true;
            }
            if x + 1 < width && is_edge(*pixel, img.get_pixel(x + 1, y).clone(), edge_threshold) {
                is_surrounded_by_edges = true;
            }
            if y > 0 && is_edge(*pixel, img.get_pixel(x, y - 1).clone(), edge_threshold) {
                is_surrounded_by_edges = true;
            }
            if y + 1 < height && is_edge(*pixel, img.get_pixel(x, y + 1).clone(), edge_threshold) {
                is_surrounded_by_edges = true;
            }

            // If an edge is nearby, stop removing
            if is_surrounded_by_edges {
                continue;
            }

            // Make background transparent
            output.put_pixel(x, y, Rgba([0, 0, 0, 0]));

            // Add neighboring pixels
            if x > 0 {
                queue.push_back((x - 1, y));
            }
            if x + 1 < width {
                queue.push_back((x + 1, y));
            }
            if y > 0 {
                queue.push_back((x, y - 1));
            }
            if y + 1 < height {
                queue.push_back((x, y + 1));
            }
        }
    }

    output
}

fn remove_bg_from_images(source_dir: &Path, output_dir: &Path, edge_threshold: u8) -> Result<(), Box<dyn std::error::Error>> {
    if !source_dir.exists() || !source_dir.is_dir() {
        return Err("Source directory does not exist or is not a directory".into());
    }

    let files = collect_image_files(source_dir);
    if files.is_empty() {
        println!("No images found in the source directory.");
        return Ok(());
    }

    files.par_iter().for_each(|input_path| {
        let img_result = ImageReader::open(input_path)
            .and_then(|reader| reader.decode().map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e)))
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e));

        let img = match img_result {
            Ok(img) => img,
            Err(_) => {
                println!("Skipping file (could not decode): {:?}", input_path);
                return;
            }
        };

        let processed_img = remove_background(&img, edge_threshold);

        // Get relative path from the source directory
        let relative_path = input_path.strip_prefix(source_dir).unwrap();

        // Construct the full output path (including subdirectories)
        let mut output_path = output_dir.join(relative_path);
        output_path.set_extension("png"); // Ensure output format is PNG

        // Create parent directories if they don't exist
        if let Some(parent) = output_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).expect("Failed to create output subdirectory");
            }
        }

        // Save the processed image
        if let Err(e) = processed_img.save(&output_path) {
            eprintln!("Failed to save {:?}: {}", output_path, e);
        } else {
            println!("Processed: {:?} -> {:?}", input_path, output_path);
        }
    });

    Ok(())
}

fn main() {
    // CLI argument parsing with clap
    let matches = Command::new("RICO - Rust Image Converter")
        .version("1.0")
        .author("Rana Jahanzaib <work@withrana.com>")
        .about("RICO is a Rust-powered CLI tool for rapid, parallel image conversion.")
        .subcommand(
            Command::new("remove")
                .about("Remove background from images")
                .arg(
                    Arg::new("background")
                        .short('b')
                        .long("background")
                        .action(ArgAction::SetTrue) // Flag without a value
                        .help("Remove background from images"),
                )
                .arg(
                    Arg::new("source")
                        .short('s')
                        .long("source")
                        .value_parser(clap::value_parser!(String))
                        .required(true)
                        .help("Source directory for input images"),
                )
                .arg(
                    Arg::new("output")
                        .short('o')
                        .long("output")
                        .value_parser(clap::value_parser!(String))
                        .help("Output directory for processed images (optional, defaults to source directory)"),
                )
                .arg(
                    Arg::new("edge-threshold")
                        .short('e')
                        .long("edge-threshold")
                        .value_parser(clap::value_parser!(u8))
                        .default_value("30")
                        .help("Set the edge detection threshold (default: 30)"),
                ),
        )
        .subcommand(
            Command::new("convert")
                .about("Convert images to different formats")
                .arg(
                    Arg::new("source")
                        .short('s')
                        .long("source")
                        .value_parser(clap::value_parser!(String))
                        .required(true)
                        .help("Source directory for input images"),
                )
                .arg(
                    Arg::new("output")
                        .short('o')
                        .long("output")
                        .value_parser(clap::value_parser!(String))
                        .help("Output directory for converted images (optional, defaults to source directory)"),
                )
                .arg(
                    Arg::new("format")
                        .short('f')
                        .long("format")
                        .value_parser(clap::value_parser!(String))
                        .default_value("png")
                        .help("Target format for conversion (e.g., png, jpg, bmp, webp)"),
                ),
        )
        .get_matches();

    // Handle "remove" command
    if let Some(remove_matches) = matches.subcommand_matches("remove") {
        let remove_bg = remove_matches.get_flag("background");
        let source_dir = Path::new(remove_matches.get_one::<String>("source").unwrap());
        let output_dir = remove_matches
            .get_one::<String>("output")
            .map(Path::new)
            .unwrap_or(source_dir);
        let edge_threshold: u8 = *remove_matches.get_one::<u8>("edge-threshold").unwrap_or(&30);

        if !source_dir.exists() || !source_dir.is_dir() {
            eprintln!("Source directory does not exist or is not a directory");
            std::process::exit(1);
        }

        if !output_dir.exists() {
            fs::create_dir_all(output_dir).expect("Failed to create output directory");
        }

        if remove_bg {
            if let Err(e) = remove_bg_from_images(source_dir, output_dir, edge_threshold) {
                eprintln!("Error removing background: {}", e);
            } else {
                println!("Background removal completed.");
            }
        }

        return;
    }

    // Handle "convert" command
    if let Some(convert_matches) = matches.subcommand_matches("convert") {
        let source_dir = Path::new(convert_matches.get_one::<String>("source").unwrap());
        let output_dir = convert_matches
            .get_one::<String>("output")
            .map(Path::new)
            .unwrap_or(source_dir);
        let target_format = convert_matches.get_one::<String>("format").unwrap();

        if !source_dir.exists() || !source_dir.is_dir() {
            eprintln!("Source directory does not exist or is not a directory");
            std::process::exit(1);
        }

        if !output_dir.exists() {
            fs::create_dir_all(output_dir).expect("Failed to create output directory");
        }

        if let Err(e) = process_images(source_dir, output_dir, target_format) {
            eprintln!("Error processing images: {}", e);
        } else {
            println!("Image processing completed.");
        }

        return;
    }


    // Extract command-line arguments
    let source_dir = Path::new(matches.get_one::<String>("source").unwrap());
    let output_dir = matches
        .get_one::<String>("output")
        .map(Path::new)
        .unwrap_or(source_dir); // Default to the source directory if no output directory is provided
    let target_format = matches.get_one::<String>("format").unwrap();

    // Ensure the source directory exists and is a valid directory
    if !source_dir.exists() || !source_dir.is_dir() {
        eprintln!("Source directory does not exist or is not a directory");
        std::process::exit(1); // Exit if the source directory is invalid
    }

    // Ensure the output directory exists or create it
    if !output_dir.exists() {
        fs::create_dir_all(output_dir).expect("Failed to create output directory");
    }

    // Process images in the source directory and convert them to the target format
    if let Err(e) = process_images(source_dir, output_dir, target_format) {
        eprintln!("Error processing images: {}", e); // Log any errors during image processing
    } else {
        println!("Image processing completed."); // Notify the user when processing is complete
    }
}
