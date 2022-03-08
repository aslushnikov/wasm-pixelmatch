use wasm_bindgen::prelude::*;
use std::cmp;

#[wasm_bindgen]
pub fn pixelmatch(img1: &[u8], img2: &[u8], output: &mut [u8], width: usize, height: usize, threshold: f64) -> usize {
    console_error_panic_hook::set_once();
    if img1.len() != img2.len() || img1.len() != output.len() {
        wasm_bindgen::throw_str("Image sizes do not match");
    }

    if img1.len() != width * height * 4 {
        wasm_bindgen::throw_str("Image data size does not match width/height");
    }

    let includeAA = false;       // whether to skip anti-aliasing detection
    let alpha = 0.1;             // opacity of original image in diff output
    let aaColor = [255, 255, 0]; // color of anti-aliased pixels in diff output
    let diffColor = [255, 0, 0]; // color of different pixels in diff output

    let len = width * height;
    let mut identical = true;
    for i in 0..len {
        if img1[i] != img2[i] {
            identical = false;
            break;
        }
    }
    if identical {
        for i in 0..len {
            drawGrayPixel(img1, 4 * i, alpha, output);
        }
        return 0;
    }

    // maximum acceptable square distance between two colors;
    // 35215 is the maximum possible value for the YIQ difference metric
    let max_delta = 35215.0 * threshold * threshold;
    let mut diff = 0;

    // compare each pixel of one image against the other one
    for y in 0..height {
        for x in 0..width {
            let pos = (y * width + x) * 4;
            // squared YUV distance between colors at this pixel position, negative if the img2 pixel is darker
            let delta = colorDelta(img1, img2, pos, pos, false);
            if delta.abs() > max_delta {
                // check it's a real rendering difference or just anti-aliasing
                if !includeAA && antialiased(img1, x, y, width, height, img2) || antialiased(img2, x, y, width, height, img1) {
                    // one of the pixels is anti-aliasing; draw as yellow and do not count as difference
                    drawPixel(output, pos, aaColor[0], aaColor[1], aaColor[2]);
                } else {
                    // found substantial difference not caused by anti-aliasing; draw it as such
                    drawPixel(output, pos, diffColor[0], diffColor[1], diffColor[2]);
                    diff = diff + 1;
                }
            } else {
                // pixels are similar; draw background as grayscale image blended with white
                drawGrayPixel(img1, pos, alpha, output);
            }
        }
    }

    diff
}

fn antialiased(img: &[u8], x1: usize, y1: usize, width: usize, height: usize, img2: &[u8]) -> bool {
    let x0 = cmp::max(x1 - 1, 0);
    let y0 = cmp::max(y1 - 1, 0);
    let x2 = cmp::min(x1 + 1, width - 1);
    let y2 = cmp::min(y1 + 1, height - 1);
    let pos = (y1 * width + x1) * 4;
    let mut zeroes = if x1 == x0 || x1 == x2 || y1 == y0 || y1 == y2 {
        1
    } else {
        0
    };

    let mut min = 0.0;
    let mut max = 0.0;
    let mut min_x = 0;
    let mut min_y = 0;
    let mut max_x = 0;
    let mut max_y = 0;

    // go through 8 adjacent pixels
    for x in x0..=x2 {
        for y in y0..=y2 {
            if x == x1 && y == y1 {
                continue;
            }

            // brightness delta between the center pixel and adjacent one
            let delta = colorDelta(img, img, pos, (y * width + x) * 4, true);
            // count the number of equal, darker and brighter adjacent pixels
            if delta == 0.0 {
                zeroes = zeroes + 1;
                if zeroes > 2 {
                    return false;
                }
            } else if delta < min {
                min = delta;
                min_x = x;
                min_y = y;
            } else if delta > max {
                max = delta;
                max_x = x;
                max_y = y;
            }

        }
    }

    // if there are no both darker and brighter pixels among siblings, it's not anti-aliasing
    if min == 0.0 || max == 0.0 {
        return false;
    }

    // if either the darkest or the brightest pixel has 3+ equal siblings in both images
    // (definitely not anti-aliased), this pixel is anti-aliased
    return (hasManySiblings(img, min_x, min_y, width, height) && hasManySiblings(img2, min_x, min_y, width, height)) ||
           (hasManySiblings(img, max_x, max_y, width, height) && hasManySiblings(img2, max_x, max_y, width, height));
}

// check if a pixel has 3+ adjacent pixels of the same color.
fn hasManySiblings(img: &[u8], x1: usize, y1: usize, width: usize, height: usize) -> bool {
    let x0 = cmp::max(x1 - 1, 0);
    let y0 = cmp::max(y1 - 1, 0);
    let x2 = cmp::max(x1 + 1, width - 1);
    let y2 = cmp::max(y1 + 1, height - 1);
    let pos = (y1 * width + x1) * 4;
    let mut zeroes = if x1 == x0 || x1 == x2 || y1 == y0 || y1 == y2  { 1 } else { 0 };

    // go through 8 adjacent pixels
    for x in x0..=x2 {
        for y in y0..=y2 {
            if x == x1 && y == y1 { continue; }

            let pos2 = (y * width + x) * 4;
            if img[pos] == img[pos2] &&
               img[pos + 1] == img[pos2 + 1] &&
               img[pos + 2] == img[pos2 + 2] &&
               img[pos + 3] == img[pos2 + 3] {
                zeroes = zeroes + 1;
            }

            if zeroes > 2 {
                return true;
            }
        }
    }

    return false;
}

// calculate color difference according to the paper "Measuring perceived color difference
// using YIQ NTSC transmission color space in mobile applications" by Y. Kotsarenko and F. Ramos

fn colorDelta(img1: &[u8], img2: &[u8], k: usize, m: usize, yOnly: bool) -> f64 {
    let mut r1 = img1[k + 0];
    let mut g1 = img1[k + 1];
    let mut b1 = img1[k + 2];
    let mut a1 = img1[k + 3] as f64;

    let mut r2 = img2[m + 0];
    let mut g2 = img2[m + 1];
    let mut b2 = img2[m + 2];
    let mut a2 = img2[m + 3] as f64;

    if a1 == a2 && r1 == r2 && g1 == g2 && b1 == b2 {
        return 0.0;
    }

    if a1 < 255.0 {
        a1 /= 255.0;
        r1 = blend(r1, a1);
        g1 = blend(g1, a1);
        b1 = blend(b1, a1);
    }

    if a2 < 255.0 {
        a2 /= 255.0;
        r2 = blend(r2, a2);
        g2 = blend(g2, a2);
        b2 = blend(b2, a2);
    }

    let y1 = rgb2y(r1, g1, b1);
    let y2 = rgb2y(r2, g2, b2);
    let y = y1 - y2;

    if yOnly {
        return y as f64; // brightness difference only
    }

    let i = rgb2i(r1, g1, b1) - rgb2i(r2, g2, b2);
    let q = rgb2q(r1, g1, b1) - rgb2q(r2, g2, b2);

    let delta = 0.5053 * (y as f64) * (y as f64) + 0.299 * i * i + 0.1957 * q * q;

    // encode whether the pixel lightens or darkens in the sign
    if y1 > y2 {
        -delta
    } else {
        delta
    }
}

fn rgb2y(r: u8, g: u8, b: u8) -> u8 { ((r as f64) * 0.29889531 + (g as f64) * 0.58662247 + (b as f64) * 0.11448223) as u8 }
fn rgb2i(r: u8, g: u8, b: u8) -> f64 { (r as f64) * 0.59597799 - (g as f64) * 0.27417610 - (b as f64) * 0.32180189 }
fn rgb2q(r: u8, g: u8, b: u8) -> f64 { (r as f64) * 0.21147017 - (g as f64) * 0.52261711 + (b as f64) * 0.31114694 }

// blend semi-transparent color with white
fn blend(c: u8, a: f64) -> u8 {
    (255.0 + ((c as f64) - 255.0) * a) as u8
}

fn drawPixel(output: &mut [u8], pos: usize, r: u8, g: u8, b: u8) {
    output[pos + 0] = r;
    output[pos + 1] = g;
    output[pos + 2] = b;
    output[pos + 3] = 255;
}

fn drawGrayPixel(img: &[u8], i: usize, alpha: f64, output: &mut [u8]) {
    let r = img[i + 0];
    let g = img[i + 1];
    let b = img[i + 2];
    let val = blend(rgb2y(r, g, b), alpha * (img[i + 3] as f64) / 255.0);
    drawPixel(output, i, val, val, val);
}
