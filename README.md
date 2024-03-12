# Simple HTTP Server Client

Simple HTTP Server Client is a versatile command-line utility designed to interact with a [Simple HTTP File Server](https://github.com/wiresock/simple-file-server). It allows users to generate files of a specified size, upload files to a server, download files (with an option for chunked downloads), and perform these operations multiple times based on user-defined iterations.

## Features

- File Generation: Create files with a specified size.
- File Upload: Upload files to a specified server.
- File Download: Download files from the server with an option for chunked downloads.
- Iterations: Perform upload and download operations multiple times.

## Usage

```bash
simple-file-client [OPTIONS]
```

### Options

- `-g`, `--generate <FILE>`: Generates a file of specified size.
- `-u`, `--upload <FILE>`: Uploads the specified file.
- `-d`, `--download <FILE>`: Downloads the specified file.
- `-c`, `--chunked`: Enables chunked download.
- `-s`, `--server <URL>`: Sets the server URL.
- `--size <SIZE>`: Sets the file size for generation.
- `-i`, `--iterations <NUMBER>`: Specifies the number of iterations for upload/download.

## Examples

1. Generate a file named `test.txt` with a size of 100,000,000 bytes:
    ```bash
    simple-file-client -g test.txt --size 100000000
    ```

2. Upload `test.txt` to the server and download the same file in chunked mode, repeating these operations 100 times:
    ```bash
    simple-file-client -u test.txt -d test.txt -c -s http://127.0.0.1:3000 -i 100
    ```

## Building and Running

1. Clone the repository:
    ```bash
    git clone https://github.com/yourusername/simple-file-client.git
    ```

2. Navigate to the project directory:
    ```bash
    cd simple-file-client
    ```

3. Build the project using Cargo (Rust's package manager and build tool):
    ```bash
    cargo build --release
    ```

4. The built binary will be available in `target/release`.

## Dependencies

- Rust Programming Language
- Clap for parsing command line arguments.
- Reqwest for HTTP requests.
- Sha2 and Hex for generating SHA-256 hashes.

---

For more information on how to use or contribute to Simple-File-Client, please refer to the project's repository.