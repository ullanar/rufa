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
                _ => {}
            }
        }
        s
    }

    pub fn save(&self, dir: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(dir)?;
        let text = format!(
            "skill_level = {}\nfactor_reload = {}\ncharacter = {}\nbrowser_ratio = {:.3}\nfit_ratio = {:.3}\nesi_client_id = {}\nui_scale = {:.3}\nskipped_version = {}\n",
            self.skill_level,
            self.factor_reload,
            self.character,
            self.browser_ratio,
            self.fit_ratio,
            self.esi_client_id,
            self.ui_scale,
            self.skipped_version
        );
        std::fs::write(dir.join(FILE), text)
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
        };
        written.save(&dir).unwrap();
        assert_eq!(Settings::load(&dir), written);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
