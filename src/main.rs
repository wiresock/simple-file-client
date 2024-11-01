use chrono::Local;
use clap::{Arg, Command};
use rand::{distributions::Alphanumeric, Rng};
use reqwest::blocking::{ClientBuilder, Response};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;
use std::time::{Duration, Instant};
use thiserror::Error;

// Define a custom error type
#[derive(Error, Debug)]
pub enum DownloadError {
    #[error("Network error")]
    Network(#[from] reqwest::Error),

    #[error("IO error")]
    Io(#[from] io::Error),
}

fn generate_random_text_file(filename: &Path, size: usize) -> io::Result<String> {
    if filename.exists() && filename.metadata()?.len() as usize == size {
        println!(
            "File: {:?} already exists with the correct size of {} bytes.",
            filename, size
        );
        return Ok(hex::encode(Sha256::digest(&std::fs::read(filename)?)));
    }

    let mut file = File::create(filename)?;
    let mut generated_size = 0;
    let block_size = 1024;
    let mut hasher = Sha256::new();

    while generated_size < size {
        let remaining = size - generated_size;
        let chunk_size = std::cmp::min(block_size, remaining);
        let block: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(chunk_size)
            .map(char::from)
            .collect();

        let block_bytes = block.as_bytes();
        file.write_all(block_bytes)?;
        hasher.update(block_bytes);
        generated_size += chunk_size;
    }

    println!("Generated file: {:?}", filename);
    Ok(hex::encode(hasher.finalize()))
}

fn upload_file(
    server_url: &str,
    filename: &Path,
    timeout_secs: u64,
) -> Result<reqwest::blocking::Response, Box<dyn std::error::Error>> {
    let client = ClientBuilder::new()
        .danger_accept_invalid_certs(true)
        .timeout(Duration::from_secs(timeout_secs)) // Set the timeout to the specified number of seconds
        .build()?;

    let url = format!("{}/upload", server_url);
    let form = reqwest::blocking::multipart::Form::new().file("file", filename)?; // Propagate the error instead of unwrapping
    let response = client.post(url).multipart(form).send()?;
    Ok(response)
}

fn download_file(
    server_url: &str,
    filename: &str,
    chunked: bool,
) -> Result<(usize, String), DownloadError> {
    let client = ClientBuilder::new()
        .danger_accept_invalid_certs(true)
        .build()?;

    let endpoint = if chunked {
        "download-chunked"
    } else {
        "download"
    };
    let url = format!("{}/{}/{}", server_url, endpoint, filename);
    let mut response = client.get(url).send()?;

    let mut hasher = Sha256::new();
    let mut buffer = Vec::new();

    response.read_to_end(&mut buffer)?;

    hasher.update(&buffer);

    Ok((buffer.len(), hex::encode(hasher.finalize())))
}

fn delete_file(server_url: &str, filename: &str) -> reqwest::Result<Response> {
    let client = ClientBuilder::new()
        .danger_accept_invalid_certs(true)
        .build()?;

    let url = format!("{}/{}", server_url, filename);
    client.delete(url).send()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new("File Server Client")
        .version("1.0")
        .author("Vadim Smirnov <vadim@ntkernel.com>")
        .about("Handles file operations with a server")
        .arg(
            Arg::new("generate")
                .long("generate")
                .short('g')
                .value_name("FILE")
                .help("Generates a file of specified size"),
        )
        .arg(
            Arg::new("upload")
                .long("upload")
                .short('u')
                .value_name("FILE")
                .help("Uploads the specified file"),
        )
        .arg(
            Arg::new("download")
                .long("download")
                .short('d')
                .value_name("FILE")
                .help("Downloads the specified file"),
        )
        .arg(
            Arg::new("chunked")
                .long("chunked")
                .short('c')
                .help("Enables chunked download")
                .action(clap::ArgAction::SetTrue)
                .default_value("false"),
        ) // Set the action for this argument)
        .arg(
            Arg::new("server")
                .long("server")
                .short('s')
                .value_name("URL")
                .help("Sets the server URL")
                .required(false),
        )
        .arg(
            Arg::new("size")
                .long("size")
                .value_name("SIZE")
                .help("Sets the file size for generation"),
        )
        .arg(
            Arg::new("timeout")
                .long("timeout")
                .short('t')
                .value_name("TIMEOUT")
                .help("Specifies the HTTP request timeout for upload")
                .default_value("30"),
        ) // Default to 1 iteration)
        .arg(
            Arg::new("iterations")
                .long("iterations")
                .short('i')
                .value_name("NUMBER")
                .help("Specifies the number of iterations for upload/download")
                .default_value("1"),
        ) // Default to 1 iteration)
        .get_matches();

    if !matches.args_present() {
        println!("No arguments provided. Use --help for usage information.");
        return Ok(());
    }

    let server_url = matches.get_one::<String>("server");

    let iterations = matches
        .get_one::<String>("iterations")
        .and_then(|it| it.parse::<usize>().ok())
        .unwrap_or(1);

    let timeout = matches
        .get_one::<String>("timeout")
        .and_then(|it| it.parse::<u64>().ok())
        .unwrap_or(30);

    if let Some(file) = matches.get_one::<String>("generate") {
        let size = matches
            .get_one::<String>("size")
            .map(|s| s.parse().unwrap())
            .unwrap_or(1024);
        let path = Path::new(file);
        match generate_random_text_file(path, size) {
            Ok(hash) => println!("SHA256: {}", hash),
            Err(e) => eprintln!("Error: {}", e),
        }
    } else {
        let mut upload_durations = Vec::new();
        let mut download_durations = Vec::new();

        for _ in 0..iterations {
            // Check if upload is specified
            if let Some(file) = matches.get_one::<String>("upload") {
                if server_url.is_none() {
                    eprintln!(
                        "{} - Server URL is required for uploading files.",
                        Local::now()
                    );
                    std::process::exit(1);
                }
                let server = server_url.unwrap();

                // Attempt to delete the file from the server before uploading
                let _ = delete_file(server, file);

                // Proceed to upload the file
                println!("{} - Start uploading file: {}", Local::now(), file);

                // Record start time
                let start_time = Instant::now();

                match upload_file(server, Path::new(file), timeout) {
                    Ok(response) => {
                        // Calculate the duration and store it
                        let duration = start_time.elapsed();
                        upload_durations.push(duration);
                        println!(
                            "{} - {}: Uploaded. Status: {}\nTime taken: {:.2?} seconds",
                            Local::now(),
                            file,
                            response.status(),
                            duration
                        );
                    }
                    Err(e) => eprintln!("{} - Error uploading file {}: {}", Local::now(), file, e),
                }
            }

            // Check if download is specified
            if let Some(file) = matches.get_one::<String>("download") {
                if server_url.is_none() {
                    eprintln!(
                        "{} - Server URL is required for downloading files.",
                        Local::now()
                    );
                    std::process::exit(1);
                }

                let chunked = matches.get_one::<bool>("chunked").copied().unwrap_or(false);
                println!("{} - Start downloading file: {}", Local::now(), file);

                // Record start time
                let start_time = Instant::now();

                match download_file(server_url.unwrap(), file, chunked) {
                    Ok((size, hash)) => {
                        // Calculate the duration and store it
                        let duration = start_time.elapsed();
                        download_durations.push(duration);
                        println!(
                            "{} - {}: Downloaded chunked = {} Size = {} bytes SHA256: {}\nTime taken: {:.2?} seconds",
                            Local::now(),
                            file,
                            chunked,
                            size,
                            hash,
                            duration
                        );
                    }
                    Err(e) => {
                        eprintln!("{} - Error downloading file {}: {}", Local::now(), file, e)
                    }
                }
            }
        }

        // Calculate and print the average times
        if upload_durations.len() > 0 {
            let average_upload =
                upload_durations.iter().copied().sum::<Duration>() / upload_durations.len() as u32;
            println!(
                "{} - Average upload time: {:.2?} seconds",
                Local::now(),
                average_upload
            );
        }

        if download_durations.len() > 0 {
            let average_download = download_durations.iter().copied().sum::<Duration>()
                / download_durations.len() as u32;

            println!(
                "{} - Average download time: {:.2?} seconds",
                Local::now(),
                average_download
            );
        }
    }

    Ok(())
}
