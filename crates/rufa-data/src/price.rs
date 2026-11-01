use std::collections::BTreeMap;
use std::time::Duration;

const AGGREGATES: &str = "https://market.fuzzwork.co.uk/aggregates/";
pub const REGION_THE_FORGE: i32 = 10000002;
const BATCH: usize = 200;

pub type Prices = BTreeMap<i32, f64>;

pub fn parse(json: &str) -> Result<Prices, String> {
    let value: serde_json::Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let object = value.as_object().ok_or("prices: not an object")?;
    let mut out = Prices::new();
    for (key, entry) in object {
        let Ok(type_id) = key.parse::<i32>() else {
            continue;
        };
        let price = entry
            .get("sell")
            .and_then(|s| s.get("percentile"))
            .and_then(number)
            .filter(|p| *p > 0.0)
            .or_else(|| {
                entry
                    .get("buy")
                    .and_then(|b| b.get("percentile"))
                    .and_then(number)
            });
        if let Some(price) = price {
            out.insert(type_id, price);
        }
    }
    Ok(out)
}

fn number(v: &serde_json::Value) -> Option<f64> {
    match v {
        serde_json::Value::String(s) => s.parse().ok(),
        serde_json::Value::Number(n) => n.as_f64(),
        _ => None,
    }
}

pub fn fetch(region: i32, types: &[i32]) -> Result<Prices, String> {
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(30)))
        .user_agent("rufa (https://github.com/EVEShipFit)")
        .build()
        .new_agent();
    let mut out = Prices::new();
    for chunk in types.chunks(BATCH) {
        let list = chunk
            .iter()
            .map(|t| t.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let url = format!("{AGGREGATES}?region={region}&types={list}");
        let text = agent
            .get(&url)
            .call()
            .map_err(|e| e.to_string())?
            .body_mut()
            .read_to_string()
            .map_err(|e| e.to_string())?;
        out.extend(parse(&text)?);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_aggregates_shape() {
        let json = r#"{"2048":{"buy":{"percentile":"351105.67"},"sell":{"percentile":"367497.43","min":"367200.0"}},
                       "34":{"buy":{"percentile":"4.1"},"sell":{"percentile":"0"}},"x":{}}"#;
        let prices = parse(json).unwrap();
        assert_eq!(prices.len(), 2);
        assert!((prices[&2048] - 367497.43).abs() < 1e-6);
        assert!(
            (prices[&34] - 4.1).abs() < 1e-9,
            "falls back to buy when sell is zero"
        );
    }
}
