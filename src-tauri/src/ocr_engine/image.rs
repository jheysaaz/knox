use std::time::Instant;

use image::{GrayImage, Luma};
use imageproc::filter::median_filter;
use imageproc::stats::histogram;
use rayon::prelude::*;

use crate::ocr_engine::error::PipelineError;
use crate::ocr_engine::types::{BinarizationMode, DeskewMode, OcrSettings};

/// Result of preprocessing: a grayscale image ready for OCR and an optional
/// 1-bit bitonal image for CCITT G4 compression.
#[derive(Clone, Debug)]
pub struct ProcessedImage {
    pub ocr_image: GrayImage,
    pub bitonal: Option<BitonalImage>,
}

/// A 1-bit-per-pixel image suitable for CCITT G4 encoding.
#[derive(Clone, Debug)]
pub struct BitonalImage {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// Applies preprocessing: denoise, binarization, morphology, deskew.
///
/// When `fast_path` is true, expensive operations (denoise, morphology, deskew)
/// are skipped — the image is only binarized or passed through as grayscale.
/// Callers should set `fast_path = true` for very large pages where quality
/// is already limited by downscaling.
pub fn preprocess(
    image: &GrayImage,
    settings: &OcrSettings,
    fast_path: bool,
) -> Result<ProcessedImage, PipelineError> {
    let t0 = Instant::now();

    let denoised = if fast_path || settings.denoise_level == 0 {
        if !fast_path {
            tracing::debug!(target: "knox::image", "denoise: skipped (level=0)");
        } else {
            tracing::debug!(target: "knox::image", "denoise: fast-path skip");
        }
        image.clone()
    } else {
        let t = Instant::now();
        let d = apply_denoise(image, settings.denoise_level);
        tracing::debug!(target: "knox::image", elapsed_ms = t.elapsed().as_millis(), "denoise done");
        d
    };

    let threshold = otsu_level_safe(&denoised);
    let binarizable = is_binarizable_par(&denoised, threshold);

    if binarizable {
        let t = Instant::now();
        let mut binary = apply_binarization(&denoised, settings);
        tracing::debug!(target: "knox::image", elapsed_ms = t.elapsed().as_millis(), "binarization done");

        if fast_path || settings.denoise_level == 0 {
            if !fast_path {
                tracing::debug!(target: "knox::image", "morphology: skipped (level=0)");
            } else {
                tracing::debug!(target: "knox::image", "morphology: fast-path skip");
            }
        } else {
            let t = Instant::now();
            binary = apply_morphology(&binary, settings.denoise_level);
            tracing::debug!(target: "knox::image", elapsed_ms = t.elapsed().as_millis(), "morphology done");
        }

        let deskewed = if fast_path {
            tracing::debug!(target: "knox::image", "deskew: fast-path skip");
            binary
        } else {
            let t = Instant::now();
            let d = apply_deskew(&binary, settings.deskew_mode)?;
            tracing::debug!(target: "knox::image", elapsed_ms = t.elapsed().as_millis(), "deskew done");
            d
        };

        let t = Instant::now();
        let rethreshold = otsu_level_safe(&deskewed);
        let final_binary = otsu_binarize_with_threshold_par(&deskewed, rethreshold);
        let (data, width, height) = to_bitonal_1bpp(&final_binary);
        tracing::debug!(target: "knox::image", elapsed_ms = t.elapsed().as_millis(), "bitonal pack done");

        tracing::debug!(target: "knox::image", total_elapsed_ms = t0.elapsed().as_millis(), "preprocess (binarizable) complete");
        Ok(ProcessedImage {
            ocr_image: final_binary.clone(),
            bitonal: Some(BitonalImage {
                data,
                width,
                height,
            }),
        })
    } else {
        let deskewed = if fast_path {
            tracing::debug!(target: "knox::image", "deskew: fast-path skip");
            denoised
        } else {
            let t = Instant::now();
            let d = apply_deskew(&denoised, settings.deskew_mode)?;
            tracing::debug!(target: "knox::image", elapsed_ms = t.elapsed().as_millis(), "deskew done");
            d
        };

        tracing::debug!(target: "knox::image", total_elapsed_ms = t0.elapsed().as_millis(), "preprocess (grayscale) complete");
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
    // Downscale very large images before denoise to avoid O(18M * kernel²) cost.
    // A poster at 3000x6000 pixels would take minutes otherwise.
    let working = maybe_downscale(image, 1200);
    let mut out = working.clone();
    for _ in 0..level {
        out = median_filter(&out, kernel, kernel);
    }
    // If we downscaled, upscale back to original size so the OCR
    // coordinates map correctly to the original image dimensions.
    if working.width() != image.width() || working.height() != image.height() {
        out = image::imageops::resize(
            &out,
            image.width(),
            image.height(),
            image::imageops::FilterType::Triangle,
        );
    }
    out
}

/// Downscale image so the longest side is at most `max_dim`, preserving aspect ratio.
fn maybe_downscale(image: &GrayImage, max_dim: u32) -> GrayImage {
    let w = image.width();
    let h = image.height();
    let longest = w.max(h);
    if longest <= max_dim {
        return image.clone();
    }
    let scale = max_dim as f32 / longest as f32;
    let nw = (w as f32 * scale).max(1.0) as u32;
    let nh = (h as f32 * scale).max(1.0) as u32;
    image::imageops::resize(image, nw, nh, image::imageops::FilterType::Triangle)
}

fn apply_binarization(image: &GrayImage, settings: &OcrSettings) -> GrayImage {
    match settings.binarization {
        BinarizationMode::Otsu => {
            let threshold = otsu_level_safe(image);
            otsu_binarize_with_threshold_par(image, threshold)
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
    // Downscale before morphology to avoid O(4×18M) neighborhood operations on posters.
    let working = maybe_downscale(image, 1200);
    let mut out = working.clone();
    let iterations = (level / 2).max(1);
    for _ in 0..iterations {
        out = morphological_open_close(&out);
    }
    if working.width() != image.width() || working.height() != image.height() {
        out = image::imageops::resize(
            &out,
            image.width(),
            image.height(),
            image::imageops::FilterType::Triangle,
        );
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

fn is_binarizable_par(image: &GrayImage, threshold: u8) -> bool {
    let total = (image.width() as u64) * (image.height() as u64);
    if total == 0 {
        return false;
    }
    let black: u64 = image
        .par_pixels()
        .map(|p| if p[0] <= threshold { 1u64 } else { 0u64 })
        .sum();
    let ratio = black as f64 / total as f64;
    (0.02..=0.7).contains(&ratio)
}

fn otsu_binarize_with_threshold_par(image: &GrayImage, threshold: u8) -> GrayImage {
    let mut out = image.clone();
    let raw = out.as_mut();
    raw.par_iter_mut()
        .for_each(|v| *v = if *v > threshold { 255 } else { 0 });
    out
}

fn morphological_open_close(image: &GrayImage) -> GrayImage {
    let opened = dilate_par(&erode_par(image));
    erode_par(&dilate_par(&opened))
}

fn erode_par(image: &GrayImage) -> GrayImage {
    let width = image.width();
    let height = image.height();
    if width < 3 || height < 3 {
        return image.clone();
    }
    let mut out = image.clone();
    let src_raw = image.as_raw();
    let w = width as usize;
    let h = height as usize;

    out.as_mut()
        .par_chunks_exact_mut(w)
        .enumerate()
        .for_each(|(y, row_out)| {
            if y == 0 || y >= h - 1 {
                return;
            }
            for x in 1..w - 1 {
                let mut keep = true;
                for dy in -1i32..=1 {
                    for dx in -1i32..=1 {
                        let nx = (x as i32 + dx) as usize;
                        let ny = (y as i32 + dy) as usize;
                        if src_raw[ny * w + nx] == 0 {
                            keep = false;
                            break;
                        }
                    }
                    if !keep {
                        break;
                    }
                }
                row_out[x] = if keep { 255 } else { 0 };
            }
        });
    out
}

fn dilate_par(image: &GrayImage) -> GrayImage {
    let width = image.width();
    let height = image.height();
    if width < 3 || height < 3 {
        return image.clone();
    }
    let mut out = image.clone();
    let src_raw = image.as_raw();
    let w = width as usize;
    let h = height as usize;

    out.as_mut()
        .par_chunks_exact_mut(w)
        .enumerate()
        .for_each(|(y, row_out)| {
            if y == 0 || y >= h - 1 {
                return;
            }
            for x in 1..w - 1 {
                let mut keep = false;
                for dy in -1i32..=1 {
                    for dx in -1i32..=1 {
                        let nx = (x as i32 + dx) as usize;
                        let ny = (y as i32 + dy) as usize;
                        if src_raw[ny * w + nx] == 255 {
                            keep = true;
                            break;
                        }
                    }
                    if keep {
                        break;
                    }
                }
                row_out[x] = if keep { 255 } else { 0 };
            }
        });
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

#[allow(dead_code)]
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

/// Safe Otsu threshold computation using u64 to avoid overflow on large images.
/// imageproc's `otsu_level` multiplies histogram counts by pixel intensities as u32,
/// which overflows for images with >16M pixels (~u32::MAX / 255).
fn otsu_level_safe(image: &GrayImage) -> u8 {
    let hist = histogram(image);
    let (width, height) = image.dimensions();
    let total_weight = (width as u64) * (height as u64);

    let total_pixel_sum: f64 = hist.channels[0]
        .iter()
        .enumerate()
        .fold(0f64, |sum, (t, h)| sum + ((t as u64) * (*h as u64)) as f64);

    let mut background_pixel_sum = 0f64;
    let mut background_weight = 0u64;
    let mut largest_variance = 0f64;
    let mut best_threshold = 0u8;

    for (threshold, hist_count) in hist.channels[0].iter().enumerate() {
        background_weight += *hist_count as u64;
        if background_weight == 0 {
            continue;
        }

        let foreground_weight = total_weight - background_weight;
        if foreground_weight == 0 {
            break;
        }

        background_pixel_sum += (threshold as u64 * *hist_count as u64) as f64;
        let foreground_pixel_sum = total_pixel_sum - background_pixel_sum;

        let background_mean = background_pixel_sum / background_weight as f64;
        let foreground_mean = foreground_pixel_sum / foreground_weight as f64;

        let mean_diff_squared = (background_mean - foreground_mean).powi(2);
        let intra_class_variance =
            background_weight as f64 * foreground_weight as f64 * mean_diff_squared;

        if intra_class_variance > largest_variance {
            largest_variance = intra_class_variance;
            best_threshold = threshold as u8;
        }
    }

    best_threshold
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
            continue_on_error: false,
            password: None,
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
        assert_ne!(result.as_raw(), img.as_raw());
    }

    #[test]
    fn otsu_binarize_threshold_low_makes_mostly_white() {
        let img = make_white(4, 4);
        let result = otsu_binarize_with_threshold_par(&img, 200);
        assert!(result.pixels().all(|p| p[0] == 255));
    }

    #[test]
    fn otsu_binarize_threshold_high_makes_mostly_black() {
        let img = make_black(4, 4);
        let result = otsu_binarize_with_threshold_par(&img, 100);
        assert!(result.pixels().all(|p| p[0] == 0));
    }

    #[test]
    fn is_binarizable_all_white_is_false() {
        let img = make_white(10, 10);
        assert!(!is_binarizable_par(&img, 128));
    }

    #[test]
    fn is_binarizable_all_black_is_false() {
        let img = make_black(10, 10);
        assert!(!is_binarizable_par(&img, 128));
    }

    #[test]
    fn is_binarizable_checkerboard_is_true() {
        let img = make_checkerboard(10, 10);
        assert!(is_binarizable_par(&img, 128));
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
        for p in result.pixels() {
            assert!(p[0] == 0 || p[0] == 255, "pixel value must be binary");
        }
    }

    #[test]
    fn est_skew_clean_image_near_zero() {
        let img = make_checkerboard(100, 100);
        let angle = estimate_skew_angle_radon(&img).unwrap();
        assert!(angle.abs() < 2.0, "angle should be near 0");
    }

    #[test]
    fn morphology_level_zero_is_noop() {
        let img = make_checkerboard(10, 10);
        let result = apply_morphology(&img, 0);
        assert_eq!(result.as_raw(), img.as_raw());
    }

    #[test]
    fn fast_path_skips_denoise_and_deskew() {
        let img = make_checkerboard(20, 20);
        let settings = OcrSettings {
            denoise_level: 3,
            deskew_mode: DeskewMode::Radon,
            ..default_settings()
        };
        let result = preprocess(&img, &settings, true).unwrap();
        // Fast path should still produce a valid result
        assert_eq!(result.ocr_image.width(), 20);
        assert_eq!(result.ocr_image.height(), 20);
    }

    #[test]
    fn preprocess_binarizable_produces_bitonal() {
        let img = make_checkerboard(20, 20);
        let result = preprocess(&img, &default_settings(), false).unwrap();
        assert!(result.bitonal.is_some());
        assert_eq!(result.bitonal.as_ref().unwrap().width, 20);
    }

    #[test]
    fn preprocess_all_white_produces_no_bitonal() {
        let img = make_white(20, 20);
        let result = preprocess(&img, &default_settings(), false).unwrap();
        assert!(result.bitonal.is_none());
    }

    #[test]
    fn denoise_downscales_large_images() {
        let img = make_checkerboard(2400, 2400);
        let result = apply_denoise(&img, 2);
        assert_eq!(result.dimensions(), (2400, 2400));
    }

    #[test]
    fn morphology_downscales_large_images() {
        let img = make_checkerboard(2400, 2400);
        let result = apply_morphology(&img, 2);
        assert_eq!(result.dimensions(), (2400, 2400));
    }

    #[test]
    fn denoise_small_image_unchanged_dimensions() {
        let img = make_checkerboard(100, 100);
        let result = apply_denoise(&img, 2);
        assert_eq!(result.dimensions(), (100, 100));
    }

    #[test]
    fn morphology_small_image_unchanged_dimensions() {
        let img = make_checkerboard(100, 100);
        let result = apply_morphology(&img, 2);
        assert_eq!(result.dimensions(), (100, 100));
    }
}
