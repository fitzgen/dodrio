//! Color utility functions.

use std::f64;

/// An RGB color.
#[derive(Copy, Clone)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// An HSV color.
#[derive(Copy, Clone)]
pub struct Hsv {
    pub h: f64,
    pub s: f64,
    pub v: f64,
}

/// Convert an RGB color into HSV.
impl From<Rgb> for Hsv {
    fn from(Rgb { r, g, b }: Rgb) -> Hsv {
        let r = r as f64 / 255.0;
        let g = g as f64 / 255.0;
        let b = b as f64 / 255.0;
        let max = f64::max(f64::max(r, g), b);
        let min = f64::min(f64::min(r, g), b);
        let v = max;
        let diff = v - min;
        let mut h = 0.0;
        let mut s = 0.0;
        if diff != 0.0 {
            s = diff / v;
            let diffc = |c| (v - c) / 6.0 / diff + 1.0 / 2.0;
            let rr = diffc(r);
            let gg = diffc(g);
            let bb = diffc(b);
            if r == v {
                h = bb - gg;
            } else if g == v {
                h = (1.0 / 3.0) + rr - bb;
            } else if b == v {
                h = (2.0 / 3.0) + gg - rr;
            }
        }
        Hsv { h, s, v }
    }
}

/// Convert an HSV color into RGB.
impl From<Hsv> for Rgb {
    fn from(Hsv { h, s, v }: Hsv) -> Rgb {
        let i = (h * 6.0).floor();
        let f = h * 6.0 - i;
        let p = v * (1.0 - s);
        let q = v * (1.0 - f * s);
        let t = v * (1.0 - (1.0 - f) * s);
        let r;
        let g;
        let b;
        match i as u64 % 6 {
            0 => {
                r = v;
                g = t;
                b = p;
            }
            1 => {
                r = q;
                g = v;
                b = p;
            }
            2 => {
                r = p;
                g = v;
                b = t;
            }
            3 => {
                r = p;
                g = q;
                b = v;
            }
            4 => {
                r = t;
                g = p;
                b = v;
            }
            5 => {
                r = v;
                g = p;
                b = q;
            }
            _ => wasm_bindgen::throw_str("impossible"),
        }
        let r = (r * 255.0).round() as u8;
        let g = (g * 255.0).round() as u8;
        let b = (b * 255.0).round() as u8;
        Rgb { r, g, b }
    }
}

/// Linearly interpolate between two values.
fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

/// Clamp the `x` value to within the range `lo..hi`.
fn clamp(lo: f64, hi: f64, x: f64) -> f64 {
    if x < lo {
        lo
    } else if x > hi {
        hi
    } else {
        x
    }
}

/// Linearly interpolate two HSV colors and convert the result into RGB.
fn lerp_hsv_to_rgb(a: Hsv, b: Hsv, t: f64) -> Rgb {
    let h = lerp(a.h, b.h, t);
    let s = lerp(a.s, b.s, t);
    let v = lerp(a.v, b.v, t);
    let hsv = Hsv { h, s, v };
    hsv.into()
}

thread_local! {
    static PALETTES: [(Hsv, Hsv); 3] = [
        (
            (Rgb { r: 0xfe, g: 0xf6, b: 0xdf }).into(),
            (Rgb { r: 0x64, g: 0xc3, b: 0xc8 }).into(),
        ),
        (
            (Rgb { r: 0xfc, g: 0xcf, b: 0x74 }).into(),
            (Rgb { r: 0x41, g: 0x65, b: 0x7f }).into(),
        ),
        (
            (Rgb { r: 0xd9, g: 0xe0, b: 0xe8 }).into(),
            (Rgb { r: 0x94, g: 0x5d, b: 0x72 }).into(),
        ),
    ];
}

/// Get a color from our color palette that is linearly interpolated based on
/// the `t` elapsed time value.
///
/// The `which` function allows callers to select whether they want the
/// foreground or background color.
pub fn get_interpolated_color<F>(mut which: F, t: f64) -> Rgb
where
    F: FnMut((Hsv, Hsv)) -> Hsv,
{
    PALETTES.with(|palettes| {
        let t = t * (palettes.len() as f64);
        let prev_index = t.floor() as usize % palettes.len();
        let next_index = (prev_index + 1) % palettes.len();
        let t = clamp(-0.5, 0.5, (t % 1.0 - 0.5) * 5.0) + 0.5;
        let a = which(palettes[prev_index]);
        let b = which(palettes[next_index]);
        lerp_hsv_to_rgb(a, b, t)
    })
}
