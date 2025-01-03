# rico

**rico** is a Rust-powered CLI tool for rapid, parallel image conversion.

It efficiently processes images in parallel, allowing you to quickly convert large numbers of files to your preferred format (e.g., PNG, JPEG) and save them to your chosen destination.

## Performance

In a test, **rico** converted 1,569 images to PNG on an M1 Max MacBook Pro in just 1.7 seconds, highlighting its parallel processing power.

### Features

- Convert images to common formats: PNG, JPEG, BMP.
- Skips unsupported formats (e.g., SVG) and files that cannot be decoded (yet).
- Parallel processing for faster conversions.
- Easy-to-use CLI interface.
- Automatically creates output directories if it doesn't exist.
- **Available for download via GitHub releases** (no need to build if you prefer not to).

### Installation

#### Option 1: Install via curl

You can quickly install Rico by running the following command in your terminal:

```sh
curl -fsSL https://raw.githubusercontent.com/ranajahanzaib/rico/main/install_rico.sh | sudo bash
```

This command will:

1. Download the latest version of Rico from the repository.
2. Build the release version using Cargo.
3. Move the rico binary to /usr/local/bin so it can be accessed system-wide.

**Note**: Ensure you have sudo privileges as the script moves the binary to a system directory.

#### Option 2: Build from Source

To build **rico**, you'll need to have Rust installed on your machine. If you don't have Rust yet, you can install it from [https://www.rust-lang.org/](https://www.rust-lang.org/).

##### 1. Clone the repository

```sh
git clone https://github.com/ranajahanzaib/rico.git
cd rico
```

##### 2. Build the project

Build the project using Cargo, Rust's package manager.

```sh
cargo build --release
```

The compiled binary can be found in the "target/release" directory.

##### 3. Run the converter

Move the binary to a directory in your PATH (e.g., /usr/local/bin):

```sh
sudo mv target/release/rico /usr/local/bin/
rico -s /path/to/source -o /path/to/output -f png
```

**Alternatively, download the latest release from the GitHub releases page.**

## Usage

The utility accepts the following command-line arguments:

```sh
rico 0.1
RICO is a Rust-powered CLI tool for rapid, parallel image conversion.

USAGE:
rico [OPTIONS]

OPTIONS:
-f, --format <format> # Target format for conversion (e.g., png, jpg, bmp) [default: png]
-o, --output <output> # Output directory for converted images (optional, defaults to source directory)
-s, --source <source> # Source directory for input images (required)
-h, --help Print help information
-V, --version Print version information

Arguments
• –source (-s): # The source directory containing the images you want to convert (required).
• –output (-o): # The output directory where the converted images will be saved (optional, defaults to the source directory).
• –format (-f): # The target format for the conversion (e.g., png, jpg, bmp). The default format is png.
```

#### Example

Convert all images in the ./images directory to JPG format and save them in the ./converted directory:

```sh
./rico -s ./images -o ./converted -f jpg
```

#### Parallel Processing

RICO utilizes the Rayon crate to process images concurrently, leading to significantly faster conversion times, especially with numerous files.

##### Supported Formats

- **Input**: PNG, JPEG, BMP
- **Output**: PNG, JPEG, BMP

**Unsupported formats (e.g., SVG) will be skipped automatically.**

### Contributing

We love contributions! Submit a pull request or open an issue. Kindly, follow the Rust style guide and write tests where appropriate.

### License

This project is freely available under the MIT License. Use, modify, and distribute it as you wish. See LICENSE for the full terms.
