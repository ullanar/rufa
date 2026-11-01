use std::collections::BTreeMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const FILE: &str = "prices.txt";
pub const MAX_AGE: u64 = 24 * 3600;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cached {
    pub price: f64,
    pub fetched: u64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct PriceCache {
    pub entries: BTreeMap<i32, Cached>,
}

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

impl PriceCache {
    pub fn load(dir: &Path) -> PriceCache {
        let mut cache = PriceCache::default();
        let Ok(text) = std::fs::read_to_string(dir.join(FILE)) else {
            return cache;
        };
        for line in text.lines() {
            let mut parts = line.split_whitespace();
            if let (Some(id), Some(price), Some(time)) = (parts.next(), parts.next(), parts.next())
                && let (Ok(id), Ok(price), Ok(fetched)) = (id.parse(), price.parse(), time.parse())
            {
                cache.entries.insert(id, Cached { price, fetched });
            }
        }
        cache
    }

    pub fn save(&self, dir: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(dir)?;
        let mut text = String::new();
        for (id, c) in &self.entries {
            text.push_str(&format!("{id} {:.2} {}\n", c.price, c.fetched));
        }
        std::fs::write(dir.join(FILE), text)
    }

    pub fn get(&self, type_id: i32) -> Option<f64> {
        self.entries.get(&type_id).map(|c| c.price)
    }

    pub fn stale(&self, wanted: impl IntoIterator<Item = i32>) -> Vec<i32> {
        let now = now();
        let mut out: Vec<i32> = wanted
            .into_iter()
            .filter(|id| {
                self.entries
                    .get(id)
                    .is_none_or(|c| now.saturating_sub(c.fetched) > MAX_AGE)
            })
            .collect();
        out.sort();
        out.dedup();
        out
    }

    pub fn insert_all(&mut self, prices: &BTreeMap<i32, f64>) {
        let fetched = now();
        for (id, price) in prices {
            self.entries.insert(
                *id,
                Cached {
                    price: *price,
                    fetched,
                },
            );
        }
    }
}
