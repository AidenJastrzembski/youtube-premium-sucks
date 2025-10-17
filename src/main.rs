use anyhow::{Context, Result};
use axum::{
    Router,
    body::StreamBody,
    http::{StatusCode, header},
    routing::get,
};
use clap::Parser;
use std::{net::SocketAddr, path::PathBuf, process::Command, time::SystemTime};
use tokio::fs::File;
use tokio_util::io::ReaderStream;

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

    if args.serve {
        let file_path = determine_output_path(&args)?;
        serve_file(file_path).await?;
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
    for entry in std::fs::read_dir(".")? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
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

/// Serve file over LAN using Axum and async file streaming
async fn serve_file(file_path: PathBuf) -> Result<()> {
    if !file_path.exists() {
        anyhow::bail!("File does not exist: {:?}", file_path);
    }

    let file_path_for_closure = file_path.clone(); // clone here

    let app = Router::new().route(
        "/",
        get(move || {
            let file_path = file_path_for_closure.clone(); // use clone inside closure
            async move {
                let file = File::open(&file_path)
                    .await
                    .map_err(|_| StatusCode::NOT_FOUND)?;
                let stream = ReaderStream::new(file);
                let body = StreamBody::new(stream);

                Ok::<_, StatusCode>(
                    axum::response::Response::builder()
                        .header(
                            header::CONTENT_DISPOSITION,
                            format!(
                                "attachment; filename=\"{}\"",
                                file_path.file_name().unwrap().to_string_lossy()
                            ),
                        )
                        .header(header::CONTENT_TYPE, "application/octet-stream")
                        .body(body)
                        .unwrap(),
                )
            }
        }),
    );

    let addr = SocketAddr::from(([0, 0, 0, 0], 42069));
    println!("Serving {:?} at http://{}", file_path, addr); // now this works
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;
    Ok(())
}

/// Ensures yt-dlp is available, downloading it if needed
fn ensure_ytdlp_exists() -> Result<PathBuf> {
    let cache_dir = dirs::cache_dir()
        .context("Failed to locate cache directory")?
        .join("yt-dlp-cli");
    std::fs::create_dir_all(&cache_dir).context("Failed to create cache directory")?;

    let exe_name = if cfg!(windows) {
        "yt-dlp.exe"
    } else {
        "yt-dlp"
    };
    let exe_path = cache_dir.join(exe_name);

    if exe_path.exists() {
        return Ok(exe_path);
    }

    println!("Downloading yt-dlp...");

    let url = match std::env::consts::OS {
        "windows" => "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe",
        "linux" => "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp",
        "macos" => "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_macos",
        _ => anyhow::bail!("Unsupported OS"),
    };

    let bytes = reqwest::blocking::get(url)
        .context("Failed to download yt-dlp")?
        .bytes()
        .context("Failed to read yt-dlp bytes")?;
    std::fs::write(&exe_path, &bytes).context("Failed to save yt-dlp binary")?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&exe_path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&exe_path, perms)?;
    }

    println!("Saved yt-dlp to {:?}", exe_path);
    Ok(exe_path)
}
