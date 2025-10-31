use anyhow::{Context, Result};
use std::{path::PathBuf, process::Command};

pub fn ensure_ytdlp_exists() -> Result<PathBuf> {
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

    // If yt-dlp already exists, try updating it
    if exe_path.exists() {
        let update_status = Command::new(&exe_path)
            .arg("-U")
            .status()
            .context("Failed to run yt-dlp updater")?;

        if update_status.success() {
            return Ok(exe_path);
        } else {
            eprintln!("yt-dlp update failed — redownloading latest binary...");
            std::fs::remove_file(&exe_path).ok();
        }
    }

    // Download yt-dlp if it doesn't exist or update failed
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
