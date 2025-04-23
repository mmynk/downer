pub mod downloader;
pub mod errors;

use clap::Parser;
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
    let args = parse_args();

    let cancel_token = sync::CancellationToken::new();
    let token_clone = cancel_token.clone();

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
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn parse_args() -> Args {
    let args = Args::parse();
    args
}
