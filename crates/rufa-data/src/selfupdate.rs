use std::io;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use crate::checksum;
use crate::update::{Release, UpdateError, agent};

/// The target triple this binary was built for, as the release assets name it.
pub const TARGET: &str = env!("RUFA_TARGET");

const OUTGOING: &str = ".rufa-outgoing-";
const BACKUP: &str = ".rufa-backup-";
const PROBE: &str = ".rufa-probe-";

/// Set on the replacement while it proves that it starts.
pub const SELF_TEST: &str = "RUFA_SELF_TEST";

const OWNED_PREFIXES: [&str; 5] = ["/usr", "/opt", "/bin", "/sbin", "/nix/store"];

pub fn asset_name(tag: &str) -> String {
    format!("rufa-{tag}-{TARGET}{}", std::env::consts::EXE_SUFFIX)
}

pub fn checksums_name(tag: &str) -> String {
    format!("rufa-{tag}-checksums.sha256")
}

fn unique(prefix: &str) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    format!("{prefix}{}-{now}", std::process::id())
}

fn owned_by_a_package_manager(path: &Path) -> bool {
    let text = path.to_string_lossy();
    if text.contains("Program Files") {
        return true;
    }
    OWNED_PREFIXES.iter().any(|prefix| {
        text.starts_with(&format!("{prefix}/")) || text.starts_with(&format!("{prefix}\\"))
    })
}

fn accepts_a_write(dir: &Path) -> bool {
    let probe = dir.join(unique(PROBE));
    let written = std::fs::write(&probe, b"rufa").is_ok();
    let _ = std::fs::remove_file(&probe);
    written
}

/// Whether the executable at this path may be swapped in place.
pub fn is_self_replaceable(exe: &Path) -> bool {
    if owned_by_a_package_manager(exe) {
        return false;
    }
    match exe.parent() {
        Some(dir) => accepts_a_write(dir),
        None => false,
    }
}

/// A staging path beside the executable, so the rename stays on one filesystem.
pub fn staging(exe: &Path) -> io::Result<PathBuf> {
    let dir = exe
        .parent()
        .ok_or_else(|| io::Error::other("the executable has no directory"))?;
    Ok(dir.join(unique(OUTGOING)))
}

#[derive(Debug)]
pub enum SwapError {
    NotReplaceable,
    Missing(String),
    Download(String),
    Digest { placed: String, expected: String },
    DoesNotStart(String),
    Io(io::Error),
}

impl std::fmt::Display for SwapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SwapError::NotReplaceable => write!(f, "this install is not ours to replace"),
            SwapError::Missing(what) => write!(f, "the release carries no {what}"),
            SwapError::Download(e) => write!(f, "{e}"),
            SwapError::Digest { placed, expected } => {
                write!(f, "the placed file hashes to {placed}, not {expected}")
            }
            SwapError::DoesNotStart(e) => write!(f, "the new build does not start: {e}"),
            SwapError::Io(e) => write!(f, "{e}"),
        }
    }
}

impl From<io::Error> for SwapError {
    fn from(e: io::Error) -> SwapError {
        SwapError::Io(e)
    }
}

impl From<UpdateError> for SwapError {
    fn from(e: UpdateError) -> SwapError {
        SwapError::Download(e.to_string())
    }
}

#[cfg(unix)]
fn make_executable(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = std::fs::metadata(path)?.permissions();
    permissions.set_mode(permissions.mode() | 0o755);
    std::fs::set_permissions(path, permissions)
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> io::Result<()> {
    Ok(())
}

fn starts(exe: &Path) -> Result<(), SwapError> {
    let output = std::process::Command::new(exe)
        .env(SELF_TEST, "1")
        .output()
        .map_err(|e| SwapError::DoesNotStart(e.to_string()))?;
    if output.status.success() {
        return Ok(());
    }
    Err(SwapError::DoesNotStart(format!(
        "exit status {}",
        output.status
    )))
}

/// Put `staged` in the place of `exe`, keeping the outgoing file until the new
/// one has hashed right and proved that it starts.
pub fn swap(exe: &Path, staged: &Path, expected: &str) -> Result<PathBuf, SwapError> {
    if !is_self_replaceable(exe) {
        return Err(SwapError::NotReplaceable);
    }
    let dir = exe.parent().ok_or(SwapError::NotReplaceable)?;
    make_executable(staged)?;
    let backup = dir.join(unique(BACKUP));
    std::fs::rename(exe, &backup)?;
    if let Err(e) = std::fs::rename(staged, exe) {
        let _ = std::fs::rename(&backup, exe);
        return Err(SwapError::Io(e));
    }
    let placed = match checksum::of_file(exe) {
        Ok(placed) => placed,
        Err(e) => {
            roll_back(&backup, exe);
            return Err(SwapError::Io(e));
        }
    };
    if !placed.eq_ignore_ascii_case(expected.trim()) {
        roll_back(&backup, exe);
        return Err(SwapError::Digest {
            placed,
            expected: expected.trim().to_string(),
        });
    }
    if let Err(e) = starts(exe) {
        roll_back(&backup, exe);
        return Err(e);
    }
    Ok(backup)
}

/// Rename the backup over the suspect file. Nothing is removed first, so a
/// failure here still leaves an executable on disk.
fn roll_back(backup: &Path, exe: &Path) {
    let _ = std::fs::rename(backup, exe);
}

/// Remove the leftovers of an earlier swap. Call it once the process holds its
/// single-instance lock, never before.
pub fn sweep(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with(OUTGOING) || name.starts_with(BACKUP) || name.starts_with(PROBE) {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

const LISTING_LIMIT: u64 = 1024 * 1024;
const BINARY_LIMIT: u64 = 512 * 1024 * 1024;

fn fetch_text(url: &str) -> Result<String, UpdateError> {
    Ok(agent()
        .get(url)
        .call()?
        .body_mut()
        .with_config()
        .limit(LISTING_LIMIT)
        .read_to_string()?)
}

fn fetch_file(
    url: &str,
    into: &Path,
    mut progress: impl FnMut(u64, Option<u64>),
) -> Result<(), UpdateError> {
    let mut response = agent().get(url).call()?;
    let total = response
        .headers()
        .get("content-length")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok());
    progress(0, total);
    let mut reader = response
        .body_mut()
        .with_config()
        .limit(BINARY_LIMIT)
        .reader();
    let mut out = std::fs::File::create(into)?;
    let mut buffer = vec![0u8; 256 * 1024];
    let mut received = 0u64;
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        out.write_all(&buffer[..read])?;
        received += read as u64;
        progress(received, total);
    }
    out.flush()?;
    Ok(())
}

/// Download the build for this platform and leave it staged beside `exe`.
/// The digest comes from the release before the binary does.
pub fn stage(
    release: &Release,
    exe: &Path,
    progress: impl FnMut(u64, Option<u64>),
) -> Result<(PathBuf, String), SwapError> {
    let name = asset_name(&release.tag);
    let binary = release
        .asset(&name)
        .ok_or_else(|| SwapError::Missing(name.clone()))?;
    let sums = release
        .asset(&checksums_name(&release.tag))
        .ok_or_else(|| SwapError::Missing(checksums_name(&release.tag)))?;
    let listing = fetch_text(&sums.url)?;
    let expected = checksum::find(&listing, &name)
        .ok_or_else(|| SwapError::Missing(format!("digest for {name}")))?;
    let staged = staging(exe)?;
    let part = staged.with_extension("part");
    fetch_file(&binary.url, &part, progress)?;
    let got = checksum::of_file(&part)?;
    if !got.eq_ignore_ascii_case(&expected) {
        let _ = std::fs::remove_file(&part);
        return Err(SwapError::Digest {
            placed: got,
            expected,
        });
    }
    std::fs::rename(&part, &staged)?;
    Ok((staged, expected))
}

/// Download, verify, and swap. The returned path is the outgoing build, kept
/// until the next start sweeps it.
pub fn update(
    release: &Release,
    exe: &Path,
    progress: impl FnMut(u64, Option<u64>),
) -> Result<PathBuf, SwapError> {
    if !is_self_replaceable(exe) {
        return Err(SwapError::NotReplaceable);
    }
    let (staged, expected) = stage(release, exe, progress)?;
    match swap(exe, &staged, &expected) {
        Ok(backup) => Ok(backup),
        Err(e) => {
            let _ = std::fs::remove_file(&staged);
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("rufa-selfupdate-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[cfg(unix)]
    fn script(path: &Path, body: &str) {
        std::fs::write(path, format!("#!/bin/sh\n{body}\n")).unwrap();
        make_executable(path).unwrap();
    }

    #[test]
    fn the_asset_name_carries_the_tag_and_the_target() {
        let name = asset_name("v0.4.0");
        assert!(name.starts_with("rufa-v0.4.0-"), "{name}");
        assert!(name.ends_with(std::env::consts::EXE_SUFFIX), "{name}");
        assert!(!TARGET.is_empty());
        assert_eq!(checksums_name("v0.4.0"), "rufa-v0.4.0-checksums.sha256");
    }

    #[test]
    fn a_package_managed_path_is_left_alone() {
        assert!(!is_self_replaceable(Path::new("/usr/bin/rufa")));
        assert!(!is_self_replaceable(Path::new("/nix/store/abc/rufa")));
        assert!(!is_self_replaceable(Path::new(
            "C:\\Program Files\\rufa\\rufa.exe"
        )));
        let dir = scratch("writable");
        let exe = dir.join("rufa");
        std::fs::write(&exe, b"old").unwrap();
        assert!(is_self_replaceable(&exe));
    }

    #[test]
    fn staging_sits_beside_the_executable() {
        let dir = scratch("staging");
        let exe = dir.join("rufa");
        let staged = staging(&exe).unwrap();
        assert_eq!(staged.parent(), Some(dir.as_path()));
    }

    #[cfg(unix)]
    #[test]
    fn a_good_build_takes_the_place_and_the_old_one_is_kept() {
        let dir = scratch("swap-ok");
        let exe = dir.join("rufa");
        script(&exe, "exit 0");
        let staged = dir.join("staged");
        script(&staged, "exit 0");
        let expected = checksum::of_file(&staged).unwrap();
        let backup = swap(&exe, &staged, &expected).unwrap();
        assert_eq!(checksum::of_file(&exe).unwrap(), expected);
        assert!(backup.is_file(), "the outgoing build stays until a sweep");
        assert!(!staged.exists());
    }

    #[cfg(unix)]
    #[test]
    fn a_wrong_digest_puts_the_old_build_back() {
        let dir = scratch("swap-digest");
        let exe = dir.join("rufa");
        script(&exe, "echo old");
        let before = checksum::of_file(&exe).unwrap();
        let staged = dir.join("staged");
        script(&staged, "echo new");
        let error = swap(&exe, &staged, &"0".repeat(64)).unwrap_err();
        assert!(matches!(error, SwapError::Digest { .. }), "{error}");
        assert_eq!(checksum::of_file(&exe).unwrap(), before);
    }

    #[cfg(unix)]
    #[test]
    fn a_build_that_does_not_start_puts_the_old_build_back() {
        let dir = scratch("swap-start");
        let exe = dir.join("rufa");
        script(&exe, "exit 0");
        let before = checksum::of_file(&exe).unwrap();
        let staged = dir.join("staged");
        script(&staged, "exit 3");
        let expected = checksum::of_file(&staged).unwrap();
        let error = swap(&exe, &staged, &expected).unwrap_err();
        assert!(matches!(error, SwapError::DoesNotStart(_)), "{error}");
        assert_eq!(checksum::of_file(&exe).unwrap(), before);
    }

    #[test]
    fn a_sweep_removes_the_leftovers_and_keeps_the_rest() {
        let dir = scratch("sweep");
        let exe = dir.join("rufa");
        std::fs::write(&exe, b"keep").unwrap();
        for name in [unique(OUTGOING), unique(BACKUP), unique(PROBE)] {
            std::fs::write(dir.join(name), b"gone").unwrap();
        }
        sweep(&dir);
        let left: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        assert_eq!(left, vec!["rufa".to_string()], "{left:?}");
    }
}
