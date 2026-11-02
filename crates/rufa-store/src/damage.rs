use std::path::Path;

const FILE: &str = "damage.txt";

/// What a volley is made of: EM, thermal, kinetic and explosive shares.
#[derive(Debug, Clone, PartialEq)]
pub struct DamagePattern {
    pub name: String,
    pub parts: [f64; 4],
}

impl DamagePattern {
    /// The parts as fractions of one volley. An all zero pattern reads as an
    /// even split, so EHP never divides by nothing.
    pub fn shares(&self) -> [f64; 4] {
        let total: f64 = self.parts.iter().sum();
        if total <= 0.0 {
            return [0.25; 4];
        }
        [
            self.parts[0] / total,
            self.parts[1] / total,
            self.parts[2] / total,
            self.parts[3] / total,
        ]
    }

    fn parse_line(line: &str) -> Option<DamagePattern> {
        let mut fields = line.split('|');
        let name = fields.next()?.trim();
        if name.is_empty() {
            return None;
        }
        let values: Vec<f64> = fields
            .next()?
            .split_whitespace()
            .filter_map(|v| v.parse().ok())
            .filter(|v: &f64| v.is_finite() && *v >= 0.0)
            .collect();
        if values.len() != 4 || values.iter().sum::<f64>() <= 0.0 {
            return None;
        }
        Some(DamagePattern {
            name: name.to_string(),
            parts: [values[0], values[1], values[2], values[3]],
        })
    }

    fn to_line(&self) -> String {
        format!(
            "{} | {:.4} {:.4} {:.4} {:.4}",
            self.name, self.parts[0], self.parts[1], self.parts[2], self.parts[3]
        )
    }
}

pub fn builtin() -> Vec<DamagePattern> {
    let p = |name: &str, parts: [f64; 4]| DamagePattern {
        name: name.to_string(),
        parts,
    };
    vec![
        p("Uniform", [0.25, 0.25, 0.25, 0.25]),
        p("EM", [1.0, 0.0, 0.0, 0.0]),
        p("Thermal", [0.0, 1.0, 0.0, 0.0]),
        p("Kinetic", [0.0, 0.0, 1.0, 0.0]),
        p("Explosive", [0.0, 0.0, 0.0, 1.0]),
    ]
}

pub fn load(dir: &Path) -> Vec<DamagePattern> {
    let mut out = builtin();
    if let Ok(text) = std::fs::read_to_string(dir.join(FILE)) {
        for pattern in text.lines().filter_map(DamagePattern::parse_line) {
            if out.iter().any(|p| p.name == pattern.name) {
                continue;
            }
            out.push(pattern);
        }
    }
    out
}

pub fn save(dir: &Path, patterns: &[DamagePattern]) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let builtin = builtin();
    let text: String = patterns
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
        let p = DamagePattern {
            name: "Angel".to_string(),
            parts: [0.0, 0.21, 0.37, 0.42],
        };
        assert_eq!(DamagePattern::parse_line(&p.to_line()), Some(p));
    }

    #[test]
    fn a_line_without_four_positive_numbers_is_dropped() {
        assert!(DamagePattern::parse_line("short | 1 2 3").is_none());
        assert!(DamagePattern::parse_line("empty | 0 0 0 0").is_none());
        assert!(DamagePattern::parse_line(" | 1 1 1 1").is_none());
    }

    #[test]
    fn a_saved_pattern_comes_back_beside_the_builtin_ones() {
        let dir = std::env::temp_dir().join(format!("rufa-damage-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mine = DamagePattern {
            name: "Angel".to_string(),
            parts: [0.0, 0.21, 0.37, 0.42],
        };
        let mut all = builtin();
        all.push(mine.clone());
        save(&dir, &all).unwrap();
        let back = load(&dir);
        assert_eq!(back.len(), builtin().len() + 1);
        assert_eq!(back.last(), Some(&mine));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn shares_add_up_to_one() {
        let p = DamagePattern {
            name: "Lopsided".to_string(),
            parts: [3.0, 1.0, 0.0, 0.0],
        };
        let s = p.shares();
        assert!((s.iter().sum::<f64>() - 1.0).abs() < 1e-9);
        assert!((s[0] - 0.75).abs() < 1e-9);
    }
}
