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
                _ => {}
            }
        }
        s
    }

    pub fn save(&self, dir: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(dir)?;
        let text = format!(
            "skill_level = {}\nfactor_reload = {}\ncharacter = {}\nbrowser_ratio = {:.3}\nfit_ratio = {:.3}\nesi_client_id = {}\n",
            self.skill_level,
            self.factor_reload,
            self.character,
            self.browser_ratio,
            self.fit_ratio,
            self.esi_client_id
        );
        std::fs::write(dir.join(FILE), text)
    }
}
