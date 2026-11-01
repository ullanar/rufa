use std::io;
use std::path::{Path, PathBuf};

pub const APP_ID: &str = "rufa";
const RECEIPT: &str = "install.conf";
const DESKTOP: &str = "rufa.desktop";
const ICON: &[u8] = include_bytes!("../assets/icons/rufa-512.png");

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

/// Where an install puts the executable, with no administrator rights.
pub fn default_dir() -> Option<PathBuf> {
    if cfg!(target_os = "windows") {
        let base = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .or_else(|| home().map(|h| h.join("AppData").join("Local")))?;
        return Some(base.join("Programs").join("rufa"));
    }
    Some(home()?.join(".local").join("bin"))
}

pub fn exe_name() -> String {
    format!("rufa{}", std::env::consts::EXE_SUFFIX)
}

/// The path the receipt points at, if an install was recorded.
pub fn recorded(config_dir: &Path) -> Option<PathBuf> {
    let text = std::fs::read_to_string(config_dir.join(RECEIPT)).ok()?;
    let line = text
        .lines()
        .find_map(|line| line.split_once('='))
        .map(|(_, value)| value.trim().to_string())?;
    let path = PathBuf::from(line);
    path.is_file().then_some(path)
}

fn record(config_dir: &Path, exe: &Path) -> io::Result<()> {
    std::fs::create_dir_all(config_dir)?;
    std::fs::write(
        config_dir.join(RECEIPT),
        format!("path = {}\n", exe.display()),
    )
}

/// A binary under `target/debug` or `target/release` is a build, not an
/// install, so it never opens the install screen. Test binaries land one level
/// deeper, in `deps`.
pub fn in_a_build_tree(exe: &Path) -> bool {
    let Some(mut parent) = exe.parent() else {
        return false;
    };
    if parent.file_name().and_then(|n| n.to_str()) == Some("deps") {
        let Some(above) = parent.parent() else {
            return false;
        };
        parent = above;
    }
    let profile = parent.file_name().and_then(|n| n.to_str());
    let holder = parent
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str());
    matches!(profile, Some("debug" | "release")) && holder == Some("target")
}

/// Whether the running executable is the installed one.
pub fn running_installed(exe: &Path, config_dir: &Path) -> bool {
    match recorded(config_dir) {
        Some(recorded) => same_file(&recorded, exe),
        None => match default_dir() {
            Some(dir) => same_file(&dir.join(exe_name()), exe),
            None => false,
        },
    }
}

fn same_file(a: &Path, b: &Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
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

fn share_dir() -> Option<PathBuf> {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| home().map(|h| h.join(".local").join("share")))
}

/// Write the desktop entry and the hicolor icon, so the compositor matches the
/// window to a name and a picture. Only Linux reads these.
fn write_desktop_entry(share: &Path, exe: &Path) -> io::Result<()> {
    let icons = share.join("icons/hicolor/512x512/apps");
    std::fs::create_dir_all(&icons)?;
    std::fs::write(icons.join("rufa.png"), ICON)?;
    let applications = share.join("applications");
    std::fs::create_dir_all(&applications)?;
    let entry = format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=Rufa\n\
         GenericName=EVE Online fitting tool\n\
         Exec={} %f\n\
         Icon=rufa\n\
         Terminal=false\n\
         Categories=Game;Utility;\n\
         MimeType=application/x-eve-fit;\n\
         StartupWMClass={APP_ID}\n",
        exe.display()
    );
    std::fs::write(applications.join(DESKTOP), entry)
}

/// Copy the running build into `dir` and record where it went.
pub fn install(from: &Path, dir: &Path, config_dir: &Path) -> io::Result<PathBuf> {
    std::fs::create_dir_all(dir)?;
    let target = dir.join(exe_name());
    if same_file(from, &target) {
        return Ok(target);
    }
    let staged = dir.join(format!("{}.part", exe_name()));
    std::fs::copy(from, &staged)?;
    make_executable(&staged)?;
    if target.exists() {
        let outgoing = dir.join(format!(".rufa-outgoing-{}", std::process::id()));
        std::fs::rename(&target, &outgoing)?;
        let placed = std::fs::rename(&staged, &target);
        if placed.is_err() {
            let _ = std::fs::rename(&outgoing, &target);
            placed?;
        }
        let _ = std::fs::remove_file(&outgoing);
    } else {
        std::fs::rename(&staged, &target)?;
    }
    if cfg!(target_os = "linux")
        && let Some(share) = share_dir()
    {
        write_desktop_entry(&share, &target)?;
    }
    record(config_dir, &target)?;
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("rufa-setup-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn the_default_directory_needs_no_administrator() {
        let dir = default_dir().unwrap();
        let text = dir.to_string_lossy();
        assert!(!text.starts_with("/usr"), "{text}");
        assert!(!text.starts_with("/opt"), "{text}");
        assert!(!text.contains("Program Files"), "{text}");
    }

    #[test]
    fn a_binary_in_the_cargo_target_tree_is_not_an_install() {
        assert!(in_a_build_tree(Path::new(
            "/home/x/prj/rufa/target/release/rufa"
        )));
        assert!(in_a_build_tree(Path::new(
            "/home/x/prj/rufa/target/debug/rufa"
        )));
        assert!(!in_a_build_tree(Path::new("/home/x/.local/bin/rufa")));
        assert!(in_a_build_tree(Path::new(
            "/home/x/prj/rufa/target/debug/deps/rufa-1a2b"
        )));
        assert!(!in_a_build_tree(Path::new("/home/x/Downloads/rufa")));
        assert!(!in_a_build_tree(Path::new("/home/x/deps/rufa")));
    }

    #[test]
    fn an_install_copies_the_build_and_leaves_a_receipt() {
        let root = scratch("install");
        let source = root.join("downloaded");
        std::fs::write(&source, b"the build").unwrap();
        let target_dir = root.join("bin");
        let config = root.join("config");

        let placed = install(&source, &target_dir, &config).unwrap();
        assert_eq!(placed, target_dir.join(exe_name()));
        assert_eq!(std::fs::read(&placed).unwrap(), b"the build");
        assert_eq!(recorded(&config).as_deref(), Some(placed.as_path()));
        assert!(running_installed(&placed, &config));
        assert!(!running_installed(&source, &config));
    }

    #[test]
    fn a_second_install_replaces_the_first_and_leaves_no_leftovers() {
        let root = scratch("replace");
        let first = root.join("first");
        std::fs::write(&first, b"old build").unwrap();
        let second = root.join("second");
        std::fs::write(&second, b"new build").unwrap();
        let target_dir = root.join("bin");
        let config = root.join("config");

        install(&first, &target_dir, &config).unwrap();
        let placed = install(&second, &target_dir, &config).unwrap();
        assert_eq!(std::fs::read(&placed).unwrap(), b"new build");
        let left: Vec<_> = std::fs::read_dir(&target_dir)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        assert_eq!(left, vec![exe_name()], "{left:?}");
    }

    #[test]
    fn the_desktop_entry_names_the_installed_path_and_the_window_class() {
        let root = scratch("desktop");
        let share = root.join("share");
        let exe = root.join("bin").join(exe_name());
        write_desktop_entry(&share, &exe).unwrap();
        let entry = std::fs::read_to_string(share.join("applications").join(DESKTOP)).unwrap();
        assert!(
            entry.contains(&format!("Exec={} %f", exe.display())),
            "{entry}"
        );
        assert!(
            entry.contains(&format!("StartupWMClass={APP_ID}")),
            "{entry}"
        );
        assert!(entry.contains("Icon=rufa"), "{entry}");
        let icon = share.join("icons/hicolor/512x512/apps/rufa.png");
        assert_eq!(std::fs::read(&icon).unwrap(), ICON);
    }

    #[test]
    fn installing_over_the_running_build_changes_nothing() {
        let root = scratch("same");
        let target_dir = root.join("bin");
        std::fs::create_dir_all(&target_dir).unwrap();
        let exe = target_dir.join(exe_name());
        std::fs::write(&exe, b"the build").unwrap();
        let config = root.join("config");
        assert_eq!(install(&exe, &target_dir, &config).unwrap(), exe);
        assert_eq!(std::fs::read(&exe).unwrap(), b"the build");
    }
}
