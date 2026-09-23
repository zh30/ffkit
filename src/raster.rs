use image::{Rgba, RgbaImage};

use crate::error::Error;

pub fn render_caption(text: &str, font_bytes: &[u8], video_w: u32) -> Result<RgbaImage, Error> {
    render_caption_styled(text, font_bytes, video_w, [255, 255, 255], 1.0)
}

pub fn render_caption_styled(
    text: &str,
    font_bytes: &[u8],
    video_w: u32,
    fg: [u8; 3],
    size: f32,
) -> Result<RgbaImage, Error> {
    render_text(text, font_bytes, video_w, 14.0, fg, size)
}

/// Caption text with per-line alignment (left/right speaker lines, lyrics).
pub fn render_caption_aligned(
    text: &str,
    font_bytes: &[u8],
    video_w: u32,
    fg: [u8; 3],
    size: f32,
    align: TextAlign,
) -> Result<RgbaImage, Error> {
    render_text_inner(text, font_bytes, video_w, 14.0, fg, size, None, align)
}

/// Caption text stroked for readability on busy frames.
pub fn render_caption_outlined(
    text: &str,
    font_bytes: &[u8],
    video_w: u32,
    fg: [u8; 3],
    size: f32,
    outline: ([u8; 3], u32),
) -> Result<RgbaImage, Error> {
    render_text_inner(
        text,
        font_bytes,
        video_w,
        14.0,
        fg,
        size,
        Some(outline),
        TextAlign::Center,
    )
}

/// Scale every pixel's alpha by pct/100 — ghost/watermark text cards.
pub fn alpha_scale(img: &mut RgbaImage, pct: f64) -> Result<(), Error> {
    if !(1.0..=100.0).contains(&pct) {
        return Err(Error::input("--opacity must be 1..=100"));
    }
    for px in img.pixels_mut() {
        px.0[3] = (px.0[3] as f64 * pct / 100.0).round() as u8;
    }
    Ok(())
}

pub fn render_title_styled(
    text: &str,
    font_bytes: &[u8],
    video_w: u32,
    fg: [u8; 3],
    size: f32,
) -> Result<RgbaImage, Error> {
    render_text(text, font_bytes, video_w, 8.0, fg, size)
}

/// Title text with a stroke around each glyph (readability on busy frames).
/// `outline` = (rgb, width_px) — blits each glyph at 16 offsets before the fill.
pub fn render_title_outlined(
    text: &str,
    font_bytes: &[u8],
    video_w: u32,
    fg: [u8; 3],
    size: f32,
    outline: ([u8; 3], u32),
) -> Result<RgbaImage, Error> {
    render_text_inner(
        text,
        font_bytes,
        video_w,
        8.0,
        fg,
        size,
        Some(outline),
        TextAlign::Center,
    )
}

/// Title card with a soft drop shadow: renders the card twice, blurs a
/// darkened copy, offsets it down-right, then lays the card on top.
pub fn render_title_shadow(
    text: &str,
    font_bytes: &[u8],
    video_w: u32,
    fg: [u8; 3],
    size: f32,
    blur: u32,
) -> Result<RgbaImage, Error> {
    let card = render_text_inner(
        text,
        font_bytes,
        video_w,
        8.0,
        fg,
        size,
        None,
        TextAlign::Center,
    )?;
    let off = (blur.max(2) / 2).max(2);
    let mut ghost = render_text_inner(
        text,
        font_bytes,
        video_w,
        8.0,
        [12, 12, 16],
        size,
        None,
        TextAlign::Center,
    )?;
    for px in ghost.pixels_mut() {
        px.0[0] = 12;
        px.0[1] = 12;
        px.0[2] = 16;
    }
    let shadow = image::imageops::blur(&ghost, blur.max(1) as f32);
    let mut canvas = RgbaImage::from_pixel(
        card.width() + off * 2,
        card.height() + off * 2,
        Rgba([0, 0, 0, 0]),
    );
    image::imageops::overlay(&mut canvas, &shadow, off as i64, off as i64);
    image::imageops::overlay(&mut canvas, &card, 0, 0);
    Ok(canvas)
}

pub fn render_title(text: &str, font_bytes: &[u8], video_w: u32) -> Result<RgbaImage, Error> {
    render_text(text, font_bytes, video_w, 8.0, [255, 255, 255], 1.0)
}

fn render_text(
    text: &str,
    font_bytes: &[u8],
    video_w: u32,
    divisor: f32,
    fg: [u8; 3],
    size: f32,
) -> Result<RgbaImage, Error> {
    render_text_inner(
        text,
        font_bytes,
        video_w,
        divisor,
        fg,
        size,
        None,
        TextAlign::Center,
    )
}

#[allow(clippy::too_many_arguments)]
fn render_text_inner(
    text: &str,
    font_bytes: &[u8],
    video_w: u32,
    divisor: f32,
    fg: [u8; 3],
    size: f32,
    outline: Option<([u8; 3], u32)>,
    align: TextAlign,
) -> Result<RgbaImage, Error> {
    let font = fontdue::Font::from_bytes(font_bytes, fontdue::FontSettings::default())
        .map_err(|e| Error::input(format!("font parse: {e}")))?;
    let px = ((video_w as f32) / divisor * size).clamp(8.0, 512.0);
    let pad = (px * 0.35).round() as u32;
    let lines: Vec<&str> = text.lines().filter(|l| !l.is_empty()).collect();
    if lines.is_empty() {
        return Err(Error::input("caption cue is empty"));
    }

    let mut line_glyphs: Vec<Vec<(fontdue::Metrics, Vec<u8>)>> = Vec::new();
    let mut line_widths: Vec<u32> = Vec::new();
    let mut line_heights: Vec<u32> = Vec::new();
    for line in &lines {
        let mut glyphs = Vec::new();
        let mut w = 0u32;
        let mut h = 0u32;
        for ch in line.chars() {
            let (metrics, bitmap) = font.rasterize(ch, px);
            w = w.saturating_add(metrics.advance_width.ceil().max(0.0) as u32);
            h = h.max(metrics.height as u32 + metrics.ymin.abs().max(0) as u32);
            glyphs.push((metrics, bitmap));
        }
        line_glyphs.push(glyphs);
        line_widths.push(w);
        line_heights.push(h.max(px.ceil() as u32));
    }

    let content_w = line_widths.iter().copied().max().unwrap_or(1);
    let content_h: u32 =
        line_heights.iter().sum::<u32>() + pad * ((lines.len().saturating_sub(1) as u32) / 2);
    let img_w = (content_w + pad * 2).max(2) & !1;
    let img_h = (content_h + pad * 2).max(2) & !1;
    let mut img = RgbaImage::from_pixel(img_w, img_h, Rgba([0, 0, 0, 180]));

    let mut y = pad as i32;
    for (glyphs, (lw, lh)) in line_glyphs
        .iter()
        .zip(line_widths.iter().zip(line_heights.iter()))
    {
        let mut x = match align {
            TextAlign::Left => pad as f32,
            TextAlign::Right => (pad + content_w.saturating_sub(*lw)) as f32,
            TextAlign::Center => (pad + (content_w.saturating_sub(*lw)) / 2) as f32,
        };
        for (metrics, bitmap) in glyphs {
            let gx = x + metrics.xmin as f32;
            let gy = y as f32 + (*lh as f32) + metrics.ymin as f32 - metrics.height as f32;
            if let Some((oc, ow)) = outline {
                // cheap stroke: ring of offset blits under the fill
                let r = ow.max(1) as f32;
                for k in 0..16 {
                    let a = k as f32 * std::f32::consts::TAU / 16.0;
                    blit_glyph(
                        &mut img,
                        (gx + a.cos() * r).round() as i32,
                        (gy + a.sin() * r).round() as i32,
                        metrics,
                        bitmap,
                        oc,
                    );
                }
            }
            blit_glyph(
                &mut img,
                gx.round() as i32,
                gy.round() as i32,
                metrics,
                bitmap,
                fg,
            );
            x += metrics.advance_width;
        }
        y += *lh as i32 + (pad as i32 / 2);
    }
    Ok(img)
}

fn blit_glyph(
    img: &mut RgbaImage,
    x0: i32,
    y0: i32,
    metrics: &fontdue::Metrics,
    bitmap: &[u8],
    fg: [u8; 3],
) {
    let w = metrics.width;
    let h = metrics.height;
    if w == 0 || h == 0 {
        return;
    }
    for gy in 0..h {
        for gx in 0..w {
            let a = bitmap[gy * w + gx];
            if a == 0 {
                continue;
            }
            let x = x0 + gx as i32;
            let y = y0 + gy as i32;
            if x < 0 || y < 0 {
                continue;
            }
            let (x, y) = (x as u32, y as u32);
            if x >= img.width() || y >= img.height() {
                continue;
            }
            img.put_pixel(x, y, Rgba([fg[0], fg[1], fg[2], a]));
        }
    }
}

/// Multi-line alignment inside a rendered text card.
#[derive(Clone, Copy, Debug, Default, clap::ValueEnum)]
pub enum TextAlign {
    Left,
    #[default]
    Center,
    Right,
}

/// render_title_styled with a line alignment (lower-thirds convention).
pub fn render_title_aligned(
    text: &str,
    font_bytes: &[u8],
    video_w: u32,
    fg: [u8; 3],
    size: f32,
    align: TextAlign,
) -> Result<RgbaImage, Error> {
    render_text_inner(text, font_bytes, video_w, 8.0, fg, size, None, align)
}
