use std::io::Write;
use std::path::Path;
use std::time::Duration;

use crate::{NAMES_FILE, SDE_FILE, TEXTS_FILE};

const RELEASES: &str = "https://github.com/EVEShipFit/sde-patched/releases";
const API: &str = "https://api.github.com/repos";
pub const REPO: &str = "ullanar/rufa";
pub const VERSION_FILE: &str = "version.txt";
const BODY_LIMIT: u64 = 256 * 1024 * 1024;
const JSON_LIMIT: usize = 4 * 1024 * 1024;

#[derive(Debug)]
pub enum UpdateError {
    Http(String),
    Parse(String),
    Io(std::io::Error),
}

impl std::fmt::Display for UpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdateError::Http(e) => write!(f, "download failed: {e}"),
            UpdateError::Parse(e) => write!(f, "unreadable release: {e}"),
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

pub(crate) fn agent() -> ureq::Agent {
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
pub struct Asset {
    pub name: String,
    pub url: String,
    pub size: u64,
}

/// A GitHub release, as much of it as an update window needs.
#[derive(Debug, Clone, PartialEq)]
pub struct Release {
    pub tag: String,
    pub published: String,
    pub notes: String,
    pub assets: Vec<Asset>,
}

impl Release {
    pub fn asset(&self, name: &str) -> Option<&Asset> {
        self.assets.iter().find(|asset| asset.name == name)
    }
}

pub fn latest_release(repo: &str) -> Result<Release, UpdateError> {
    let text = agent()
        .get(format!("{API}/{repo}/releases/latest"))
        .header("accept", "application/vnd.github+json")
        .call()?
        .body_mut()
        .with_config()
        .limit(JSON_LIMIT as u64)
        .read_to_string()?;
    read_release(&text)
}

pub fn read_release(text: &str) -> Result<Release, UpdateError> {
    let json: serde_json::Value =
        serde_json::from_str(text).map_err(|e| UpdateError::Parse(e.to_string()))?;
    let tag = json["tag_name"]
        .as_str()
        .filter(|tag| !tag.is_empty())
        .ok_or_else(|| UpdateError::Parse("no tag_name".to_string()))?;
    let assets = json["assets"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .filter_map(|asset| {
            Some(Asset {
                name: asset["name"].as_str()?.to_string(),
                url: asset["browser_download_url"].as_str()?.to_string(),
                size: asset["size"].as_u64().unwrap_or(0),
            })
        })
        .collect();
    Ok(Release {
        tag: tag.to_string(),
        published: json["published_at"]
            .as_str()
            .unwrap_or_default()
            .chars()
            .take(10)
            .collect(),
        notes: json["body"].as_str().unwrap_or_default().to_string(),
        assets,
    })
}

/// Major, minor and patch out of `v0.3.0`, `0.3.0` or `v0.3.0-rc.1`.
pub fn version(text: &str) -> Option<(u32, u32, u32)> {
    let text = text.trim().trim_start_matches('v');
    let mut parts = text.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts
        .next()
        .unwrap_or("0")
        .split(['-', '+'])
        .next()?
        .parse()
        .ok()?;
    Some((major, minor, patch))
}

/// The page that carries the release notes of one tag.
pub fn release_page(repo: &str, tag: &str) -> String {
    format!("https://github.com/{repo}/releases/tag/{tag}")
}

pub fn is_newer(tag: &str, installed: &str) -> bool {
    match (version(tag), version(installed)) {
        (Some(found), Some(running)) => found > running,
        _ => false,
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
        "tag_name": "v0.4.0",
        "published_at": "2026-11-01T09:08:12Z",
        "body": "notes",
        "assets": [
            {
                "name": "rufa-v0.4.0-x86_64-pc-windows-msvc.exe",
                "browser_download_url": "https://example.invalid/rufa.exe",
                "size": 42
            },
            { "name": "rufa-v0.4.0-checksums.b3" }
        ]
    }"#;

    #[test]
    fn a_release_keeps_the_tag_the_date_and_the_named_assets() {
        let release = read_release(SAMPLE).unwrap();
        assert_eq!(release.tag, "v0.4.0");
        assert_eq!(release.published, "2026-11-01");
        assert_eq!(release.notes, "notes");
        let exe = release
            .asset("rufa-v0.4.0-x86_64-pc-windows-msvc.exe")
            .unwrap();
        assert_eq!(exe.url, "https://example.invalid/rufa.exe");
        assert_eq!(exe.size, 42);
        assert_eq!(release.assets.len(), 1, "an asset without a url is dropped");
    }

    #[test]
    fn a_release_without_a_tag_does_not_parse() {
        assert!(matches!(
            read_release(r#"{"body": "x"}"#),
            Err(UpdateError::Parse(_))
        ));
    }

    #[test]
    fn the_release_page_is_the_tag_page_of_the_repository() {
        assert_eq!(
            release_page("ullanar/rufa", "v0.4.0"),
            "https://github.com/ullanar/rufa/releases/tag/v0.4.0"
        );
    }

    #[test]
    fn versions_compare_across_the_v_prefix_and_a_pre_release() {
        assert_eq!(version("v0.3.0"), Some((0, 3, 0)));
        assert_eq!(version("0.3.0"), Some((0, 3, 0)));
        assert_eq!(version("v1.2.3-rc.1"), Some((1, 2, 3)));
        assert_eq!(version("latest"), None);
        assert!(is_newer("v0.4.0", "0.3.0"));
        assert!(is_newer("v0.3.1", "0.3.0"));
        assert!(!is_newer("v0.3.0", "0.3.0"));
        assert!(!is_newer("v0.2.9", "0.3.0"));
        assert!(!is_newer("nightly", "0.3.0"));
    }
}
