//! Telling one saved fit from another, and remembering what came from Pyfa.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use rufa_data::InfoEsf;
use rufa_sim::Fit;

use crate::{StoreError, fit_to_text, list_fits, load_fit};

/// The name of a fit beside its shape. Two fits are the same when both match.
pub type Identity = (String, String);

const INDEX: &str = "imported.txt";

/// The canonical text of a fit with the name left out. `save_esf` always
/// renders canonical text, so two fits with the same items give the same
/// string whatever they are called.
pub fn shape(info: &impl InfoEsf, fit: &Fit) -> Result<String, StoreError> {
    let mut bare = fit.clone();
    bare.name = None;
    fit_to_text(info, &bare)
}

pub fn identity(info: &impl InfoEsf, fit: &Fit) -> Result<Identity, StoreError> {
    let name = fit.name.clone().unwrap_or_default();
    Ok((name, shape(info, fit)?))
}

/// The identity of every fit in the directory. A file that does not parse is
/// left out: it cannot be compared with anything.
pub fn saved_identities(info: &impl InfoEsf, dir: &Path) -> std::io::Result<BTreeSet<Identity>> {
    let mut out = BTreeSet::new();
    for path in list_fits(dir)? {
        if let Ok(fit) = load_fit(info, &path)
            && let Ok(id) = identity(info, &fit)
        {
            out.insert(id);
        }
    }
    Ok(out)
}

fn index_path(dir: &Path) -> PathBuf {
    dir.join(INDEX)
}

/// The Pyfa fits already imported from `db`, as ids whose file is still here.
/// A line pointing at a file someone deleted is dropped, so the fit comes back
/// on the next import.
pub fn imported_ids(dir: &Path, db: &Path) -> BTreeSet<i64> {
    let Ok(text) = std::fs::read_to_string(index_path(dir)) else {
        return BTreeSet::new();
    };
    let fits = crate::fits_dir(dir);
    let db = db.to_string_lossy();
    let mut out = BTreeSet::new();
    for line in text.lines() {
        let mut parts = line.split('\t');
        let (Some(id), Some(file), Some(from)) = (parts.next(), parts.next(), parts.next()) else {
            continue;
        };
        if from != db {
            continue;
        }
        let Ok(id) = id.parse::<i64>() else {
            continue;
        };
        if fits.join(file).exists() {
            out.insert(id);
        }
    }
    out
}

/// Writes down that a Pyfa fit became this file.
pub fn remember_import(dir: &Path, db: &Path, id: i64, file: &Path) -> std::io::Result<()> {
    use std::io::Write;
    let name = file.file_name().unwrap_or_default().to_string_lossy();
    std::fs::create_dir_all(dir)?;
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(index_path(dir))?;
    writeln!(f, "{id}\t{name}\t{}", db.to_string_lossy())
}

/// One group of files holding the same fit: the one to keep, and the rest.
#[derive(Debug, Clone)]
pub struct Duplicates {
    pub keep: PathBuf,
    pub remove: Vec<PathBuf>,
}

/// Groups the saved fits by identity and reports every group of more than one.
/// The file kept is the one with the shortest stem, ties by byte order: the
/// numeric suffix only ever makes a name longer, so the original wins.
pub fn duplicates(info: &impl InfoEsf, dir: &Path) -> std::io::Result<Vec<Duplicates>> {
    let mut groups: BTreeMap<Identity, Vec<PathBuf>> = BTreeMap::new();
    for path in list_fits(dir)? {
        if let Ok(fit) = load_fit(info, &path)
            && let Ok(id) = identity(info, &fit)
        {
            groups.entry(id).or_default().push(path);
        }
    }
    let mut out = Vec::new();
    for (_, mut paths) in groups {
        if paths.len() < 2 {
            continue;
        }
        paths.sort_by(|a, b| {
            let stem = |p: &Path| p.file_stem().unwrap_or_default().to_os_string();
            let (a, b) = (stem(a), stem(b));
            a.len().cmp(&b.len()).then_with(|| a.cmp(&b))
        });
        let keep = paths.remove(0);
        out.push(Duplicates {
            keep,
            remove: paths,
        });
    }
    Ok(out)
}

/// Deletes the copies and drops the index lines that named them.
pub fn remove_duplicates(dir: &Path, groups: &[Duplicates]) -> std::io::Result<usize> {
    let mut gone = BTreeSet::new();
    let mut removed = 0;
    for group in groups {
        for path in &group.remove {
            std::fs::remove_file(path)?;
            if let Some(name) = path.file_name() {
                gone.insert(name.to_string_lossy().into_owned());
            }
            removed += 1;
        }
    }
    prune_index(dir, &gone)?;
    Ok(removed)
}

fn prune_index(dir: &Path, gone: &BTreeSet<String>) -> std::io::Result<()> {
    let path = index_path(dir);
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Ok(());
    };
    let kept: Vec<&str> = text
        .lines()
        .filter(|line| {
            line.split('\t')
                .nth(1)
                .is_none_or(|file| !gone.contains(file))
        })
        .collect();
    let mut out = kept.join("\n");
    if !out.is_empty() {
        out.push('\n');
    }
    std::fs::write(path, out)
}
