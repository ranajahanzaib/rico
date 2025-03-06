# rico

RICO is the fastest, Rust-powered CLI tool designed for bulk image conversion and background removal.

It efficiently processes images in parallel, allowing you to quickly convert large numbers of files to your preferred format (e.g., PNG, JPEG) or remove backgrounds from thousands of images with ease.

## Performance

Unbelievably fast: RICO removed the background from 5,740 images in under 5 seconds and converted 5,740 images to PNG in ~1 second on an M1 Max MacBook Pro, showcasing its extraordinary parallel processing power.

## Features

- Convert images to PNG, JPEG, BMP, WEBP.
- Remove backgrounds from images using fast edge detection.
- Parallel processing for high-speed performance.
- Skips unsupported formats (e.g., SVG) automatically.
- Lightweight and efficient Rust-powered CLI.
- Automatically creates output directories if they don’t exist.

## Installation

#### Quick Install

You can quickly install Rico by running the following command in your terminal:

```sh
curl -fsSL https://raw.githubusercontent.com/ranajahanzaib/rico/main/install_rico.sh | sudo bash
```

This command will:

1. Download the latest version of Rico.
2. Build the release version using Cargo.
3. Move the rico binary to `/usr/local/bin` for system-wide access.

Note: Ensure you have sudo privileges as the script moves the binary to a system directory.

#### Build from Source

To build rico, you’ll need Rust installed. If you don’t have Rust, install it from https://www.rust-lang.org/.

##### 1. Clone the Repository

```sh
git clone https://github.com/ranajahanzaib/rico.git
cd rico
```

##### 2. Build the Project

```sh
cargo build --release
```

The compiled binary will be in the target/release directory.

##### 3. Move the Binary to Your PATH

```sh
sudo mv target/release/rico /usr/local/bin/
```

##### 4. Verify Installation

```sh
rico --version
```

## Usage

The rico CLI provides two main commands: **convert** and **remove**.

```sh
rico 1.0
RICO is a Rust-powered CLI tool for rapid, parallel image conversion.

USAGE:
rico <SUBCOMMAND> [OPTIONS]

SUBCOMMANDS:
remove  Remove background from images
convert Convert images to different formats
help    Print this help message

OPTIONS:
-h, --help Print help information
-V, --version Print version information
```

### 1. Converting Images to a Different Format

To convert images in a folder to another format:

```sh
rico convert -s images/ -o converted/ -f jpg

Options for convert command:

-s, --source <source> Source directory for input images (required)
-o, --output <output> Output directory for converted images (optional, defaults to source directory)
-f, --format <format> Target format (png, jpg, bmp, webp) [default: png]
```

#### Example Usage:

Convert all images in **images/** to WEBP and save them in **converted/**:

```sh
rico convert -s images/ -o converted/ -f webp
```

Convert images in-place:

```sh
rico convert -s images/ -f webp
```

### 1. Removing Backgrounds from Images

To remove backgrounds from images:

```sh
rico remove -s images/ -o processed/ --background

Options for remove command:

-s, --source <source> Source directory for input images (required)
-o, --output <output> Output directory for processed images (optional, defaults to source directory)
-b, --background Enable background removal
-e, --edge-threshold <value> Set the edge detection threshold (default: 30)

```

#### Example Usage:

Remove backgrounds from all images in images/ and save to processed/:

```sh
rico remove -s images/ -o processed/ -b
```

Remove backgrounds with a custom edge threshold:

```sh
rico remove -s images/ -o processed/ -b -e 40
```

### Supported Formats

#### Input Formats:

- PNG
- JPEG
- BMP
- WEBP

#### Output Formats (for convert command):

- PNG
- JPEG
- BMP
- WEBP

###### Unsupported formats (e.g., SVG) are automatically skipped.

## Contributing

We welcome contributions! Feel free to submit pull requests or open issues.

- Follow Rust’s style guide.
- Write tests where appropriate.
- Optimize performance where possible.

## License

This project is licensed under the MIT License. See LICENSE for details.

## 🚀 Start Using Rico Today!

```sh
rico convert -s images/ -o converted/ -f webp
```
