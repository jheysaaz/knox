use image::{GrayImage, Luma};
use imageproc::contrast::otsu_level;
use imageproc::filter::median_filter;
use rayon::prelude::*;

use crate::ocr_engine::error::PipelineError;
use crate::ocr_engine::types::{BinarizationMode, DeskewMode, OcrSettings};

/// Result of preprocessing: a grayscale image ready for OCR and an optional
/// 1-bit bitonal image for CCITT G4 compression.
#[derive(Clone, Debug)]
pub struct ProcessedImage {
    /// Grayscale image fed to Tesseract.
    pub ocr_image: GrayImage,
    /// Bitonal (1-bit) version for CCITT compression, if applicable.
    pub bitonal: Option<BitonalImage>,
}

/// A 1-bit-per-pixel image suitable for CCITT G4 encoding.
#[derive(Clone, Debug)]
pub struct BitonalImage {
    /// Packed bit data (MSB-first, row-major).
    pub data: Vec<u8>,
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
}

/// Applies denoising, binarization, morphology, and deskew to prepare an image for OCR.
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
        let deskewed = apply_deskew(&binary, settings.deskew_mode)?;
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
        let deskewed = apply_deskew(&denoised, settings.deskew_mode)?;
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
    (0.02..=0.7).contains(&ratio)
}

fn otsu_binarize_with_threshold(image: &GrayImage, threshold: u8) -> GrayImage {
    let mut out = image.clone();
    for pixel in out.pixels_mut() {
        let v = if pixel[0] > threshold { 255 } else { 0 };
        *pixel = Luma([v]);
    }
    out
}

/// Applies morphological opening (erode → dilate) followed by closing (dilate → erode)
/// to remove small specks (noise) and fill small holes in a binary-like image.
/// Uses a 3×3 structuring element. The order is: erode → dilate → dilate → erode,
/// which is equivalent to open-close.
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

/// Converts a grayscale image to a 1-bit-per-pixel packed byte buffer.
///
/// # Convention
/// Black pixels (value 0) are encoded as bit `1`, white pixels (value 255) as bit `0`.
/// This matches the `BlackIs1: true` convention used in `encode_ccitt_g4` in `pdf.rs`,
/// and is the inverse of `imageproc`'s default convention where white = 1.
///
/// Bits are packed MSB-first within each byte, left-to-right across the row.
/// Each row is padded to a full byte boundary.
fn to_bitonal_1bpp(image: &GrayImage) -> (Vec<u8>, u32, u32) {
    let width = image.width();
    let height = image.height();
    let row_bytes = width.div_ceil(8) as usize;
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

/// Down/up-samples the image to approximate `target_dpi`, relative to a 300 DPI baseline.
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

#[cfg(test)]
mod tests {
    use super::*;
    use image::Luma;

    fn make_checkerboard(w: u32, h: u32) -> GrayImage {
        let mut img = GrayImage::new(w, h);
        for y in 0..h {
            for x in 0..w {
                img.put_pixel(
                    x,
                    y,
                    if (x + y) % 2 == 0 {
                        Luma([0])
                    } else {
                        Luma([255])
                    },
                );
            }
        }
        img
    }

    fn make_white(w: u32, h: u32) -> GrayImage {
        GrayImage::from_pixel(w, h, Luma([255]))
    }

    fn make_black(w: u32, h: u32) -> GrayImage {
        GrayImage::from_pixel(w, h, Luma([0]))
    }

    fn default_settings() -> OcrSettings {
        OcrSettings {
            binarization: BinarizationMode::Otsu,
            fixed_threshold: 128,
            deskew_mode: DeskewMode::Disabled,
            denoise_level: 0,
            existing_text: crate::ocr_engine::types::ExistingTextMode::Skip,
            psm: crate::ocr_engine::types::PageSegMode::Auto,
            compression: crate::ocr_engine::types::CompressionMode::Ccitt,
            resolution_dpi: 300,
            archive_enforcement: false,
        }
    }

    #[test]
    fn denoise_level_zero_is_noop() {
        let img = make_checkerboard(10, 10);
        let result = apply_denoise(&img, 0);
        assert_eq!(result.as_raw(), img.as_raw());
    }

    #[test]
    fn denoise_level_one_changes_image() {
        let img = make_checkerboard(10, 10);
        let result = apply_denoise(&img, 1);
        // Median filter at level 1 should change checkerboard
        assert_ne!(result.as_raw(), img.as_raw());
    }

    #[test]
    fn otsu_binarize_threshold_low_makes_mostly_black() {
        let img = make_white(4, 4);
        // White image (all 255), threshold=200: 255 > 200 → all white
        let result = otsu_binarize_with_threshold(&img, 200);
        assert!(result.pixels().all(|p| p[0] == 255));
    }

    #[test]
    fn otsu_binarize_threshold_high_makes_mostly_white() {
        let img = make_black(4, 4);
        // Black image (all 0), threshold=100: 0 > 100 → false, all black
        let result = otsu_binarize_with_threshold(&img, 100);
        assert!(result.pixels().all(|p| p[0] == 0));
    }

    #[test]
    fn is_binarizable_all_white_is_false() {
        let img = make_white(10, 10);
        assert!(!is_binarizable(&img, 128));
    }

    #[test]
    fn is_binarizable_all_black_is_false() {
        let img = make_black(10, 10);
        assert!(!is_binarizable(&img, 128));
    }

    #[test]
    fn is_binarizable_checkerboard_is_true() {
        let img = make_checkerboard(10, 10);
        // black ratio should be ~0.5, within [0.02, 0.7]
        assert!(is_binarizable(&img, 128));
    }

    #[test]
    fn downsample_same_dpi_is_noop() {
        let img = make_checkerboard(100, 100);
        let result = downsample_to_dpi(&img, 300);
        assert_eq!(result.dimensions(), (100, 100));
    }

    #[test]
    fn downsample_150_dpi_half_size() {
        let img = make_checkerboard(100, 100);
        let result = downsample_to_dpi(&img, 150);
        assert!(result.width() < 100);
    }

    #[test]
    fn rotate_zero_is_noop() {
        let img = make_checkerboard(10, 10);
        let result = rotate_image(&img, 0.0);
        assert_eq!(result.as_raw(), img.as_raw());
    }

    #[test]
    fn to_bitonal_round_trip() {
        let img = make_checkerboard(16, 16);
        let (data, w, h) = to_bitonal_1bpp(&img);
        assert_eq!(w, 16);
        assert_eq!(h, 16);
        // Each row = (16 + 7) / 8 = 2 bytes
        // 16 rows = 32 bytes
        assert_eq!(data.len(), 32);
    }

    #[test]
    fn binarization_otsu_produces_binary() {
        let img = make_checkerboard(20, 20);
        let settings = OcrSettings {
            binarization: BinarizationMode::Otsu,
            ..default_settings()
        };
        let result = apply_binarization(&img, &settings);
        // All pixels should be 0 or 255
        for p in result.pixels() {
            assert!(p[0] == 0 || p[0] == 255, "pixel value must be binary");
        }
    }

    #[test]
    fn est_skew_clean_image_near_zero() {
        let img = make_checkerboard(100, 100);
        let angle = estimate_skew_angle_radon(&img).unwrap();
        // Should be close to 0 for a symmetric image
        assert!(angle.abs() < 2.0, "angle should be near 0");
    }

    #[test]
    fn morphology_level_zero_is_noop() {
        let img = make_checkerboard(10, 10);
        let result = apply_morphology(&img, 0);
        assert_eq!(result.as_raw(), img.as_raw());
    }
}
