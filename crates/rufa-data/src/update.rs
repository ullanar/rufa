use std::io::Write;
use std::path::Path;
use std::time::Duration;

use crate::{NAMES_FILE, SDE_FILE, TEXTS_FILE};

const RELEASES: &str = "https://github.com/EVEShipFit/sde-patched/releases";
pub const VERSION_FILE: &str = "version.txt";
const BODY_LIMIT: u64 = 256 * 1024 * 1024;

#[derive(Debug)]
pub enum UpdateError {
    Http(String),
    Io(std::io::Error),
}

impl std::fmt::Display for UpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdateError::Http(e) => write!(f, "download failed: {e}"),
            UpdateError::Io(e) => write!(f, "{e}"),
        }
    }
}

impl From<ureq::Error> for UpdateError {
    fn from(e: ureq::Error) -> UpdateError {
        UpdateError::Http(e.to_string())
    }
}

impl From<std::io::Error> for UpdateError {
    fn from(e: std::io::Error) -> UpdateError {
        UpdateError::Io(e)
    }
}

pub fn needs_download(dir: &Path) -> bool {
    !dir.join(SDE_FILE).is_file()
}

pub fn installed_tag(dir: &Path) -> Option<String> {
    std::fs::read_to_string(dir.join(VERSION_FILE))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(120)))
        .user_agent("rufa (https://github.com/EVEShipFit)")
        .build()
        .new_agent()
}

pub fn latest_tag() -> Result<String, UpdateError> {
    let response = agent()
        .get(format!("{RELEASES}/latest"))
        .config()
        .max_redirects(0)
        .http_status_as_error(false)
        .build()
        .call()?;
    let location = response
        .headers()
        .get("location")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| UpdateError::Http("no redirect to the latest release".to_string()))?;
    location
        .rsplit('/')
        .next()
        .filter(|tag| !tag.is_empty())
        .map(str::to_string)
        .ok_or_else(|| UpdateError::Http(format!("odd release location {location}")))
}

#[derive(Debug, Clone, PartialEq)]
pub struct Progress {
    pub file: &'static str,
    pub received: u64,
    pub total: Option<u64>,
}

pub const FILES: [&str; 3] = [SDE_FILE, NAMES_FILE, TEXTS_FILE];

pub fn download(dir: &Path, mut progress: impl FnMut(Progress)) -> Result<String, UpdateError> {
    use std::io::Read;
    std::fs::create_dir_all(dir)?;
    let agent = agent();
    let tag = latest_tag()?;
    for file in FILES {
        let url = format!("{RELEASES}/download/{tag}/{file}");
        let mut response = agent.get(&url).call()?;
        let total = response
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok());
        progress(Progress {
            file,
            received: 0,
            total,
        });
        let mut reader = response.body_mut().with_config().limit(BODY_LIMIT).reader();
        let part = dir.join(format!("{file}.part"));
        let mut out = std::fs::File::create(&part)?;
        let mut buffer = [0u8; 64 * 1024];
        let mut received: u64 = 0;
        loop {
            let n = reader.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            out.write_all(&buffer[..n])?;
            received += n as u64;
            progress(Progress {
                file,
                received,
                total,
            });
        }
        out.flush()?;
        drop(out);
        std::fs::rename(&part, dir.join(file))?;
    }
    std::fs::write(dir.join(VERSION_FILE), format!("{tag}\n"))?;
    Ok(tag)
}
