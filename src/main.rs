mod installer;

use anyhow::{Context, Result};
use clap::Parser;
use installer::ensure_ytdlp_exists;
use std::process::Command;

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// YouTube video URL
    url: String,

    /// Optional output template (e.g. "%(title)s.%(ext)s")
    #[arg(short = 'o', long)]
    output: Option<String>,

    /// Download audio only (extracts best available audio)
    #[arg(short = 'a', long)]
    audio_only: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let ytdlp_path = ensure_ytdlp_exists()?;

    // Run yt-dlp
    let mut cmd = Command::new(&ytdlp_path);
    cmd.arg(&args.url);

    if let Some(out) = &args.output {
        cmd.args(["-o", out]);
    }

    if args.audio_only {
        cmd.args(["-x", "--audio-format", "mp3", "--audio-quality", "0"]);
    }

    let status = cmd.status().context("Failed to execute yt-dlp")?;
    if !status.success() {
        eprintln!("yt-dlp failed with exit code {:?}", status.code());
        return Ok(());
    }
    Ok(())
}
