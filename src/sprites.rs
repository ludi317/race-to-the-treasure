use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

const GRASS: [u8; 4] = [89, 140, 64, 255];
const GRASS_DARK: [u8; 4] = [70, 115, 50, 255];
const PATH_TAN: [u8; 4] = [225, 200, 145, 255];
const PATH_TAN_EDGE: [u8; 4] = [180, 150, 100, 255];
const KEY_RED: [u8; 4] = [200, 55, 50, 255];
const KEY_RED_DARK: [u8; 4] = [140, 30, 28, 255];
const KEY_GOLD: [u8; 4] = [235, 190, 60, 255];
const OGRE_BG: [u8; 4] = [25, 20, 15, 255];
const OGRE_FACE: [u8; 4] = [120, 160, 70, 255];
const OGRE_HAIR: [u8; 4] = [230, 220, 210, 255];
const OGRE_EYE: [u8; 4] = [30, 18, 10, 255];
const SNACK_BROWN: [u8; 4] = [145, 95, 50, 255];
const SNACK_DARK: [u8; 4] = [95, 60, 30, 255];
const CHEST_WOOD: [u8; 4] = [120, 75, 40, 255];
const CHEST_DARK: [u8; 4] = [70, 40, 18, 255];
const BLACK: [u8; 4] = [20, 14, 8, 255];
const TRANSPARENT: [u8; 4] = [0, 0, 0, 0];

fn make_image(size: u32, data: Vec<u8>) -> Image {
    Image::new(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

fn blend(a: [u8; 4], b: [u8; 4], t: f32) -> [u8; 4] {
    let t = t.clamp(0.0, 1.0);
    let lerp = |x: u8, y: u8| (x as f32 * (1.0 - t) + y as f32 * t) as u8;
    [
        lerp(a[0], b[0]),
        lerp(a[1], b[1]),
        lerp(a[2], b[2]),
        lerp(a[3], b[3]),
    ]
}

fn put(data: &mut [u8], size: u32, x: u32, y: u32, c: [u8; 4]) {
    if x >= size || y >= size {
        return;
    }
    let idx = ((y * size + x) * 4) as usize;
    data[idx..idx + 4].copy_from_slice(&c);
}

fn fill(data: &mut [u8], size: u32, c: [u8; 4]) {
    for y in 0..size {
        for x in 0..size {
            put(data, size, x, y, c);
        }
    }
}

fn fill_grass(data: &mut [u8], size: u32) {
    for y in 0..size {
        for x in 0..size {
            // speckled grass via cheap hash
            let n = ((x.wrapping_mul(73) ^ y.wrapping_mul(131)).wrapping_add(x ^ y)) % 9;
            let c = blend(GRASS, GRASS_DARK, n as f32 / 9.0 * 0.35);
            put(data, size, x, y, c);
        }
    }
}

fn draw_path_pixel(data: &mut [u8], size: u32, x: u32, y: u32, dist_from_center: f32, half: f32, edge: f32) {
    let d = dist_from_center.abs();
    if d < half {
        let t = (d / half).powi(2);
        let c = blend(PATH_TAN, PATH_TAN_EDGE, t * 0.4);
        put(data, size, x, y, c);
    } else if d < half + edge {
        let t = (d - half) / edge;
        let c = blend(PATH_TAN_EDGE, GRASS_DARK, t);
        put(data, size, x, y, c);
    }
}

pub fn gen_path_straight(size: u32) -> Image {
    let mut data = vec![0u8; (size * size * 4) as usize];
    let s = size as f32;
    fill_grass(&mut data, size);
    let half = 0.14 * s;
    let edge = 0.05 * s;
    for y in 0..size {
        let fy = y as f32;
        let wave = (fy / s * std::f32::consts::PI * 2.0 * 1.2).sin() * s * 0.06;
        let center = s / 2.0 + wave;
        for x in 0..size {
            draw_path_pixel(&mut data, size, x, y, x as f32 - center, half, edge);
        }
    }
    make_image(size, data)
}

pub fn gen_path_curve(size: u32) -> Image {
    // Base openings: N | E. Path connects (size/2, 0) to (size, size/2).
    // Arc center = (size, 0), radius = size/2.
    let mut data = vec![0u8; (size * size * 4) as usize];
    let s = size as f32;
    fill_grass(&mut data, size);
    let cx = s;
    let cy = 0.0;
    let target = s / 2.0;
    let half = 0.14 * s;
    let edge = 0.05 * s;
    for y in 0..size {
        for x in 0..size {
            let fx = x as f32;
            let fy = y as f32;
            let dist = ((fx - cx).powi(2) + (fy - cy).powi(2)).sqrt();
            // wavy radius
            let angle = (fy / (fx - cx).abs().max(0.001)).atan();
            let wave = (angle * 6.0).sin() * s * 0.03;
            draw_path_pixel(&mut data, size, x, y, dist - target + wave, half, edge);
        }
    }
    make_image(size, data)
}

pub fn gen_path_tee(size: u32) -> Image {
    // Base openings: N | E | S. Vertical path top↔bottom plus an east branch.
    let mut data = vec![0u8; (size * size * 4) as usize];
    let s = size as f32;
    fill_grass(&mut data, size);
    let half = 0.14 * s;
    let edge = 0.05 * s;
    let cx = s / 2.0;
    let cy = s / 2.0;
    // vertical trunk (N↔S)
    for y in 0..size {
        for x in 0..size {
            let fy = y as f32;
            let wave = (fy / s * std::f32::consts::PI * 2.0 * 1.2).sin() * s * 0.04;
            draw_path_pixel(&mut data, size, x, y, x as f32 - cx + wave, half, edge);
        }
    }
    // east branch (center → east edge)
    for y in 0..size {
        for x in (size / 2)..size {
            let fx = x as f32;
            let wave = (fx / s * std::f32::consts::PI * 2.0 * 1.2).sin() * s * 0.04;
            draw_path_pixel(&mut data, size, x, y, y as f32 - cy + wave, half, edge);
        }
    }
    make_image(size, data)
}

pub fn gen_key(size: u32) -> Image {
    let mut data = vec![0u8; (size * size * 4) as usize];
    let s = size as f32;
    fill(&mut data, size, TRANSPARENT);
    let cx = s / 2.0;
    let cy = s / 2.0;
    let rx = s * 0.46;
    let ry = s * 0.34;
    // red oval
    for y in 0..size {
        for x in 0..size {
            let dx = (x as f32 - cx) / rx;
            let dy = (y as f32 - cy) / ry;
            let d = dx * dx + dy * dy;
            if d < 1.0 {
                let c = blend(KEY_RED, KEY_RED_DARK, d * 0.35);
                put(&mut data, size, x, y, c);
            }
        }
    }
    // gold key: bow (annulus) + shaft + teeth
    let bow_cx = s * 0.32;
    let bow_cy = s * 0.5;
    let bow_r = s * 0.10;
    let shaft_end = s * 0.75;
    let shaft_y0 = cy - s * 0.035;
    let shaft_y1 = cy + s * 0.035;
    for y in 0..size {
        for x in 0..size {
            let fx = x as f32;
            let fy = y as f32;
            let bd = ((fx - bow_cx).powi(2) + (fy - bow_cy).powi(2)).sqrt();
            let in_bow = bd < bow_r && bd > bow_r * 0.5;
            let in_shaft = fx > bow_cx && fx < shaft_end && fy > shaft_y0 && fy < shaft_y1;
            let tooth1 = fx > shaft_end - s * 0.03
                && fx < shaft_end
                && fy > shaft_y1
                && fy < shaft_y1 + s * 0.07;
            let tooth2 = fx > shaft_end - s * 0.11
                && fx < shaft_end - s * 0.08
                && fy > shaft_y1
                && fy < shaft_y1 + s * 0.05;
            if in_bow || in_shaft || tooth1 || tooth2 {
                put(&mut data, size, x, y, KEY_GOLD);
            }
        }
    }
    make_image(size, data)
}

pub fn gen_ogre(size: u32) -> Image {
    let mut data = vec![0u8; (size * size * 4) as usize];
    let s = size as f32;
    fill(&mut data, size, OGRE_BG);
    let cx = s * 0.5;
    let cy = s * 0.55;
    let rx = s * 0.38;
    let ry = s * 0.33;
    // face
    for y in 0..size {
        for x in 0..size {
            let dx = (x as f32 - cx) / rx;
            let dy = (y as f32 - cy) / ry;
            if dx * dx + dy * dy < 1.0 {
                put(&mut data, size, x, y, OGRE_FACE);
            }
        }
    }
    // hair/beard tufts
    let tufts: [(f32, f32, f32); 5] = [
        (cx - rx * 0.9, cy - ry * 0.1, s * 0.11),
        (cx + rx * 0.9, cy - ry * 0.1, s * 0.11),
        (cx - rx * 0.5, cy - ry * 0.9, s * 0.09),
        (cx + rx * 0.5, cy - ry * 0.9, s * 0.09),
        (cx, cy + ry * 0.85, s * 0.08),
    ];
    for (tcx, tcy, tr) in tufts {
        for y in 0..size {
            for x in 0..size {
                let fx = x as f32;
                let fy = y as f32;
                if (fx - tcx).powi(2) + (fy - tcy).powi(2) < tr * tr {
                    put(&mut data, size, x, y, OGRE_HAIR);
                }
            }
        }
    }
    // eyes
    let eyes = [(cx - rx * 0.32, cy - ry * 0.15), (cx + rx * 0.32, cy - ry * 0.15)];
    for (ecx, ecy) in eyes {
        let r = s * 0.05;
        for y in 0..size {
            for x in 0..size {
                let fx = x as f32;
                let fy = y as f32;
                if (fx - ecx).powi(2) + (fy - ecy).powi(2) < r * r {
                    put(&mut data, size, x, y, OGRE_EYE);
                }
            }
        }
    }
    make_image(size, data)
}

pub fn gen_snack(size: u32) -> Image {
    let mut data = vec![0u8; (size * size * 4) as usize];
    let s = size as f32;
    fill(&mut data, size, TRANSPARENT);
    // basket body
    let x0 = s * 0.18;
    let x1 = s * 0.82;
    let y0 = s * 0.35;
    let y1 = s * 0.82;
    for y in 0..size {
        for x in 0..size {
            let fx = x as f32;
            let fy = y as f32;
            if fx > x0 && fx < x1 && fy > y0 && fy < y1 {
                // weave check pattern
                let cell = (size / 10).max(1);
                let checker = ((x / cell) + (y / cell)) % 2;
                let c = if checker == 0 { SNACK_BROWN } else { SNACK_DARK };
                put(&mut data, size, x, y, c);
            }
        }
    }
    // rim
    for y in 0..size {
        for x in 0..size {
            let fx = x as f32;
            let fy = y as f32;
            if fx > x0 - s * 0.02 && fx < x1 + s * 0.02 && (fy - y0).abs() < s * 0.04 {
                put(&mut data, size, x, y, CHEST_DARK);
            }
        }
    }
    // handle arc
    let hcx = s * 0.5;
    let hcy = s * 0.35;
    let hr = s * 0.24;
    let hw = s * 0.035;
    for y in 0..size {
        for x in 0..size {
            let fx = x as f32;
            let fy = y as f32;
            if fy > hcy {
                continue;
            }
            let dd = ((fx - hcx).powi(2) + (fy - hcy).powi(2)).sqrt();
            if (dd - hr).abs() < hw {
                put(&mut data, size, x, y, SNACK_DARK);
            }
        }
    }
    make_image(size, data)
}

pub fn gen_treasure(size: u32) -> Image {
    let mut data = vec![0u8; (size * size * 4) as usize];
    let s = size as f32;
    fill(&mut data, size, TRANSPARENT);
    let x0 = s * 0.12;
    let x1 = s * 0.88;
    let y_top = s * 0.28;
    let y_mid = s * 0.48;
    let y_bot = s * 0.88;
    // chest body
    for y in 0..size {
        for x in 0..size {
            let fx = x as f32;
            let fy = y as f32;
            if fx > x0 && fx < x1 && fy > y_top && fy < y_bot {
                let near_edge = fx < x0 + s * 0.04
                    || fx > x1 - s * 0.04
                    || fy < y_top + s * 0.04
                    || fy > y_bot - s * 0.04;
                let c = if near_edge { CHEST_DARK } else { CHEST_WOOD };
                put(&mut data, size, x, y, c);
            }
        }
    }
    // lid band (dark divider)
    for y in 0..size {
        for x in 0..size {
            let fx = x as f32;
            let fy = y as f32;
            if fx > x0 && fx < x1 && (fy - y_mid).abs() < s * 0.03 {
                put(&mut data, size, x, y, CHEST_DARK);
            }
        }
    }
    // gold bands (vertical)
    let bands = [s * 0.25, s * 0.5, s * 0.75];
    for bx in bands {
        for y in 0..size {
            for x in 0..size {
                let fx = x as f32;
                let fy = y as f32;
                if (fx - bx).abs() < s * 0.025 && fy > y_top && fy < y_bot {
                    put(&mut data, size, x, y, KEY_GOLD);
                }
            }
        }
    }
    // keyhole (black dot on middle band)
    let kcx = s * 0.5;
    let kcy = y_mid;
    for y in 0..size {
        for x in 0..size {
            let fx = x as f32;
            let fy = y as f32;
            if (fx - kcx).powi(2) + (fy - kcy).powi(2) < (s * 0.035).powi(2) {
                put(&mut data, size, x, y, BLACK);
            }
        }
    }
    make_image(size, data)
}

