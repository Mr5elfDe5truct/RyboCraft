//! The RyboCraft mark, shared by desktop, packaging and CLI: a cube in printed layers.
type P = (f32, f32);

/// One face in that SVG's units: its outline, and its linear gradient from one point and
/// colour to another.
struct Face {
    outline: &'static [P],
    from: (P, [u8; 3]),
    to: (P, [u8; 3]),
}

const FACES: [Face; 9] = [
    Face {
        outline: &[(512.0, 134.0), (852.0, 330.0), (512.0, 526.0), (172.0, 330.0)],
        from: ((0.0, 134.0), [110, 222, 208]),
        to: ((0.0, 526.0), [60, 196, 184]),
    },
    Face {
        outline: &[(172.0, 346.0), (503.0, 537.0), (503.0, 628.5), (172.0, 437.5)],
        from: ((0.0, 346.0), [30, 178, 168]),
        to: ((0.0, 628.5), [10, 128, 122]),
    },
    Face {
        outline: &[(521.0, 537.0), (852.0, 346.0), (852.0, 437.5), (521.0, 628.5)],
        from: ((0.0, 346.0), [252, 140, 56]),
        to: ((0.0, 628.5), [226, 96, 24]),
    },
    Face {
        outline: &[(172.0, 453.5), (503.0, 644.5), (503.0, 736.0), (172.0, 545.0)],
        from: ((0.0, 453.5), [30, 178, 168]),
        to: ((0.0, 736.0), [10, 128, 122]),
    },
    Face {
        outline: &[(521.0, 644.5), (852.0, 453.5), (852.0, 545.0), (521.0, 736.0)],
        from: ((0.0, 453.5), [252, 140, 56]),
        to: ((0.0, 736.0), [226, 96, 24]),
    },
    Face {
        outline: &[(172.0, 561.0), (503.0, 752.0), (503.0, 843.5), (172.0, 652.5)],
        from: ((0.0, 561.0), [30, 178, 168]),
        to: ((0.0, 843.5), [10, 128, 122]),
    },
    Face {
        outline: &[(521.0, 752.0), (852.0, 561.0), (852.0, 652.5), (521.0, 843.5)],
        from: ((0.0, 561.0), [252, 140, 56]),
        to: ((0.0, 843.5), [226, 96, 24]),
    },
    Face {
        outline: &[(172.0, 668.5), (503.0, 859.5), (503.0, 951.0), (172.0, 760.0)],
        from: ((0.0, 668.5), [30, 178, 168]),
        to: ((0.0, 951.0), [10, 128, 122]),
    },
    Face {
        outline: &[(521.0, 859.5), (852.0, 668.5), (852.0, 760.0), (521.0, 951.0)],
        from: ((0.0, 668.5), [252, 140, 56]),
        to: ((0.0, 951.0), [226, 96, 24]),
    },
];

/// The mark's extent in SVG units: left, top, width, height.
const BOUNDS: (f32, f32, f32, f32) = (172.0, 134.0, 680.0, 817.0);

/// This block is the icon image data
pub fn rgba(size: usize) -> Result<Vec<u8>, String> {
    if !(1..=2048).contains(&size) {
        return Err("icon size must be 1..=2048".into());
    }
    // Coverage buffer at 4x4 samples per pixel (2x2 for big sizes), averaged down at the end.
    let ss = if size > 512 { 2 } else { 4 };
    let n = size * ss;
    let mut buf = vec![[0u8; 4]; n * n];
    let (bx, by, bw, bh) = BOUNDS;
    let s = n as f32 / bh; // SVG units -> samples: the mark fills the height
    let (ox, oy) = ((n as f32 - bw * s) / 2.0 - bx * s, -by * s); // and is centred sideways
    // Fill each face's outline (even-odd spans per row, so concave outlines work) with its gradient.
    for face in &FACES {
        let v: Vec<P> = face.outline.iter().map(|p| (ox + p.0 * s, oy + p.1 * s)).collect();
        let (((fx, fy), c0), ((tx, ty), c1)) = (face.from, face.to);
        let (gx, gy) = (tx - fx, ty - fy);
        let g2 = gx * gx + gy * gy;
        let (top, bottom) = v.iter().fold((f32::MAX, f32::MIN), |(a, b), p| (a.min(p.1), b.max(p.1)));
        let mut xs = Vec::with_capacity(v.len());
        for y in ((top - 0.5).ceil() as i32).max(0)..=((bottom - 0.5).floor() as i32).min(n as i32 - 1) {
            let cy = y as f32 + 0.5;
            xs.clear();
            for i in 0..v.len() {
                let (a, b) = (v[i], v[(i + 1) % v.len()]);
                if (a.1 <= cy) != (b.1 <= cy) {
                    xs.push(a.0 + (cy - a.1) / (b.1 - a.1) * (b.0 - a.0));
                }
            }
            xs.sort_by(f32::total_cmp);
            for &[from, to] in xs.as_chunks::<2>().0 {
                for x in ((from - 0.5).ceil() as i32).max(0)..=((to - 0.5).floor() as i32).min(n as i32 - 1) {
                    // The sample's place along the gradient, in SVG units.
                    let (px, py) = ((x as f32 + 0.5 - ox) / s, (cy - oy) / s);
                    let t = (((px - fx) * gx + (py - fy) * gy) / g2).clamp(0.0, 1.0);
                    let mix = |k: usize| (f32::from(c0[k]) + (f32::from(c1[k]) - f32::from(c0[k])) * t).round() as u8;
                    buf[y as usize * n + x as usize] = [mix(0), mix(1), mix(2), 255];
                }
            }
        }
    }
    // Average each ss x ss block down to one pixel (straight alpha = covered share).
    let mut rgba = Vec::with_capacity(size * size * 4);
    for y in 0..size {
        for x in 0..size {
            let (mut sum, mut cnt) = ([0u32; 3], 0u32);
            for j in 0..ss {
                for i in 0..ss {
                    let p = buf[(y * ss + j) * n + x * ss + i];
                    if p[3] != 0 {
                        cnt += 1;
                        for k in 0..3 {
                            sum[k] += u32::from(p[k]);
                        }
                    }
                }
            }
            if cnt == 0 {
                rgba.extend_from_slice(&[0, 0, 0, 0]);
            } else {
                let a = 255 * cnt / (ss * ss) as u32;
                rgba.extend_from_slice(&[(sum[0] / cnt) as u8, (sum[1] / cnt) as u8, (sum[2] / cnt) as u8, a as u8]);
            }
        }
    }
    Ok(rgba)
}

#[cfg(test)]
mod tests {
    #[test]
    fn shared_mark_has_transparency_and_original_colors_at_window_and_package_sizes() {
        for size in [16, 24, 32, 48, 64, 128, 256] {
            let pixels = super::rgba(size).expect("valid size");
            assert_eq!(pixels.len(), size * size * 4);
            assert!(pixels.as_chunks::<4>().0.iter().any(|p| p[3] == 0));
            assert!(pixels.as_chunks::<4>().0.iter().any(|p| p[3] == 255 && p[2] > p[0]));
        }
        assert!(super::rgba(0).is_err());
        assert!(super::rgba(2049).is_err());
    }

    /// Borderless: no white edge lines, and the gaps between faces are see-through. At 256 px
    /// the gap between the left and right faces (SVG x 494..530) is transparent on the row
    /// through the middle of the mark, and no opaque pixel is white.
    #[test]
    fn shared_mark_is_borderless() {
        let size = 256;
        let pixels = super::rgba(size).expect("valid size");
        let px = |x: usize, y: usize| pixels.as_chunks::<4>().0[y * size + x];
        let s = size as f32 / 904.436;
        let gap_x = ((size as f32 - 791.188 * s) / 2.0 + (512.0 - 116.406) * s) as usize;
        let row = ((750.0 - 53.364) * s) as usize;
        assert_eq!(px(gap_x, row)[3], 0, "the seam between the left and right faces is a gap");
        assert!(px(gap_x - 8, row)[3] == 255 && px(gap_x + 8, row)[3] == 255, "faces on both sides of the gap");
        assert!(pixels.as_chunks::<4>().0.iter().all(|p| p[3] == 0 || p[0] < 200), "no white edges");
    }
}
