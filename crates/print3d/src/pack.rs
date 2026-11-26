//! Placing footprints on the bed.

/// Where each footprint's lower-left corner goes, and which plate it is on. `None` for one that
/// does not fit the bed at all. Tallest first, on shelves.
pub fn arrange(sizes: &[[f64; 2]], bed: [f64; 2], gap: f64, margin: f64) -> Vec<Option<(usize, [f64; 2])>> {
    let usable = [bed[0] - 2.0 * margin, bed[1] - 2.0 * margin];
    let mut order: Vec<usize> = (0..sizes.len()).collect();
    let size = |i: usize| sizes.get(i).copied().unwrap_or([0.0; 2]);
    order.sort_by(|&a, &b| size(b)[1].total_cmp(&size(a)[1]).then(a.cmp(&b)));
    let mut out = vec![None; sizes.len()];
    // Shelves: (plate, y, height, next x).
    let mut shelves: Vec<(usize, f64, f64, f64)> = Vec::new();
    // Next free y per plate.
    let mut plates: Vec<f64> = Vec::new();
    for i in order {
        let [w, h] = size(i);
        if w > usable[0] + 1e-9 || h > usable[1] + 1e-9 {
            continue;
        }
        let (plate, x, y) = if let Some(s) = shelves.iter_mut().find(|s| h <= s.2 + 1e-9 && s.3 + w <= usable[0] + 1e-9) {
            let x = s.3;
            s.3 += w + gap;
            (s.0, x, s.1)
        } else {
            let plate = plates.iter().position(|y| y + h <= usable[1] + 1e-9).unwrap_or_else(|| {
                plates.push(0.0);
                plates.len() - 1
            });
            let y = plates.get(plate).copied().unwrap_or(0.0);
            if let Some(p) = plates.get_mut(plate) {
                *p = y + h + gap;
            }
            shelves.push((plate, y, h, w + gap));
            (plate, 0.0, y)
        };
        if let Some(o) = out.get_mut(i) {
            *o = Some((plate, [x + margin, y + margin]));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_overlaps_and_nothing_leaves_the_bed() {
        let sizes: Vec<[f64; 2]> = (0..40).map(|i| [10.0 + f64::from(i % 7) * 9.0, 8.0 + f64::from(i % 5) * 13.0]).collect();
        let placed = arrange(&sizes, [220.0, 220.0], 5.0, 5.0);
        let r: Vec<_> = placed.iter().zip(&sizes).map(|(p, s)| (p.unwrap(), *s)).collect();
        for (i, ((pa, a), sa)) in r.iter().enumerate() {
            assert!(a[0] >= 5.0 && a[1] >= 5.0 && a[0] + sa[0] <= 215.0 + 1e-9 && a[1] + sa[1] <= 215.0 + 1e-9);
            for ((pb, b), sb) in &r[i + 1..] {
                let apart = |k: usize| a[k] + sa[k] + 5.0 <= b[k] + 1e-9 || b[k] + sb[k] + 5.0 <= a[k] + 1e-9;
                assert!(pa != pb || apart(0) || apart(1));
            }
        }
        assert_eq!(arrange(&[[300.0, 10.0]], [220.0, 220.0], 5.0, 5.0), [None]);
    }
}
