pub mod downloader;
pub mod errors;

use std::str::FromStr;

use clap::Parser;
use fern::colors::{Color, ColoredLevelConfig};
use log::{debug, error};
use tokio_util::sync;
use tokio::signal;

#[derive(Parser)]
struct Args {
    #[clap(short, long)]
    url: String,
    #[clap(short, long)]
    output: Option<String>,
}

#[tokio::main]
async fn main() {
    if let Err(e) = setup_logger() {
        eprintln!("Failed to setup logger: {}", e);
        std::process::exit(1);
    }

    let args = parse_args();

    let cancel_token = sync::CancellationToken::new();
    let token_clone = cancel_token.clone();
    let token_clone_2 = cancel_token.clone();

    // Spawn a task to handle Ctrl+C
    tokio::spawn(async move {
        signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
        cancel_token.cancel();
    });

    let mut downloader = downloader::Downloader::new()
        .with_url(args.url)
        .with_token(token_clone);

    if let Some(output) = args.output {
        downloader = downloader.with_output_path(output);
    }

    if let Err(e) = downloader.download().await {
        error!("Error: {}", e);
        std::process::exit(1);
    }

    if token_clone_2.is_cancelled() {
        debug!("Download cancelled!");
    } else {
        debug!("Download completed!");
    }
}

fn parse_args() -> Args {
    let args = Args::parse();
    args
}

fn setup_logger() -> Result<(), fern::InitError> {
    let grey = Color::TrueColor { r: 128, g: 128, b: 128 };
    let colors = ColoredLevelConfig::new()
        .debug(grey)
        .info(Color::Green)
        .warn(Color::Yellow)
        .error(Color::Red);

    let log_level = std::env::var("LOG_LEVEL").unwrap_or("INFO".to_string());
    fern::Dispatch::new()
        .format(move|out, message, record| {
            out.finish(format_args!(
                "[{}] {}",
                colors.color(record.level()),
                message
            ))
        })
        .level(log::LevelFilter::from_str(&log_level).unwrap())
        .chain(std::io::stdout())
        .apply()?;
    Ok(())
}
