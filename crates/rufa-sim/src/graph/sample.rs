//! Even samples over a range, with midpoints added where the curve bends.

/// `base` intervals between `lo` and `hi`, then up to `2^depth - 1` extra
/// points inside every interval whose ends differ in value.
pub fn sample(
    lo: f64,
    hi: f64,
    base: usize,
    depth: u32,
    mut f: impl FnMut(f64) -> f64,
) -> Vec<(f64, f64)> {
    let (lo, hi) = if lo <= hi { (lo, hi) } else { (hi, lo) };
    let step = (hi - lo) / base.max(1) as f64;
    if step == 0.0 || !step.is_finite() {
        return vec![(lo, f(lo))];
    }
    let mut out = Vec::with_capacity(base + 1);
    let mut prev = (lo, f(lo));
    out.push(prev);
    for i in 1..=base {
        let x = if i == base { hi } else { lo + step * i as f64 };
        let next = (x, f(x));
        refine(prev, next, depth, &mut f, &mut out);
        out.push(next);
        prev = next;
    }
    out
}

fn refine(
    left: (f64, f64),
    right: (f64, f64),
    depth: u32,
    f: &mut impl FnMut(f64) -> f64,
    out: &mut Vec<(f64, f64)>,
) {
    if depth == 0 || left.1 == right.1 {
        return;
    }
    let x = (left.0 + right.0) / 2.0;
    let mid = (x, f(x));
    refine(left, mid, depth - 1, f, out);
    out.push(mid);
    refine(mid, right, depth - 1, f, out);
}

/// Change points to a polyline of horizontal runs and vertical jumps between
/// `lo` and `hi`. Every value holds until the next point.
pub fn steps(points: &[(f64, f64)], lo: f64, hi: f64) -> Vec<(f64, f64)> {
    let mut out = Vec::new();
    let mut value = 0.0;
    let mut started = false;
    for &(t, v) in points {
        if t < lo {
            value = v;
            continue;
        }
        if !started {
            out.push((lo, value));
            started = true;
        }
        if t >= hi {
            break;
        }
        if v != value {
            out.push((t, value));
            out.push((t, v));
            value = v;
        }
    }
    if !started {
        out.push((lo, value));
    }
    out.push((hi, value));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_flat_curve_gets_no_extra_points() {
        let pts = sample(0.0, 10.0, 10, 2, |_| 1.0);
        assert_eq!(pts.len(), 11);
        assert_eq!(pts.first(), Some(&(0.0, 1.0)));
        assert_eq!(pts.last(), Some(&(10.0, 1.0)));
    }

    #[test]
    fn a_bend_gets_three_extra_points_at_depth_two() {
        let pts = sample(0.0, 2.0, 2, 2, |x| if x < 1.0 { 0.0 } else { x });
        let xs: Vec<f64> = pts.iter().map(|p| p.0).collect();
        assert_eq!(xs, vec![0.0, 0.5, 0.75, 1.0, 1.25, 1.5, 1.75, 2.0]);
        assert!(pts.windows(2).all(|w| w[0].0 <= w[1].0));
    }

    #[test]
    fn an_empty_range_is_one_point() {
        assert_eq!(sample(3.0, 3.0, 50, 2, |x| x * 2.0), vec![(3.0, 6.0)]);
    }

    #[test]
    fn steps_hold_the_last_value_before_the_window() {
        let pts = [(1.0, 5.0), (4.0, 7.0), (9.0, 0.0)];
        assert_eq!(
            steps(&pts, 2.0, 6.0),
            vec![(2.0, 5.0), (4.0, 5.0), (4.0, 7.0), (6.0, 7.0)]
        );
        assert_eq!(steps(&[], 0.0, 3.0), vec![(0.0, 0.0), (3.0, 0.0)]);
    }
}
