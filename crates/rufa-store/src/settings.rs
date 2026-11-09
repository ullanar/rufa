use std::path::Path;

const FILE: &str = "settings.conf";

#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    pub skill_level: u8,
    pub factor_reload: bool,
    pub character: String,
    pub browser_ratio: f32,
    pub fit_ratio: f32,
    pub esi_client_id: String,
    pub ui_scale: f32,
    pub skipped_version: String,
    /// The window as it was left. Wayland keeps its coordinates from the
    /// client, so the size can be known while the position is not.
    pub window_size: Option<(f32, f32)>,
    pub window_at: Option<(f32, f32)>,
    /// Graph toggles as `key=value` words; the app reads and writes them.
    pub graph_options: String,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            skill_level: 5,
            factor_reload: false,
            character: String::new(),
            browser_ratio: 0.2,
            fit_ratio: 0.55,
            esi_client_id: String::new(),
            ui_scale: 1.0,
            skipped_version: String::new(),
            window_size: None,
            window_at: None,
            graph_options: String::new(),
        }
    }
}

impl Settings {
    pub fn load(dir: &Path) -> Settings {
        let mut s = Settings::default();
        let Ok(text) = std::fs::read_to_string(dir.join(FILE)) else {
            return s;
        };
        for line in text.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let value = value.trim();
            match key.trim() {
                "skill_level" => s.skill_level = value.parse().unwrap_or(s.skill_level).min(5),
                "factor_reload" => s.factor_reload = value == "true",
                "character" => s.character = value.to_string(),
                "browser_ratio" => s.browser_ratio = value.parse().unwrap_or(s.browser_ratio),
                "fit_ratio" => s.fit_ratio = value.parse().unwrap_or(s.fit_ratio),
                "esi_client_id" => s.esi_client_id = value.to_string(),
                "ui_scale" => s.ui_scale = value.parse().unwrap_or(1.0),
                "skipped_version" => s.skipped_version = value.to_string(),
                "window_size" => {
                    s.window_size = parse_pair(value).filter(|(w, h)| *w >= 100.0 && *h >= 100.0)
                }
                "window_at" => s.window_at = parse_pair(value),
                "graph_options" => s.graph_options = value.to_string(),
                _ => {}
            }
        }
        s
    }

    pub fn save(&self, dir: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(dir)?;
        let pair = |v: Option<(f32, f32)>| match v {
            Some((a, b)) => format!("{a:.0} {b:.0}"),
            None => String::new(),
        };
        let text = format!(
            "skill_level = {}\nfactor_reload = {}\ncharacter = {}\nbrowser_ratio = {:.3}\nfit_ratio = {:.3}\nesi_client_id = {}\nui_scale = {:.3}\nskipped_version = {}\nwindow_size = {}\nwindow_at = {}\ngraph_options = {}\n",
            self.skill_level,
            self.factor_reload,
            self.character,
            self.browser_ratio,
            self.fit_ratio,
            self.esi_client_id,
            self.ui_scale,
            self.skipped_version,
            pair(self.window_size),
            pair(self.window_at),
            self.graph_options
        );
        std::fs::write(dir.join(FILE), text)
    }
}

/// Two finite numbers on one line, or nothing.
fn parse_pair(value: &str) -> Option<(f32, f32)> {
    let parts: Vec<f32> = value
        .split_whitespace()
        .filter_map(|v| v.parse().ok())
        .filter(|v: &f32| v.is_finite())
        .collect();
    match parts[..] {
        [a, b] => Some((a, b)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_field_survives_a_save_and_a_load() {
        let dir = std::env::temp_dir().join(format!("rufa-settings-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let written = Settings {
            skill_level: 4,
            factor_reload: true,
            character: "Pilot".to_string(),
            browser_ratio: 0.25,
            fit_ratio: 0.6,
            esi_client_id: "abc".to_string(),
            ui_scale: 1.2,
            skipped_version: "v0.4.0".to_string(),
            window_size: Some((1280.0, 720.0)),
            window_at: Some((40.0, -12.0)),
            graph_options: "resists=1 projected=0".to_string(),
        };
        written.save(&dir).unwrap();
        assert_eq!(Settings::load(&dir), written);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_pair_needs_exactly_two_numbers() {
        assert_eq!(parse_pair("1280 720"), Some((1280.0, 720.0)));
        assert_eq!(parse_pair("40 -12"), Some((40.0, -12.0)));
        assert_eq!(parse_pair(""), None);
        assert_eq!(parse_pair("1280 720 40"), None);
    }

    #[test]
    fn a_window_too_small_to_use_is_dropped_on_load() {
        let dir = std::env::temp_dir().join(format!("rufa-window-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(FILE), "window_size = 10 10\nwindow_at = 5 5\n").unwrap();
        let s = Settings::load(&dir);
        assert_eq!(s.window_size, None);
        assert_eq!(s.window_at, Some((5.0, 5.0)));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
