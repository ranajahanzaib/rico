use clap::{Arg, ArgAction, ArgMatches, Command};
use image::{io::Reader as ImageReader, DynamicImage, ImageFormat, Rgba, RgbaImage};
use rayon::prelude::*;
use std::collections::VecDeque;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use walkdir::WalkDir;

/// Collects all image files with allowed extensions from the source directory.
fn collect_image_files(source_dir: &Path) -> Vec<PathBuf> {
    // Initialize an empty vector to store the paths of image files.
    let mut image_files = Vec::new();
    // Define a list of allowed image file extensions.
    let allowed_extensions = ["jpg", "jpeg", "png", "gif", "bmp", "webp", "tiff"];

    // Iterate through the source directory recursively using WalkDir.
    for entry in WalkDir::new(source_dir).into_iter().filter_map(Result::ok) {
        // Get the path of the current entry.
        let path = entry.path();
        // Check if the current entry is a file.
        if path.is_file() {
            // Get the file extension.
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                // Convert the extension to lowercase and check if it is in the allowed list.
                if allowed_extensions.contains(&ext.to_lowercase().as_str()) {
                    // If the extension is allowed, add the file path to the vector.
                    image_files.push(path.to_path_buf());
                }
            }
        }
    }

    // Return the vector of image file paths.
    image_files
}

/// Converts an image from its current format to a target format (e.g., PNG, JPEG, BMP).
/// This function will skip unsupported formats and files that cannot be decoded.
fn convert_image(
    input_path: &Path,
    output_dir: &Path,
    target_format: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Skip unsupported formats, such as SVG (image::guess_format will return an error for it)
    if let Some(ext) = input_path.extension() {
        let ext = ext.to_str().unwrap_or("").to_lowercase();
        if ext == "svg" {
            println!("Skipping SVG file: {:?}", input_path);
            return Ok(()); // Skip SVG files, as they're not supported
        }
    }

    // Open the input file and read its contents into a buffer.
    let mut file = std::fs::File::open(input_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    // Guess the format of the image based on its contents.
    let format = image::guess_format(&buffer)?;

    // If the format is unsupported, skip the file.
    if !matches!(
        format,
        ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::Bmp
    ) {
        println!("Skipping unsupported file format: {:?}", input_path);
        return Ok(()); // Skip unsupported file formats
    }

    // Try opening and decoding the image file.
    let img_result = ImageReader::open(input_path);

    // If the image cannot be decoded, skip the file.
    let img = match img_result {
        Ok(reader) => reader.decode(),
        Err(_) => {
            println!("Skipping file (could not decode): {:?}", input_path);
            return Ok(());
        }
    };

    // Unwrap the result of image decoding.
    let img = img?;

    // Create the output path by changing the file extension to the target format.
    let mut output_path = output_dir.to_path_buf();
    output_path.push(input_path.file_stem().unwrap());
    output_path.set_extension(target_format);

    // Check if the output file already exists.
    if output_path.exists() {
        println!("Output already exists for {:?}; skipping", input_path);
        return Ok(()); // Skip if the file already exists
    }

    // Determine the format to save the image based on the target_format string.
    let format = match target_format {
        "png" => ImageFormat::Png,
        "jpg" | "jpeg" => ImageFormat::Jpeg,
        "bmp" => ImageFormat::Bmp,
        "webp" => ImageFormat::WebP,
        // If the target format is not supported, return an error.
        _ => return Err(format!("Unsupported format: {}", target_format).into()),
    };

    // Save the image in the specified format.
    img.save_with_format(output_path.clone(), format)?;
    // Print a message indicating the successful conversion and the input/output paths.
    println!("Converted: {:?} -> {:?}", input_path, output_path);
    Ok(())
}

/// Traverses the source directory, processes all image files, and converts them to the specified format.
fn process_images(
    source_dir: &Path,
    output_dir: &Path,
    target_format: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Mutex is used to safely share the file list among threads.
    // Initialize a Mutex-protected vector to store the paths of files to be processed.

    // Mutex is used to safely share the file list among threads.
    // Initialize a Mutex-protected vector to store the paths of files to be processed.
    let files_to_process: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());

    // Traverse the source directory recursively using WalkDir.
    WalkDir::new(source_dir)
        .into_iter()
        .filter_map(Result::ok) // Ignore errors from unreadable directories.
        .for_each(|entry| {
            // Get the path of the current entry.
            let path = entry.path();

            // Check if the current entry is a file.
            if path.is_file() {
                // Get the file extension.
                if let Some(ext) = path.extension() {
                    // Convert the extension to lowercase.
                    let ext = ext.to_str().unwrap_or("").to_lowercase();

                    // Skip unsupported file formats like SVG.
                    if ext == "svg" {
                        println!("Skipping SVG file: {:?}", path);
                    } else if ext != target_format {
                        // Lock the mutex to safely access the shared file list.
                        let mut files = files_to_process.lock().unwrap();
                        // Add the file path to the list of files to be processed.
                        files.push(path.to_path_buf());
                        // Log that a supported image file was found.
                        println!("Found supported image file: {:?}", path);
                    }
                }
            }
        });

    // Retrieve the list of files to process by unlocking the mutex and extracting the vector.
    let files = files_to_process.into_inner().unwrap();

    // If no files were found to process, print a message and exit.
    if files.is_empty() {
        println!("No files found to convert!");
    }

    // Process the image files in parallel using rayon.
    files.par_iter().for_each(|file| {
        // Attempt to convert the image file.
        if let Err(e) = convert_image(file, output_dir, target_format) {
            // If an error occurs during conversion, log the error to stderr.
            eprintln!("Failed to process {:?}: {}", file, e);
        }
    });

    // Return Ok to indicate successful completion.
    Ok(())
}

/// Checks if two pixels are significantly different (i.e., an edge)
fn is_edge(p1: Rgba<u8>, p2: Rgba<u8>, edge_threshold: u8) -> bool {
    // Calculate the absolute difference between the red components of the two pixels.
    let diff_r = p1[0].abs_diff(p2[0]);
    // Calculate the absolute difference between the green components of the two pixels.
    let diff_g = p1[1].abs_diff(p2[1]);
    // Calculate the absolute difference between the blue components of the two pixels.
    let diff_b = p1[2].abs_diff(p2[2]);

    // Check if any of the color component differences exceed the edge threshold.
    // If any difference is greater than the threshold, it indicates a significant change in color,
    // which is considered an edge. This edge is used as a stopping point.
    diff_r > edge_threshold || diff_g > edge_threshold || diff_b > edge_threshold
}
/// Removes only the outer near-white background, stopping at edges.
fn remove_background(img: &DynamicImage, edge_threshold: u8) -> RgbaImage {
    // Convert the input image to Rgba8 format for pixel-level manipulation.
    let img = img.to_rgba8();
    // Get the dimensions of the image.
    let (width, height) = img.dimensions();
    // Create a clone of the input image to store the output.
    let mut output = img.clone();
    // Create a 2D vector to track visited pixels during BFS.
    let mut visited = vec![vec![false; width as usize]; height as usize];
    // Create a queue for BFS (Breadth-First Search).
    let mut queue = VecDeque::new();

    // Initialize BFS with border pixels.
    // Add all pixels on the top and bottom rows to the queue.
    for x in 0..width {
        queue.push_back((x, 0));
        queue.push_back((x, height - 1));
    }
    // Add all pixels on the left and right columns (excluding corners) to the queue.
    for y in 1..height - 1 {
        queue.push_back((0, y));
        queue.push_back((width - 1, y));
    }

    // Perform BFS to remove the background.
    while let Some((x, y)) = queue.pop_front() {
        // Skip pixels that are out of bounds or already visited.
        if x >= width || y >= height || visited[y as usize][x as usize] {
            continue;
        }
        // Mark the current pixel as visited.
        visited[y as usize][x as usize] = true;

        // Get the RGBA values of the current pixel.
        let pixel = img.get_pixel(x, y);
        let [r, g, b, _] = pixel.0;

        // If the pixel is near-white (R, G, B > 240) and not an edge, continue flood-fill.
        if r > 240 && g > 240 && b > 240 {
            // Flag to indicate if the pixel is surrounded by edges.
            let mut is_surrounded_by_edges = false;

            // Check neighboring pixels for strong edges.
            // If any neighboring pixel has a significant color difference (edge), set the flag.
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

            // If an edge is nearby, stop removing the background at this pixel.
            if is_surrounded_by_edges {
                continue;
            }

            // Make the background pixel transparent.
            output.put_pixel(x, y, Rgba([0, 0, 0, 0]));

            // Add neighboring pixels to the queue for further processing.
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

    // Return the processed image with the background removed.
    output
}

/// Removes the background from images in the specified source directory and saves the results to the output directory.
fn remove_bg_from_images(
    source_dir: &Path,
    output_dir: &Path,
    edge_threshold: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    // Check if the source directory exists and is a directory.
    if !source_dir.exists() || !source_dir.is_dir() {
        // If not, return an error.
        return Err("Source directory does not exist or is not a directory".into());
    }

    // Collect all image files from the source directory.
    let files = collect_image_files(source_dir);
    // Check if any files were found.
    if files.is_empty() {
        // If no images were found, print a message and return Ok.
        println!("No images found in the source directory.");
        return Ok(());
    }

    // Process each image file in parallel.
    files.par_iter().for_each(|input_path| {
        // Attempt to open and decode the image file.
        let img_result = ImageReader::open(input_path)
            .and_then(|reader| {
                reader
                    .decode()
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
            })
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e));

        // Handle the result of image decoding.
        let img = match img_result {
            // If decoding was successful, use the decoded image.
            Ok(img) => img,
            // If decoding failed, print a message and skip the file.
            Err(_) => {
                println!("Skipping file (could not decode): {:?}", input_path);
                return;
            }
        };

        // Remove the background from the image using the provided edge threshold.
        let processed_img = remove_background(&img, edge_threshold);

        // Get the relative path of the input file from the source directory.
        let relative_path = input_path.strip_prefix(source_dir).unwrap();

        // Construct the full output path by joining the output directory and the relative path.
        let mut output_path = output_dir.join(relative_path);
        // Ensure the output format is PNG by setting the file extension.
        output_path.set_extension("png");

        // Create parent directories for the output file if they don't exist.
        if let Some(parent) = output_path.parent() {
            if !parent.exists() {
                // If parent directory does not exist, create it and all necessary parent directories.
                fs::create_dir_all(parent).expect("Failed to create output subdirectory");
            }
        }

        // Save the processed image to the output path.
        if let Err(e) = processed_img.save(&output_path) {
            // If saving fails, print an error message to stderr.
            eprintln!("Failed to save {:?}: {}", output_path, e);
        } else {
            // If saving is successful, print a message indicating the input and output paths.
            println!("Processed: {:?} -> {:?}", input_path, output_path);
        }
    });

    // Return Ok to indicate successful completion.
    Ok(())
}

fn main() {
    // CLI argument parsing with clap
    let matches = Command::new("RICO - Rust Image Converter")
        .version("1.0") // Set the version of the CLI tool.
        .author("Rana Jahanzaib <work@withrana.com>")
        .about("RICO is a Rust-powered CLI tool for rapid, parallel image conversion.") // Set a brief description of the CLI tool.
        .subcommand(
            Command::new("remove") // Define the "remove" subcommand.
                .about("Remove background from images") // Set a description for the "remove" subcommand.
                .arg(
                    Arg::new("background") // Define the "background" argument.
                        .short('b') // Set the short flag for the argument.
                        .long("background") // Set the long flag for the argument.
                        .action(ArgAction::SetTrue) // Set the action to set the argument to true if present.
                        .help("Remove background from images"), // Set a help message for the argument.
                )
                .arg(
                    Arg::new("source") // Define the "source" argument.
                        .short('s') // Set the short flag for the argument.
                        .long("source") // Set the long flag for the argument.
                        .value_parser(clap::value_parser!(String)) // Set the value parser to parse the argument as a String.
                        .required(true) // Make the argument required.
                        .help("Source directory for input images"), // Set a help message for the argument.
                )
                .arg(
                    Arg::new("output") // Define the "output" argument.
                        .short('o') // Set the short flag for the argument.
                        .long("output") // Set the long flag for the argument.
                        .value_parser(clap::value_parser!(String)) // Set the value parser to parse the argument as a String.
                        .help("Output directory for processed images (optional, defaults to source directory)"), // Set a help message for the argument.
                )
                .arg(
                    Arg::new("edge-threshold") // Define the "edge-threshold" argument.
                        .short('e') // Set the short flag for the argument.
                        .long("edge-threshold") // Set the long flag for the argument.
                        .value_parser(clap::value_parser!(u8)) // Set the value parser to parse the argument as a u8.
                        .default_value("30") // Set a default value for the argument.
                        .help("Set the edge detection threshold (default: 30)"), // Set a help message for the argument.
                ),
        )
        .subcommand(
            Command::new("convert") // Define the "convert" subcommand.
                .about("Convert images to different formats") // Set a description for the "convert" subcommand.
                .arg(
                    Arg::new("source") // Define the "source" argument.
                        .short('s') // Set the short flag for the argument.
                        .long("source") // Set the long flag for the argument.
                        .value_parser(clap::value_parser!(String)) // Set the value parser to parse the argument as a String.
                        .required(true) // Make the argument required.
                        .help("Source directory for input images"), // Set a help message for the argument.
                )
                .arg(
                    Arg::new("output") // Define the "output" argument.
                        .short('o') // Set the short flag for the argument.
                        .long("output") // Set the long flag for the argument.
                        .value_parser(clap::value_parser!(String)) // Set the value parser to parse the argument as a String.
                        .help("Output directory for converted images (optional, defaults to source directory)"), // Set a help message for the argument.
                )
                .arg(
                    Arg::new("format") // Define the "format" argument.
                        .short('f') // Set the short flag for the argument.
                        .long("format") // Set the long flag for the argument.
                        .value_parser(clap::value_parser!(String)) // Set the value parser to parse the argument as a String.
                        .default_value("png") // Set a default value for the argument.
                        .help("Target format for conversion (e.g., png, jpg, bmp, webp)"), // Set a help message for the argument.
                ),
        )
        .get_matches(); // Parse the command-line arguments and get the matches.

    // Handle "remove" command
    if let Some(remove_matches) = matches.subcommand_matches("remove") {
        // Check if the "background" flag was provided in the "remove" subcommand.
        // This flag indicates whether to remove the background from images.
        let remove_bg = remove_matches.get_flag("background");

        // Get the source directory path from the "source" argument.
        // Unwrap is used because "source" is a required argument.
        let source_dir = Path::new(remove_matches.get_one::<String>("source").unwrap());

        // Determine the output directory path.
        // The output directory can be specified via an argument, or it defaults to a related directory.
        let output_dir = get_output_dir(remove_matches, source_dir);

        // Get the edge threshold value from the "edge-threshold" argument.
        // If "edge-threshold" is not provided, default to 30.
        let edge_threshold: u8 = *remove_matches.get_one::<u8>("edge-threshold").unwrap_or(&30);

        // Validate that the source directory exists and the output directory can be created.
        // This ensures that the program can proceed with the file operations.
        validate_directories(source_dir, output_dir);

        // If the "background" flag is set, proceed with background removal.
        if remove_bg {
            // Attempt to remove the background from images in the source directory and save them to the output directory.
            // The edge threshold is used to determine the sensitivity of the background removal algorithm.
            if let Err(e) = remove_bg_from_images(source_dir, output_dir, edge_threshold) {
                // If an error occurs during background removal, print the error message to stderr.
                eprintln!("Error removing background: {}", e);
            } else {
                // If background removal is successful, print a success message to stdout.
                println!("Background removal completed.");
            }
        }
        // Return from the function after handling the "remove" subcommand.
        // This ensures that no further subcommands are processed.
        return;
    }

    // Handle "convert" command
    if let Some(convert_matches) = matches.subcommand_matches("convert") {
        // Get the source directory path from the "source" argument.
        // Unwrap is used because "source" is a required argument.
        let source_dir = Path::new(convert_matches.get_one::<String>("source").unwrap());

        // Determine the output directory path.
        // The output directory can be specified via an argument, or it defaults to a related directory.
        let output_dir = get_output_dir(convert_matches, source_dir);

        // Get the target image format from the "format" argument.
        // Unwrap is used because "format" is a required argument.
        let target_format = convert_matches.get_one::<String>("format").unwrap();

        // Validate that the source directory exists and the output directory can be created.
        // This function ensures that the program can proceed with the file operations.
        validate_directories(source_dir, output_dir);

        // Attempt to process images in the source directory by converting them to the target format and saving them to the output directory.
        if let Err(e) = process_images(source_dir, output_dir, target_format) {
            // If an error occurs during image processing, print the error message to stderr.
            eprintln!("Error processing images: {}", e);
        } else {
            // If image processing is successful, print a success message to stdout.
            println!("Image processing completed.");
        }
        // Return from the function after handling the "convert" subcommand.
        // This ensures that no further subcommands are processed.
        return;
    }
}

/// Retrieves the output directory, defaulting to the source directory if not specified
fn get_output_dir<'a>(matches: &'a ArgMatches, source_dir: &'a Path) -> &'a Path {
    // Attempt to retrieve the "output" argument from the command-line matches.
    // If the "output" argument is present, convert it to a Path.
    // If the "output" argument is not present, use the source directory as the output directory.
    matches
        .get_one::<String>("output")
        .map(Path::new)
        .unwrap_or(source_dir)
}

/// Ensures that the source directory exists and the output directory is created if needed
fn validate_directories(source_dir: &Path, output_dir: &Path) {
    // Check if the source directory exists and is a directory.
    if !source_dir.exists() || !source_dir.is_dir() {
        // If the source directory does not exist or is not a directory, print an error message to stderr.
        eprintln!("Source directory does not exist or is not a directory");
        // Exit the program with an error code.
        std::process::exit(1);
    }

    // Check if the output directory exists.
    if !output_dir.exists() {
        // If the output directory does not exist, create it and all necessary parent directories.
        // If the creation fails, panic with an error message.
        fs::create_dir_all(output_dir).expect("Failed to create output directory");
    }
}
