use anyhow::{Context, Result};
use clap::Parser;
use std::{fs, io::Write, net::TcpListener, path::PathBuf, process::Command, time::SystemTime};

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

    /// Serve the downloaded file over LAN
    #[arg(long)]
    serve: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let ytdlp_path = ensure_ytdlp_exists()?;

    // start the command with the path to the executable
    let mut cmd = Command::new(&ytdlp_path);
    // add the url as an arg
    cmd.arg(&args.url);

    // throw on the output template if given
    if let Some(out) = args.output.clone() {
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
        return Ok(());
    }

    // If the user requested to serve the file over LAN
    if args.serve {
        // Determine which file to serve
        let output_path = determine_output_path(&args)?;

        serve_over_lan(&output_path)?;
    }

    Ok(())
}

/// Try to find the output file path to serve
fn determine_output_path(args: &Args) -> Result<PathBuf> {
    if let Some(template) = &args.output {
        let path = PathBuf::from(template);
        if path.exists() {
            return Ok(path);
        }
        let fallback = std::env::current_dir()?.join(template);
        if fallback.exists() {
            return Ok(fallback);
        }
    }

    // If no output given, try to find the most recently modified media file
    let mut latest: Option<(PathBuf, SystemTime)> = None;
    for entry in fs::read_dir(".")? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                // crude heuristic for media files
                if ["mp3", "mp4", "m4a", "webm", "opus"].contains(&ext) {
                    let modified = entry.metadata()?.modified()?;
                    if latest.as_ref().map_or(true, |(_, t)| modified > *t) {
                        latest = Some((path, modified));
                    }
                }
            }
        }
    }

    if let Some((path, _)) = latest {
        Ok(path)
    } else {
        anyhow::bail!("Could not determine downloaded file to serve")
    }
}

/// Serves the specified file on the local network via HTTP
fn serve_over_lan(file_path: &PathBuf) -> Result<()> {
    println!("\nStarting LAN share...");

    // Try to read the file
    let file_data =
        fs::read(file_path).with_context(|| format!("Failed to read file {:?}", file_path))?;

    // Bind a simple TCP listener on port 42069 to all interfaces
    let listener = TcpListener::bind("0.0.0.0:42069").context("Failed to bind to port 42069")?;

    // let output = Command::new("hostname")
    //     .arg("-i")
    //     .output()
    //     .context("Failed to get IP")?;
    // let ip = String::from_utf8_lossy(&output.stdout)
    //     .split_whitespace()
    //     .next()
    //     .unwrap()
    //     .to_string();

    println!(
        "Serving {:?} on http://0.0.0.0:42069\n\nUse Ctrl+C to stop.",
        file_path.file_name().unwrap_or_default(),
    );

    // println!(
    //     "Serving {:?} on http://{:?}:42069\n\nUse Ctrl+C to stop.",
    //     file_path.file_name().unwrap_or_default(),
    //     ip
    // );

    // Wait for one incoming connection, serve the file, then exit.
    let (mut stream, peer_addr) = listener
        .accept()
        .context("Failed to accept incoming connection")?;
    println!("Client connected from {}", peer_addr);

    // Write a minimal HTTP response and the file contents
    stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\n\r\n")?;
    stream.write_all(&file_data)?;
    stream.flush()?;
    println!("File served to client.");

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
