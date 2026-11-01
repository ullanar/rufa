mod character;
pub mod implants;
mod prices;
mod settings;
pub mod targets;

use std::path::{Path, PathBuf};

use rufa_data::InfoEsf;
use rufa_sim::Fit;
use rufa_sim::format::esf;

pub use character::{Character, delete_character, list_characters, save_character};
pub use implants::ImplantSet;
pub use prices::{Cached, PriceCache};
pub use settings::Settings;
pub use targets::TargetProfile;

pub const FIT_EXT: &str = "esf";

pub fn config_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("RUFA_CONFIG_DIR") {
        return PathBuf::from(dir);
    }
    if let Some(dir) = std::env::var_os("XDG_CONFIG_HOME") {
        return PathBuf::from(dir).join("rufa");
    }
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    home.join(".config").join("rufa")
}

pub fn fits_dir(config: &Path) -> PathBuf {
    config.join("fits")
}

#[derive(Debug)]
pub enum StoreError {
    Io(std::io::Error),
    Format(esf::Error),
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::Io(e) => write!(f, "{e}"),
            StoreError::Format(e) => write!(f, "{e}"),
        }
    }
}

impl From<std::io::Error> for StoreError {
    fn from(e: std::io::Error) -> StoreError {
        StoreError::Io(e)
    }
}

impl From<esf::Error> for StoreError {
    fn from(e: esf::Error) -> StoreError {
        StoreError::Format(e)
    }
}

#[derive(Debug, Clone)]
pub struct SavedFit {
    pub path: PathBuf,
    pub name: String,
    pub hull: Option<i32>,
}

pub fn list_fits(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    let entries = match std::fs::read_dir(fits_dir(dir)) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(out),
        Err(e) => return Err(e),
    };
    for entry in entries {
        let path = entry?.path();
        if path.extension().is_some_and(|ext| ext == FIT_EXT) {
            out.push(path);
        }
    }
    out.sort();
    Ok(out)
}

pub fn saved_fits(info: &impl InfoEsf, dir: &Path) -> std::io::Result<Vec<SavedFit>> {
    let mut out = Vec::new();
    for path in list_fits(dir)? {
        let Ok(fit) = load_fit(info, &path) else {
            continue;
        };
        out.push(SavedFit {
            name: fit.name.clone().unwrap_or_else(|| file_stem(&path)),
            hull: Some(fit.ship.type_id),
            path,
        });
    }
    Ok(out)
}

fn file_stem(path: &Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

pub fn load_fit(info: &impl InfoEsf, path: &Path) -> Result<Fit, StoreError> {
    let bytes = std::fs::read(path)?;
    let fits = esf::load_esf(info, &bytes)?;
    let main = esf::main_fit(&fits).ok_or_else(|| {
        StoreError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "file holds no fit",
        ))
    })?;
    Ok(esf::to_fit(info, main)?)
}

pub fn fit_to_text(info: &impl InfoEsf, fit: &Fit) -> Result<String, StoreError> {
    let esf_fit = esf::from_fit(info, fit)?;
    Ok(esf::save_esf(info, &[esf_fit])?)
}

pub fn save_fit(
    info: &impl InfoEsf,
    dir: &Path,
    fit: &Fit,
    path: Option<&Path>,
    hull_name: &str,
) -> Result<PathBuf, StoreError> {
    let text = fit_to_text(info, fit)?;
    let path = match path {
        Some(p) => p.to_path_buf(),
        None => {
            let fits = fits_dir(dir);
            std::fs::create_dir_all(&fits)?;
            unique_path(&fits, &file_name_for(fit, hull_name))
        }
    };
    std::fs::write(&path, text)?;
    Ok(path)
}

fn file_name_for(fit: &Fit, hull_name: &str) -> String {
    let name = fit
        .name
        .as_deref()
        .filter(|n| !n.trim().is_empty())
        .unwrap_or(hull_name);
    let mut out = String::new();
    for c in name.chars() {
        out.push(if c.is_alphanumeric() || c == '-' || c == '_' {
            c
        } else {
            ' '
        });
    }
    let out = out
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
        .to_lowercase();
    if out.is_empty() {
        "fit".to_string()
    } else {
        out
    }
}

fn unique_path(dir: &Path, stem: &str) -> PathBuf {
    let first = dir.join(format!("{stem}.{FIT_EXT}"));
    if !first.exists() {
        return first;
    }
    (2..)
        .map(|n| dir.join(format!("{stem}-{n}.{FIT_EXT}")))
        .find(|p| !p.exists())
        .unwrap()
}
