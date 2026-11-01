use std::path::Path;

const FILE: &str = "implant-sets.txt";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImplantSet {
    pub name: String,
    pub implants: Vec<i32>,
}

impl ImplantSet {
    fn parse_line(line: &str) -> Option<ImplantSet> {
        let (name, ids) = line.split_once('|')?;
        let name = name.trim();
        if name.is_empty() {
            return None;
        }
        let implants: Vec<i32> = ids
            .split_whitespace()
            .filter_map(|v| v.parse().ok())
            .collect();
        Some(ImplantSet {
            name: name.to_string(),
            implants,
        })
    }

    fn to_line(&self) -> String {
        let ids: Vec<String> = self.implants.iter().map(|i| i.to_string()).collect();
        format!("{} | {}", self.name, ids.join(" "))
    }
}

pub fn load(dir: &Path) -> Vec<ImplantSet> {
    std::fs::read_to_string(dir.join(FILE))
        .map(|text| text.lines().filter_map(ImplantSet::parse_line).collect())
        .unwrap_or_default()
}

pub fn save(dir: &Path, sets: &[ImplantSet]) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let text: String = sets.iter().map(|s| s.to_line() + "\n").collect();
    std::fs::write(dir.join(FILE), text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_round_trip() {
        let set = ImplantSet {
            name: "Snakes".to_string(),
            implants: vec![19540, 19551, 19553],
        };
        assert_eq!(ImplantSet::parse_line(&set.to_line()), Some(set));
        assert!(ImplantSet::parse_line("| 1 2").is_none());
    }
}
