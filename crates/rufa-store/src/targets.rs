use std::path::Path;

const FILE: &str = "targets.txt";

#[derive(Debug, Clone, PartialEq)]
pub struct TargetProfile {
    pub name: String,
    pub resists: [f64; 4],
    pub signature: f64,
    pub velocity: f64,
}

impl TargetProfile {
    pub const IDEAL_NAME: &str = "Ideal target";

    pub fn ideal() -> TargetProfile {
        TargetProfile {
            name: TargetProfile::IDEAL_NAME.to_string(),
            resists: [0.0; 4],
            signature: 0.0,
            velocity: 0.0,
        }
    }

    fn parse_line(line: &str) -> Option<TargetProfile> {
        let mut parts = line.split('|');
        let name = parts.next()?.trim();
        if name.is_empty() {
            return None;
        }
        let resists: Vec<f64> = parts
            .next()?
            .split_whitespace()
            .filter_map(|v| v.parse().ok())
            .collect();
        if resists.len() != 4 {
            return None;
        }
        let rest: Vec<f64> = parts
            .next()
            .unwrap_or("0 0")
            .split_whitespace()
            .filter_map(|v| v.parse().ok())
            .collect();
        Some(TargetProfile {
            name: name.to_string(),
            resists: [resists[0], resists[1], resists[2], resists[3]],
            signature: rest.first().copied().unwrap_or(0.0),
            velocity: rest.get(1).copied().unwrap_or(0.0),
        })
    }

    fn to_line(&self) -> String {
        format!(
            "{} | {:.4} {:.4} {:.4} {:.4} | {:.1} {:.1}",
            self.name,
            self.resists[0],
            self.resists[1],
            self.resists[2],
            self.resists[3],
            self.signature,
            self.velocity
        )
    }
}

pub fn builtin() -> Vec<TargetProfile> {
    let p = |name: &str, r: [f64; 4], sig: f64, vel: f64| TargetProfile {
        name: name.to_string(),
        resists: r,
        signature: sig,
        velocity: vel,
    };
    vec![
        TargetProfile::ideal(),
        p("Shield T2 frigate", [0.0, 0.2, 0.4, 0.5], 40.0, 1200.0),
        p("Armor T2 frigate", [0.5, 0.35, 0.25, 0.1], 38.0, 900.0),
        p("Shield T2 cruiser", [0.0, 0.2, 0.4, 0.5], 130.0, 450.0),
        p("Armor T2 cruiser", [0.5, 0.35, 0.25, 0.1], 120.0, 300.0),
        p("Battleship", [0.3, 0.3, 0.3, 0.3], 400.0, 120.0),
        p("Structure", [0.2, 0.2, 0.2, 0.2], 0.0, 0.0),
    ]
}

pub fn load(dir: &Path) -> Vec<TargetProfile> {
    let mut out = builtin();
    if let Ok(text) = std::fs::read_to_string(dir.join(FILE)) {
        out.extend(text.lines().filter_map(TargetProfile::parse_line));
    }
    out
}

pub fn save(dir: &Path, profiles: &[TargetProfile]) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let builtin = builtin();
    let text: String = profiles
        .iter()
        .filter(|p| !builtin.iter().any(|b| b.name == p.name))
        .map(|p| p.to_line() + "\n")
        .collect();
    std::fs::write(dir.join(FILE), text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_round_trip() {
        let p = TargetProfile {
            name: "Test".to_string(),
            resists: [0.1, 0.2, 0.3, 0.4],
            signature: 35.0,
            velocity: 1000.0,
        };
        assert_eq!(TargetProfile::parse_line(&p.to_line()), Some(p));
        assert!(TargetProfile::parse_line("bad | 1 2").is_none());
    }
}
