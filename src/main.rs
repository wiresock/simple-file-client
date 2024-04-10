use clap::{Arg, Command};
use rand::{distributions::Alphanumeric, Rng};
use reqwest::blocking::{ClientBuilder, Response};
use std::fs::File;
use sha2::{Sha256, Digest};
use std::io::{self, Read, Write};
use std::path::Path;
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
        println!("File: {:?} already exists with the correct size of {} bytes.", filename, size);
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

fn upload_file(server_url: &str, filename: &Path) -> reqwest::Result<reqwest::blocking::Response> {
    let client = ClientBuilder::new()
        .danger_accept_invalid_certs(true)
        .build()?;

    let url = format!("{}/upload", server_url);
    let form = reqwest::blocking::multipart::Form::new()
        .file("file", filename).unwrap();
    let response = client.post(url)
        .multipart(form)
        .send()?;
    Ok(response)
}

fn download_file(server_url: &str, filename: &str, chunked: bool) -> Result<String, DownloadError> {
    let client = ClientBuilder::new()
        .danger_accept_invalid_certs(true)
        .build()?;

    let endpoint = if chunked { "download-chunked" } else { "download" };
    let url = format!("{}/{}/{}", server_url, endpoint, filename);
    let mut response = client.get(url).send()?;

    let mut hasher = Sha256::new();
    let mut buffer = Vec::new();

    response.read_to_end(&mut buffer)?;

    hasher.update(&buffer);

    Ok(hex::encode(hasher.finalize()))
}

fn delete_file(server_url: &str, filename: &str) -> reqwest::Result<Response> {
    let client = ClientBuilder::new()
        .danger_accept_invalid_certs(true)
        .build()?;

    let url = format!("{}/{}", server_url, filename);
    client.delete(url).send()
}

fn main() -> reqwest::Result<()> {
    let matches = Command::new("File Server Client")
        .version("1.0")
        .author("Vadim Smirnov <vadim@ntkernel.com>")
        .about("Handles file operations with a server")
        .arg(Arg::new("generate")
             .long("generate")
             .short('g')
             .value_name("FILE")
             .help("Generates a file of specified size"))
        .arg(Arg::new("upload")
             .long("upload")
             .short('u')
             .value_name("FILE")
             .help("Uploads the specified file"))
        .arg(Arg::new("download")
             .long("download")
             .short('d')
             .value_name("FILE")
             .help("Downloads the specified file"))
        .arg(Arg::new("chunked")
             .long("chunked")
             .short('c')
             .help("Enables chunked download")
             .action(clap::ArgAction::SetTrue)) // Set the action for this argument)
        .arg(Arg::new("server")
             .long("server")
             .short('s')
             .value_name("URL")
             .help("Sets the server URL")
             .required(false))
        .arg(Arg::new("size")
             .long("size")
             .value_name("SIZE")
             .help("Sets the file size for generation"))
        .arg(Arg::new("iterations")
             .long("iterations")
             .short('i')
             .value_name("NUMBER")
             .help("Specifies the number of iterations for upload/download")
             .default_value("1")) // Default to 1 iteration)
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

    if let Some(file) = matches.get_one::<String>("generate") {
        let size = matches.get_one::<String>("size").map(|s| s.parse().unwrap()).unwrap_or(1024);
        let path = Path::new(file);
        match generate_random_text_file(path, size) {
            Ok(hash) => println!("SHA256: {}", hash),
            Err(e) => eprintln!("Error: {}", e),
        }
    } else {
        for _ in 0..iterations {
            // Check if upload is specified
            if let Some(file) = matches.get_one::<String>("upload") {
                if server_url.is_none() {
                    eprintln!("Server URL is required for uploading files.");
                    std::process::exit(1);
                }
                let server = server_url.unwrap();

                // Attempt to delete the file from the server before uploading
                let _ = delete_file(server, file);

                // Proceed to upload the file
                let response = upload_file(server, Path::new(file))?;
                println!("Uploaded file: {}, Status: {}", file, response.status());
            }

            // Check if download is specified
            if let Some(file) = matches.get_one::<String>("download") {
                if server_url.is_none() {
                    eprintln!("Server URL is required for downloading files.");
                    std::process::exit(1);
                }
                let chunked = matches.contains_id("chunked");
                match download_file(server_url.unwrap(), file, chunked) {
                    Ok(hash) => println!("Downloaded file {file} SHA256: {hash}"),
                    Err(e) => eprintln!("Error: {}", e),
                }
            }
        }
    }

    Ok(())
}