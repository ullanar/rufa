use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const CHARACTER_EXT: &str = "char";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Character {
    pub name: String,
    pub skills: BTreeMap<i32, u8>,
    pub path: Option<PathBuf>,
}

impl Character {
    pub fn new(name: impl Into<String>) -> Character {
        Character {
            name: name.into(),
            skills: BTreeMap::new(),
            path: None,
        }
    }

    pub fn parse(text: &str) -> Character {
        let mut c = Character::new("");
        for line in text.lines() {
            let line = line.trim();
            if let Some(name) = line.strip_prefix("name") {
                if let Some(name) = name.trim_start().strip_prefix('=') {
                    c.name = name.trim().to_string();
                }
            } else if let Some(rest) = line.strip_prefix("skill ") {
                let mut parts = rest.split_whitespace();
                if let (Some(id), Some(level)) = (parts.next(), parts.next())
                    && let (Ok(id), Ok(level)) = (id.parse::<i32>(), level.parse::<u8>())
                {
                    c.skills.insert(id, level.min(5));
                }
            }
        }
        c
    }

    pub fn to_text(&self) -> String {
        let mut out = format!("name = {}\n", self.name);
        for (id, level) in &self.skills {
            if *level > 0 {
                out.push_str(&format!("skill {id} {level}\n"));
            }
        }
        out
    }
}

pub fn characters_dir(config: &Path) -> PathBuf {
    config.join("characters")
}

pub fn list_characters(config: &Path) -> std::io::Result<Vec<Character>> {
    let mut out = Vec::new();
    let entries = match std::fs::read_dir(characters_dir(config)) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(out),
        Err(e) => return Err(e),
    };
    for entry in entries {
        let path = entry?.path();
        if !path.extension().is_some_and(|e| e == CHARACTER_EXT) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let mut c = Character::parse(&text);
        c.path = Some(path);
        out.push(c);
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

pub fn save_character(config: &Path, character: &mut Character) -> std::io::Result<PathBuf> {
    let dir = characters_dir(config);
    std::fs::create_dir_all(&dir)?;
    let path = match &character.path {
        Some(p) => p.clone(),
        None => {
            let slug: String = character
                .name
                .chars()
                .map(|c| {
                    if c.is_alphanumeric() {
                        c.to_ascii_lowercase()
                    } else {
                        '-'
                    }
                })
                .collect();
            let slug = slug.trim_matches('-').to_string();
            let slug = if slug.is_empty() {
                "character".to_string()
            } else {
                slug
            };
            let mut path = dir.join(format!("{slug}.{CHARACTER_EXT}"));
            let mut n = 2;
            while path.exists() {
                path = dir.join(format!("{slug}-{n}.{CHARACTER_EXT}"));
                n += 1;
            }
            path
        }
    };
    std::fs::write(&path, character.to_text())?;
    character.path = Some(path.clone());
    Ok(path)
}

pub fn delete_character(character: &Character) -> std::io::Result<()> {
    match &character.path {
        Some(p) => std::fs::remove_file(p),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_text() {
        let mut c = Character::new("Test Pilot");
        c.skills.insert(3300, 5);
        c.skills.insert(3301, 0);
        c.skills.insert(3302, 3);
        let parsed = Character::parse(&c.to_text());
        assert_eq!(parsed.name, "Test Pilot");
        assert_eq!(parsed.skills.get(&3300), Some(&5));
        assert_eq!(parsed.skills.get(&3302), Some(&3));
        assert_eq!(parsed.skills.get(&3301), None);
    }
}
