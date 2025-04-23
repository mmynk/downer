use std::{fs::{File, OpenOptions}, path::Path};
use std::io::Write;

use reqwest::{header, Client, Response};
use tokio_util::sync;
use futures_util::StreamExt;
use log::debug;

use crate::errors::Error;

pub struct Downloader {
    url: String,
    output_path: String,
    token: sync::CancellationToken,
    client: Client,
}

impl Downloader {
    pub fn new() -> Self {
        Self {
            url: String::new(),
            output_path: String::new(),
            token: sync::CancellationToken::new(),
            client: Client::new(),
        }
    }

    pub fn with_url(mut self, url: String) -> Self {
        self.url = url;
        if self.output_path.is_empty() {
            self.output_path = file_name_from_url(&self.url).to_string();
        }
        self
    }

    pub fn with_output_path(mut self, output_path: String) -> Self {
        self.output_path = output_path;
        self
    }

    pub fn with_token(mut self, token: sync::CancellationToken) -> Self {
        self.token = token;
        self
    }

    pub async fn download(&self) -> Result<(), Error> {
        download_file(&self.url, &self.output_path, self.token.clone(), &self.client).await
    }
}

fn file_name_from_url(url: &str) -> &str {
    url.split('/').last().unwrap_or(url)
}

async fn download_file(url: &str, output_path: &str, token: sync::CancellationToken, client: &Client) -> Result<(), Error> {
    if !Path::new(output_path).exists() {
        debug!("Downloading url={} to path={}", url, output_path);
        return start_download(url, output_path, token, client).await;
    }

    let file = File::open(output_path)?;
    let file_size = file.metadata()?.len();
    let mut headers = header::HeaderMap::new();
    headers.insert(
        "Range",
        header::HeaderValue::from_str(&format!("bytes={}-", file_size)).map_err(Error::InvalidHeaderValue)?,
    );
    let response = client.get(url).headers(headers).send().await?;
    let total_bytes = get_file_size(&response).await + file_size;

    if total_bytes == file_size {
        debug!("File already downloaded: {}", output_path);
        return Ok(());
    }

    if file_size > total_bytes || file_size == 0 {
        debug!("File size={} is greater than the total bytes={} or 0, starting download from scratch for url={} to path={}", file_size, total_bytes, url, output_path);
        return start_download(url, output_path, token, client).await;
    }

    debug!("Continuing download for url={} to path={}", url, output_path);
    return continue_download(url, output_path, token, client).await;
}

async fn start_download(url: &str, output_path: &str, token: sync::CancellationToken, client: &Client) -> Result<(), Error> {
    let response = client.get(url).send().await?;
    let total_bytes = get_file_size(&response).await;
    let mut out = File::create(output_path)?;
    let mut stream = response.bytes_stream();

    let file_name = output_path.split('/').last().unwrap_or(output_path);
    debug!("Downloading {}: total={}", file_name, pretty_size(total_bytes));
    let mut downloaded_bytes = 0;
    while let Some(chunk) = stream.next().await {
        if token.is_cancelled() {
            break;
        }
        let chunk = chunk?;
        out.write_all(&chunk)?;
        downloaded_bytes += chunk.len() as u64;
        update_progress(file_name, downloaded_bytes, total_bytes).await;
    }
    println!();

    Ok(())
}

async fn continue_download(url: &str, output_path: &str, token: sync::CancellationToken, client: &Client) -> Result<(), Error> {
    let file = File::open(output_path)?;
    let file_size = file.metadata()?.len();
    let mut headers = header::HeaderMap::new();
    headers.insert(
        "Range",
        header::HeaderValue::from_str(&format!("bytes={}-", file_size)).map_err(Error::InvalidHeaderValue)?,
    );
    let response = client.get(url).headers(headers).send().await?;
    let total_bytes = get_file_size(&response).await + file_size;

    let mut stream = response.bytes_stream();
    let mut file = OpenOptions::new().append(true).open(output_path)?;

    let file_name = output_path.split('/').last().unwrap_or(output_path);
    let mut downloaded_bytes = file_size;
    debug!("Downloading {}: remaining={} total={}", file_name, pretty_size(total_bytes - downloaded_bytes), pretty_size(total_bytes));
    while let Some(chunk) = stream.next().await {
        if token.is_cancelled() {
            break;
        }
        let chunk = chunk?;
        file.write_all(&chunk)?;
        downloaded_bytes += chunk.len() as u64;
        update_progress(file_name, downloaded_bytes, total_bytes).await;
    }

    Ok(())
}

async fn update_progress(file_name: &str, downloaded_bytes: u64, total_bytes: u64) {
    if total_bytes == 0 {
        println!("\r{}: {} bytes / unknown size", file_name, downloaded_bytes);
        return;
    }
    let progress = (downloaded_bytes as f64 / total_bytes as f64) * 100.0;
    let bar_width = 30;
    let filled_width = (progress / 100.0 * bar_width as f64) as usize;
    let empty_width = bar_width - filled_width;

    print!("\r{}: [{}{:>width$}] {:.2}%",
        file_name,
        "█".repeat(filled_width),
        "",
        progress,
        width = empty_width
    );
    std::io::stdout().flush().unwrap();
}

async fn get_file_size(response: &Response) -> u64 {
    let content_length = response.headers().get(header::CONTENT_LENGTH);
    if let Some(content_length) = content_length {
        content_length.to_str().unwrap_or("0").parse::<u64>().unwrap_or(0)
    } else {
        0
    }
}

fn pretty_size(size: u64) -> String {
    let units = ["B", "KB", "MB", "GB", "TB"];
    let mut size = size as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < units.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    format!("{:.2} {}", size, units[unit_index])
}
