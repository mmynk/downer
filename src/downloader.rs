use std::{fs::{File, OpenOptions}, path::Path};
use std::io::Write;

use reqwest::{header, Client};
use tokio_util::sync;
use futures_util::StreamExt;

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
        if !Path::new(&self.output_path).exists() {
            return start_download(&self.url, &self.output_path, self.token.clone(), &self.client).await;
        }

        return continue_download(&self.url, &self.output_path, self.token.clone(), &self.client).await;
    }
}

fn file_name_from_url(url: &str) -> &str {
    url.split('/').last().unwrap_or(url)
}

async fn start_download(url: &str, output_path: &str, token: sync::CancellationToken, client: &Client) -> Result<(), Error> {
    let response = client.get(url).send().await?;
    let total_bytes = response.content_length().unwrap_or(0);
    let mut out = File::create(output_path)?;
    let mut stream = response.bytes_stream();

    let mut downloaded_bytes = 0;
    while let Some(chunk) = stream.next().await {
        if token.is_cancelled() {
            break;
        }
        let chunk = chunk?;
        out.write_all(&chunk)?;
        downloaded_bytes += chunk.len() as u64;
        update_progress(&output_path, downloaded_bytes, total_bytes).await;
    }

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
    let total_bytes = response.content_length().unwrap_or(0);
    let mut stream = response.bytes_stream();
    let mut file = OpenOptions::new().append(true).open(output_path)?;

    let mut downloaded_bytes = file_size;
    while let Some(chunk) = stream.next().await {
        if token.is_cancelled() {
            break;
        }
        let chunk = chunk?;
        file.write_all(&chunk)?;
        downloaded_bytes += chunk.len() as u64;
        update_progress(&output_path, downloaded_bytes, total_bytes).await;
    }

    Ok(())
}

async fn update_progress(file_path: &str, downloaded_bytes: u64, total_bytes: u64) {
    let file_name = file_path.split('/').last().unwrap_or(file_path);
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
