use std::path::{Path, PathBuf};
use std::time::Duration;

const SERVER: &str = "https://images.evetech.net/types";
pub const ICON_SIZE: u32 = 32;
pub const RENDER_SIZE: u32 = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Kind {
    Icon,
    Render,
}

pub fn path(dir: &Path, kind: Kind, type_id: i32) -> PathBuf {
    let name = match kind {
        Kind::Icon => format!("{type_id}.png"),
        Kind::Render => format!("{type_id}-render.jpg"),
    };
    dir.join("icons").join(name)
}

fn url(kind: Kind, type_id: i32) -> String {
    match kind {
        Kind::Icon => format!("{SERVER}/{type_id}/icon?size={ICON_SIZE}"),
        Kind::Render => format!("{SERVER}/{type_id}/render?size={RENDER_SIZE}"),
    }
}

pub fn fetch(dir: &Path, wanted: &[(Kind, i32)]) -> Vec<(Kind, i32, PathBuf)> {
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(20)))
        .user_agent("rufa (https://github.com/ullanar/rufa)")
        .build()
        .new_agent();
    let _ = std::fs::create_dir_all(dir.join("icons"));
    let mut out = Vec::new();
    for (kind, type_id) in wanted {
        let target = path(dir, *kind, *type_id);
        if target.is_file() {
            out.push((*kind, *type_id, target));
            continue;
        }
        let Ok(mut response) = agent.get(url(*kind, *type_id)).call() else {
            continue;
        };
        let Ok(bytes) = response.body_mut().read_to_vec() else {
            continue;
        };
        if bytes.len() < 100 {
            continue;
        }
        if std::fs::write(&target, &bytes).is_ok() {
            out.push((*kind, *type_id, target));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_and_urls() {
        let dir = Path::new("/tmp/x");
        assert_eq!(
            path(dir, Kind::Icon, 587),
            PathBuf::from("/tmp/x/icons/587.png")
        );
        assert_eq!(
            path(dir, Kind::Render, 587),
            PathBuf::from("/tmp/x/icons/587-render.jpg")
        );
        assert_eq!(
            url(Kind::Icon, 587),
            "https://images.evetech.net/types/587/icon?size=32"
        );
    }
}
