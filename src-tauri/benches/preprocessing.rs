use criterion::{black_box, criterion_group, criterion_main, Criterion};

use image::{GrayImage, Luma};
use knox_lib::ocr_engine::image::preprocess;
use knox_lib::ocr_engine::types::{
    BinarizationMode, CompressionMode, DeskewMode, ExistingTextMode, OcrSettings, PageSegMode,
};

fn make_test_image(w: u32, h: u32) -> GrayImage {
    let mut img = GrayImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let phase = (x / 8 + y / 6) % 2;
            let noise = ((x ^ y) & 0x3F) as u8;
            let v = if phase == 0 {
                200u8.saturating_add(noise)
            } else {
                noise.saturating_sub(30)
            };
            img.put_pixel(x, y, Luma([v]));
        }
    }
    img
}

fn default_settings() -> OcrSettings {
    OcrSettings {
        binarization: BinarizationMode::Otsu,
        fixed_threshold: 128,
        deskew_mode: DeskewMode::Radon,
        denoise_level: 2,
        existing_text: ExistingTextMode::Skip,
        psm: PageSegMode::Auto,
        compression: CompressionMode::Ccitt,
        resolution_dpi: 300,
        archive_enforcement: false,
        continue_on_error: false,
        password: None,
    }
}

fn bench_preprocess_pipeline(c: &mut Criterion) {
    let img = make_test_image(600, 400);
    let no_denoise = OcrSettings {
        denoise_level: 0,
        deskew_mode: DeskewMode::Disabled,
        ..default_settings()
    };
    let light_denoise = OcrSettings {
        denoise_level: 1,
        ..default_settings()
    };
    let heavy_denoise = OcrSettings {
        denoise_level: 3,
        ..default_settings()
    };
    let otsu = default_settings();
    let fixed_bin = OcrSettings {
        binarization: BinarizationMode::Fixed,
        fixed_threshold: 128,
        ..default_settings()
    };
    let bradley = OcrSettings {
        binarization: BinarizationMode::BradleyRoth,
        ..default_settings()
    };

    let mut group = c.benchmark_group("preprocess/pipeline");
    group.bench_function("denoise_0", |b| {
        b.iter(|| preprocess(black_box(&img), black_box(&no_denoise), black_box(false)))
    });
    group.bench_function("denoise_1", |b| {
        b.iter(|| preprocess(black_box(&img), black_box(&light_denoise), black_box(false)))
    });
    group.bench_function("denoise_2", |b| {
        b.iter(|| preprocess(black_box(&img), black_box(&otsu), black_box(false)))
    });
    group.bench_function("denoise_3", |b| {
        b.iter(|| preprocess(black_box(&img), black_box(&heavy_denoise), black_box(false)))
    });
    group.bench_function("binarization_otsu", |b| {
        b.iter(|| preprocess(black_box(&img), black_box(&otsu), black_box(false)))
    });
    group.bench_function("binarization_fixed", |b| {
        b.iter(|| preprocess(black_box(&img), black_box(&fixed_bin), black_box(false)))
    });
    group.bench_function("binarization_bradley", |b| {
        b.iter(|| preprocess(black_box(&img), black_box(&bradley), black_box(false)))
    });
    group.finish();

    // Benchmark size scalings
    let mut sz_group = c.benchmark_group("preprocess/size");
    for (label, w, h) in [("A4_72dpi", 595, 842), ("A4_150dpi", 1240, 1754)] {
        let big = make_test_image(w, h);
        sz_group.bench_function(label, |b| {
            b.iter(|| preprocess(black_box(&big), black_box(&otsu), black_box(false)))
        });
    }
    sz_group.finish();
}

criterion_group!(benches, bench_preprocess_pipeline);
criterion_main!(benches);
