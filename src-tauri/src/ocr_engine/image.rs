use image::{GrayImage, Luma};
use imageproc::contrast::otsu_level;
use imageproc::filter::median_filter;
use rayon::prelude::*;

use crate::ocr_engine::error::PipelineError;
use crate::ocr_engine::types::{BinarizationMode, DeskewMode, OcrSettings};

#[derive(Clone, Debug)]
pub struct ProcessedImage {
    pub ocr_image: GrayImage,
    pub bitonal: Option<BitonalImage>,
}

#[derive(Clone, Debug)]
pub struct BitonalImage {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

pub fn preprocess(
    image: &GrayImage,
    settings: &OcrSettings,
) -> Result<ProcessedImage, PipelineError> {
    let denoised = apply_denoise(image, settings.denoise_level);
    let threshold = otsu_level(&denoised);
    let binarizable = is_binarizable(&denoised, threshold);

    if binarizable {
        let mut binary = apply_binarization(&denoised, settings);
        binary = apply_morphology(&binary, settings.denoise_level);
        let deskewed = apply_deskew(&binary, settings.deskew_mode.clone())?;
        let rethreshold = otsu_level(&deskewed);
        let final_binary = otsu_binarize_with_threshold(&deskewed, rethreshold);
        let (data, width, height) = to_bitonal_1bpp(&final_binary);
        Ok(ProcessedImage {
            ocr_image: final_binary.clone(),
            bitonal: Some(BitonalImage {
                data,
                width,
                height,
            }),
        })
    } else {
        let deskewed = apply_deskew(&denoised, settings.deskew_mode.clone())?;
        Ok(ProcessedImage {
            ocr_image: deskewed.clone(),
            bitonal: None,
        })
    }
}

fn apply_denoise(image: &GrayImage, level: u8) -> GrayImage {
    if level == 0 {
        return image.clone();
    }
    let kernel = if level <= 2 { 3 } else { 5 };
    let mut out = image.clone();
    for _ in 0..level {
        out = median_filter(&out, kernel, kernel);
    }
    out
}

fn apply_binarization(image: &GrayImage, settings: &OcrSettings) -> GrayImage {
    match settings.binarization {
        BinarizationMode::Otsu => {
            let threshold = otsu_level(image);
            otsu_binarize_with_threshold(image, threshold)
        }
        BinarizationMode::BradleyRoth => {
            let block = (settings.denoise_level as u32).max(1) * 4 + 7;
            imageproc::contrast::adaptive_threshold(image, block)
        }
        BinarizationMode::Fixed => imageproc::contrast::threshold(image, settings.fixed_threshold),
    }
}

fn apply_morphology(image: &GrayImage, level: u8) -> GrayImage {
    if level == 0 {
        return image.clone();
    }
    let mut out = image.clone();
    let iterations = (level / 2).max(1);
    for _ in 0..iterations {
        out = morphological_open_close(&out);
    }
    out
}

fn apply_deskew(image: &GrayImage, mode: DeskewMode) -> Result<GrayImage, PipelineError> {
    match mode {
        DeskewMode::Radon => deskew_radon(image),
        DeskewMode::Hough => deskew_hough(image),
        DeskewMode::Disabled => Ok(image.clone()),
    }
}

fn is_binarizable(image: &GrayImage, threshold: u8) -> bool {
    let mut black = 0u64;
    let mut total = 0u64;
    for pixel in image.pixels() {
        total += 1;
        if pixel[0] <= threshold {
            black += 1;
        }
    }
    if total == 0 {
        return false;
    }
    let ratio = black as f64 / total as f64;
    ratio >= 0.02 && ratio <= 0.7
}

fn otsu_binarize_with_threshold(image: &GrayImage, threshold: u8) -> GrayImage {
    let mut out = image.clone();
    for pixel in out.pixels_mut() {
        let v = if pixel[0] > threshold { 255 } else { 0 };
        *pixel = Luma([v]);
    }
    out
}

fn morphological_open_close(image: &GrayImage) -> GrayImage {
    let opened = dilate(&erode(image));
    erode(&dilate(&opened))
}

fn erode(image: &GrayImage) -> GrayImage {
    let mut out = image.clone();
    let width = image.width();
    let height = image.height();
    for y in 1..height.saturating_sub(1) {
        for x in 1..width.saturating_sub(1) {
            let mut keep = true;
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    let nx = (x as i32 + dx) as u32;
                    let ny = (y as i32 + dy) as u32;
                    if image.get_pixel(nx, ny)[0] == 0 {
                        keep = false;
                        break;
                    }
                }
                if !keep {
                    break;
                }
            }
            out.put_pixel(x, y, if keep { Luma([255]) } else { Luma([0]) });
        }
    }
    out
}

fn dilate(image: &GrayImage) -> GrayImage {
    let mut out = image.clone();
    let width = image.width();
    let height = image.height();
    for y in 1..height.saturating_sub(1) {
        for x in 1..width.saturating_sub(1) {
            let mut keep = false;
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    let nx = (x as i32 + dx) as u32;
                    let ny = (y as i32 + dy) as u32;
                    if image.get_pixel(nx, ny)[0] == 255 {
                        keep = true;
                        break;
                    }
                }
                if keep {
                    break;
                }
            }
            out.put_pixel(x, y, if keep { Luma([255]) } else { Luma([0]) });
        }
    }
    out
}

fn deskew_radon(image: &GrayImage) -> Result<GrayImage, PipelineError> {
    let angle = estimate_skew_angle_radon(image)?;
    Ok(rotate_image(image, -angle))
}

fn deskew_hough(image: &GrayImage) -> Result<GrayImage, PipelineError> {
    let edges = imageproc::edges::canny(image, 50.0, 150.0);
    let lines = imageproc::hough::detect_lines(
        &edges,
        imageproc::hough::LineDetectionOptions {
            vote_threshold: 80,
            suppression_radius: 4,
        },
    );
    if lines.is_empty() {
        return Ok(image.clone());
    }
    let mut angles = Vec::new();
    for line in lines {
        let angle = line.angle_in_degrees as f32;
        let normalized = if angle > 90.0 { angle - 180.0 } else { angle };
        if normalized.abs() <= 20.0 {
            angles.push(normalized);
        }
    }
    if angles.is_empty() {
        return Ok(image.clone());
    }
    let mean = angles.iter().sum::<f32>() / angles.len() as f32;
    Ok(rotate_image(image, -mean))
}

fn to_bitonal_1bpp(image: &GrayImage) -> (Vec<u8>, u32, u32) {
    let width = image.width();
    let height = image.height();
    let row_bytes = ((width + 7) / 8) as usize;
    let mut data = vec![0u8; row_bytes * height as usize];
    for y in 0..height {
        for x in 0..width {
            let pixel = image.get_pixel(x, y)[0];
            if pixel == 0 {
                let byte_index = (y as usize * row_bytes) + (x as usize / 8);
                let bit_index = 7 - (x % 8);
                data[byte_index] |= 1u8 << bit_index;
            }
        }
    }
    (data, width, height)
}

fn estimate_skew_angle_radon(image: &GrayImage) -> Result<f32, PipelineError> {
    let scaled = downscale_for_radon(image, 800);
    let width = scaled.width() as f64;
    let height = scaled.height() as f64;
    if width == 0.0 || height == 0.0 {
        return Ok(0.0);
    }
    let max_radius = (width.powi(2) + height.powi(2)).sqrt() / 2.0;
    let bins = max_radius.ceil() as usize * 2 + 1;
    let angles: Vec<f64> = (-50..=50).map(|i| i as f64 * 0.1).collect();

    let scores: Vec<(f64, f64)> = angles
        .par_iter()
        .map(|angle| {
            let radians = angle.to_radians();
            let cos = radians.cos();
            let sin = radians.sin();
            let mut projection = vec![0f64; bins];
            let cx = width / 2.0;
            let cy = height / 2.0;
            for y in 0..scaled.height() {
                for x in 0..scaled.width() {
                    let pixel = scaled.get_pixel(x, y)[0];
                    if pixel > 0 {
                        continue;
                    }
                    let xf = x as f64 - cx;
                    let yf = y as f64 - cy;
                    let r = xf * cos + yf * sin;
                    let idx = ((r + max_radius) / (2.0 * max_radius) * (bins as f64 - 1.0)).round()
                        as isize;
                    if idx >= 0 && (idx as usize) < bins {
                        projection[idx as usize] += 1.0;
                    }
                }
            }
            let mean = projection.iter().sum::<f64>() / bins as f64;
            let variance = projection
                .iter()
                .map(|v| {
                    let diff = v - mean;
                    diff * diff
                })
                .sum::<f64>();
            (*angle, variance)
        })
        .collect();

    let best = scores
        .into_iter()
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|v| v.0)
        .unwrap_or(0.0);
    Ok(best as f32)
}

fn downscale_for_radon(image: &GrayImage, max_dim: u32) -> GrayImage {
    let width = image.width();
    let height = image.height();
    let max_current = width.max(height);
    if max_current <= max_dim {
        return image.clone();
    }
    let scale = max_dim as f32 / max_current as f32;
    let new_width = (width as f32 * scale).max(1.0) as u32;
    let new_height = (height as f32 * scale).max(1.0) as u32;
    image::imageops::resize(
        image,
        new_width,
        new_height,
        image::imageops::FilterType::Nearest,
    )
}

pub fn downsample_to_dpi(image: &GrayImage, target_dpi: u16) -> GrayImage {
    let scale = (target_dpi as f32 / 300.0).clamp(0.25, 4.0);
    if (scale - 1.0).abs() < 0.01 {
        return image.clone();
    }
    let new_width = (image.width() as f32 * scale).max(1.0) as u32;
    let new_height = (image.height() as f32 * scale).max(1.0) as u32;
    image::imageops::resize(
        image,
        new_width,
        new_height,
        image::imageops::FilterType::Triangle,
    )
}

fn rotate_image(image: &GrayImage, angle_degrees: f32) -> GrayImage {
    if angle_degrees.abs() < 0.01 {
        return image.clone();
    }
    imageproc::geometric_transformations::rotate_about_center(
        image,
        angle_degrees.to_radians(),
        imageproc::geometric_transformations::Interpolation::Bilinear,
        Luma([255]),
    )
}
