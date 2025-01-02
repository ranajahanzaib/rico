use std::path::{Path, PathBuf};
use std::fs;
use std::sync::Mutex;
use std::io::Read;
use rayon::prelude::*;
use walkdir::WalkDir;
use image::{io::Reader as ImageReader, ImageFormat};
use clap::{Command, Arg, value_parser};

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
        _ => return Err(format!("Unsupported format: {}", target_format).into()), // Error for unsupported target formats
    };

    // Save the image in the specified format
    img.save_with_format(output_path.clone(), format)?;
    println!("Converted: {:?} -> {:?}", input_path, output_path);
    Ok(())
}

/// Traverses the source directory, processes all image files, and converts them to the specified format.
fn process_images(source_dir: &Path, output_dir: &Path, target_format: &str) -> Result<(), Box<dyn std::error::Error>> {
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

fn main() {
    // CLI argument parsing with clap
    let matches = Command::new("RICO - Rust Image Converter")
        .version("0.1")
        .author("Rana Jahanzaib <work@withrana.com")
        .about("RICO is a Rust-powered CLI tool for rapid, parallel image conversion.")
        .arg(
            Arg::new("source")
                .short('s')
                .long("source")
                .value_parser(value_parser!(String))
                .required(true)
                .help("Source directory for input images"), // The source directory for images to convert
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .value_parser(value_parser!(String))
                .help("Output directory for converted images (optional, defaults to source directory)"), // Optional output directory
        )
        .arg(
            Arg::new("format")
                .short('f')
                .long("format")
                .value_parser(value_parser!(String))
                .default_value("png")
                .help("Target format for conversion (e.g., png, jpg, bmp)"), // The target format (default is PNG)
        )
        .get_matches();

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
