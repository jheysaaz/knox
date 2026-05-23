#[cfg(test)]
mod tests {
    use super::super::runtime::build_runtime;
    use super::super::types::ProcessingConfig;

    #[test]
    fn runtime_default_limits_nonzero() {
        let config = ProcessingConfig {
            max_concurrent_files: None,
            tessdata_path: "/tmp".to_string(),
            languages: "eng".to_string(),
            thread_pool_size: None,
        };
        let runtime = build_runtime(&config);
        assert!(runtime.file_semaphore.available_permits() >= 1);
    }

    #[test]
    fn extract_skew_pdf_images() {
        use std::path::Path;
        let doc = lopdf::Document::load(Path::new("/tmp/skew.pdf"))
            .expect("load skew.pdf (download from ocrmypdf repo if missing)");
        let pages = super::super::pdf::extract_page_images(
            &doc,
            super::super::types::ExistingTextMode::Rasterize,
        )
        .expect("extract_page_images");
        assert!(!pages.is_empty(), "should extract at least one page");
        let page = &pages[0];
        let img = &page.image;
        println!("Extracted image: {}x{}", img.width(), img.height());
        assert!(img.width() > 0);
        assert!(img.height() > 0);
        // Count black/white pixels
        let mut black = 0u64;
        let mut total = 0u64;
        for pixel in img.pixels() {
            total += 1;
            if pixel[0] == 0 {
                black += 1;
            }
        }
        let ratio = black as f64 / total as f64;
        println!("Black pixels: {}/{} ({:.1}%)", black, total, ratio * 100.0);
        assert!(ratio > 0.01, "should have >1% black pixels (text)");
        assert!(
            ratio < 0.15,
            "should have <15% black pixels (not all black)"
        );
    }
}
