use anyhow::{Context, Result};
use clap::Parser;
use std::{fs, path::PathBuf, process::Command};

/*
*
*   I Literally hate youtube premium
*   This shit costs money and then you still see ads
*   I went to download some music and it was like
*
*   "Hey man! Gotta fork over those hard earned clams!"
*   So heres a tool i wrote in like an hour at 12:30am
*   that wraps yt-dlp and lets you download yt vids for
*   free
*
*   remember! everything is free if you are smart enough!
*
* */

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

fn main() -> Result<()> {
    let args = Args::parse();
    let ytdlp_path = ensure_ytdlp_exists()?;

    // start the command with the path to the executable
    let mut cmd = Command::new(&ytdlp_path);
    // add the url as an arg
    cmd.arg(&args.url);

    // throw on the output template if given
    if let Some(out) = args.output {
        cmd.args(["-o", &out]);
    }

    // if audio bool, throw on the audio args
    if args.audio_only {
        cmd.args([
            "-x", // extract audio
            "--audio-format",
            "mp3",
            "--audio-quality",
            "0", // best
        ]);
    }

    let status = cmd.status().context("Failed to execute yt-dlp")?;
    if !status.success() {
        eprintln!("yt-dlp failed with exit code {:?}", status.code());
    }

    Ok(())
}

/// Ensures yt-dlp is available, downloading it if needed
fn ensure_ytdlp_exists() -> Result<PathBuf> {
    let cache_dir = dirs::cache_dir()
        .context("Failed to locate cache directory")?
        .join("yt-dlp-cli");

    // create the cache directory where the executable will be saved
    fs::create_dir_all(&cache_dir).context("Failed to create cache directory")?;

    // name of the executable
    let exe_name = if cfg!(windows) {
        "yt-dlp.exe"
    } else {
        "yt-dlp"
    };
    // construct the path to the executable
    let exe_path = cache_dir.join(exe_name);

    // if the executable already exists, return it, and return early
    if exe_path.exists() {
        return Ok(exe_path);
    }

    println!("initializing...");

    let url = match std::env::consts::OS {
        // sadly we have to cater to windows, i use it for editing
        "windows" => "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe",
        // chad os
        "linux" => "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp",
        // proprietary chad os
        "macos" => "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_macos",
        _ => anyhow::bail!("Unsupported OS"),
    };

    println!("Downloading yt-dlp binary...");

    let bytes = reqwest::blocking::get(url)
        .context("Failed to download yt-dlp")?
        .bytes()
        .context("Failed to read yt-dlp bytes")?;

    fs::write(&exe_path, &bytes).context("Failed to save yt-dlp binary")?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&exe_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&exe_path, perms)?;
    }

    println!("Saved yt-dlp to {:?}", exe_path);
    Ok(exe_path)
}
