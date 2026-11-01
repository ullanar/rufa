pub fn num(value: f64, decimals: usize) -> String {
    if !value.is_finite() {
        return "-".to_string();
    }
    let negative = value < 0.0;
    let s = format!("{:.*}", decimals, value.abs());
    let (int, frac) = s
        .split_once('.')
        .map(|(a, b)| (a, Some(b)))
        .unwrap_or((&s, None));
    let mut out = String::new();
    for (i, c) in int.chars().enumerate() {
        if i > 0 && (int.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if let Some(frac) = frac {
        out.push('.');
        out.push_str(frac);
    }
    if negative {
        out.insert(0, '-');
    }
    out
}

pub fn short(value: f64) -> String {
    let abs = value.abs();
    if abs >= 1e9 {
        format!("{:.2}B", value / 1e9)
    } else if abs >= 1e6 {
        format!("{:.2}M", value / 1e6)
    } else if abs >= 1e4 {
        format!("{:.1}k", value / 1e3)
    } else {
        num(value, if abs >= 100.0 { 0 } else { 1 })
    }
}

pub fn resist(resonance: f64) -> String {
    format!("{:.0}%", (1.0 - resonance) * 100.0)
}

pub fn seconds(value: f64) -> String {
    if value < 0.0 {
        return "-".to_string();
    }
    if value < 60.0 {
        return format!("{value:.1}s");
    }
    let total = value.round() as u64;
    let (h, m, s) = (total / 3600, (total % 3600) / 60, total % 60);
    if h > 0 {
        format!("{h}h {m}m {s}s")
    } else {
        format!("{m}m {s}s")
    }
}

pub fn distance(meters: f64) -> String {
    if meters >= 1000.0 {
        format!("{} km", num(meters / 1000.0, 1))
    } else {
        format!("{} m", num(meters, 0))
    }
}
