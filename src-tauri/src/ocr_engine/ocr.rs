use std::ffi::{CStr, CString, NulError};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr::NonNull;

use crate::ocr_engine::error::PipelineError;

/// A single recognised word with its bounding box in image pixel coordinates
/// (origin top-left).
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct WordBounds {
    pub text: String,
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    pub confidence: f32,
}

/// Safe wrapper around the Tesseract C API (`TessBaseAPI`). All FFI calls are
/// isolated with `catch_unwind` to prevent panics from propagating.
pub struct TessApi {
    api: NonNull<tesseract_sys::TessBaseAPI>,
}

unsafe impl Send for TessApi {}

impl TessApi {
    pub fn new(tessdata_path: &str, languages: &str) -> Result<Self, PipelineError> {
        let api = guard_unwind("TessBaseAPICreate", || unsafe {
            tesseract_sys::TessBaseAPICreate()
        })?;
        let api = NonNull::new(api)
            .ok_or_else(|| PipelineError::FfiOcr("TessBaseAPICreate failed".to_string()))?;
        let path = to_cstring(tessdata_path)?;
        let langs = to_cstring(languages)?;
        let init_result = guard_unwind("TessBaseAPIInit3", || unsafe {
            tesseract_sys::TessBaseAPIInit3(api.as_ptr(), path.as_ptr(), langs.as_ptr())
        })?;
        if init_result != 0 {
            return Err(PipelineError::FfiOcr("TessBaseAPIInit3 failed".to_string()));
        }
        Ok(Self { api })
    }

    pub fn set_image_bytes(
        &self,
        data: &[u8],
        width: i32,
        height: i32,
        bytes_per_pixel: i32,
        bytes_per_line: i32,
    ) -> Result<(), PipelineError> {
        guard_unwind("TessBaseAPISetImage", || unsafe {
            tesseract_sys::TessBaseAPISetImage(
                self.api.as_ptr(),
                data.as_ptr(),
                width,
                height,
                bytes_per_pixel,
                bytes_per_line,
            )
        })?;
        let rc = guard_unwind("TessBaseAPIRecognize", || unsafe {
            tesseract_sys::TessBaseAPIRecognize(self.api.as_ptr(), std::ptr::null_mut())
        })?;
        if rc != 0 {
            return Err(PipelineError::FfiOcr(
                "TessBaseAPIRecognize failed".to_string(),
            ));
        }
        Ok(())
    }

    pub fn get_text(&self) -> Result<String, PipelineError> {
        let ptr = guard_unwind("TessBaseAPIGetUTF8Text", || unsafe {
            tesseract_sys::TessBaseAPIGetUTF8Text(self.api.as_ptr())
        })?;
        if ptr.is_null() {
            return Err(PipelineError::FfiOcr(
                "TessBaseAPIGetUTF8Text returned null".to_string(),
            ));
        }
        let text = unsafe { CStr::from_ptr(ptr) }
            .to_str()
            .map_err(|_| PipelineError::FfiOcr("invalid UTF-8 from Tesseract".to_string()))?
            .to_string();
        unsafe { tesseract_sys::TessDeleteText(ptr) };
        Ok(text)
    }

    /// Iterates over recognised words and returns their bounding boxes and text.
    /// Must be called after `set_image_bytes()`. Returns an empty vec if no
    /// recognition data is available.
    pub fn get_words(&self) -> Result<Vec<WordBounds>, PipelineError> {
        let iter = guard_unwind("TessBaseAPIGetIterator", || unsafe {
            tesseract_sys::TessBaseAPIGetIterator(self.api.as_ptr())
        })?;
        let iter = match NonNull::new(iter) {
            Some(it) => it,
            None => return Ok(Vec::new()),
        };

        let mut words = Vec::new();
        let level = tesseract_sys::TessPageIteratorLevel_RIL_WORD;

        loop {
            let text_ptr = guard_unwind("TessResultIteratorGetUTF8Text", || unsafe {
                tesseract_sys::TessResultIteratorGetUTF8Text(iter.as_ptr() as *const _, level)
            })?;

            if !text_ptr.is_null() {
                let text = unsafe { CStr::from_ptr(text_ptr) }
                    .to_str()
                    .unwrap_or("")
                    .to_string();
                unsafe { tesseract_sys::TessDeleteText(text_ptr) };

                if !text.is_empty() {
                    let mut left: i32 = 0;
                    let mut top: i32 = 0;
                    let mut right: i32 = 0;
                    let mut bottom: i32 = 0;

                    // TessPageIterator (cast from ResultIterator) for bounding box
                    let page_iter = iter.as_ptr() as *const tesseract_sys::TessPageIterator;
                    let has_bbox = guard_unwind("TessPageIteratorBoundingBox", || unsafe {
                        tesseract_sys::TessPageIteratorBoundingBox(
                            page_iter,
                            level,
                            &mut left,
                            &mut top,
                            &mut right,
                            &mut bottom,
                        )
                    })?;

                    if has_bbox != 0 {
                        let confidence = guard_unwind("TessResultIteratorConfidence", || unsafe {
                            tesseract_sys::TessResultIteratorConfidence(
                                iter.as_ptr() as *const _,
                                level,
                            )
                        })?;
                        words.push(WordBounds {
                            text,
                            left,
                            top,
                            right,
                            bottom,
                            confidence,
                        });
                    }
                }
            }

            let has_next = guard_unwind("TessResultIteratorNext", || unsafe {
                tesseract_sys::TessResultIteratorNext(iter.as_ptr(), level)
            })?;
            if has_next == 0 {
                break;
            }
        }

        unsafe { tesseract_sys::TessResultIteratorDelete(iter.as_ptr()) };
        Ok(words)
    }

    pub fn clear(&self) -> Result<(), PipelineError> {
        guard_unwind("TessBaseAPIClear", || unsafe {
            tesseract_sys::TessBaseAPIClear(self.api.as_ptr())
        })?;
        Ok(())
    }

    pub fn set_page_seg_mode(&self, mode: tesseract_sys::PageSegMode) -> Result<(), PipelineError> {
        guard_unwind("TessBaseAPISetPageSegMode", || unsafe {
            tesseract_sys::TessBaseAPISetPageSegMode(self.api.as_ptr(), mode as _)
        })?;
        Ok(())
    }

    /// Informs Tesseract of the image's actual resolution in DPI.
    /// Without this, Tesseract auto-estimates resolution from image dimensions,
    /// which is often wrong for downscaled pages and causes character
    /// segmentation errors.
    pub fn set_source_resolution(&self, dpi: u32) -> Result<(), PipelineError> {
        guard_unwind("TessBaseAPISetSourceResolution", || unsafe {
            tesseract_sys::TessBaseAPISetSourceResolution(self.api.as_ptr(), dpi as i32)
        })?;
        Ok(())
    }
}

impl Drop for TessApi {
    fn drop(&mut self) {
        unsafe {
            tesseract_sys::TessBaseAPIEnd(self.api.as_ptr());
            tesseract_sys::TessBaseAPIDelete(self.api.as_ptr());
        }
    }
}

fn to_cstring(input: &str) -> Result<CString, PipelineError> {
    CString::new(input)
        .map_err(|NulError { .. }| PipelineError::FfiOcr("nul byte in tessdata path".to_string()))
}

fn guard_unwind<T>(label: &'static str, f: impl FnOnce() -> T) -> Result<T, PipelineError> {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(value) => Ok(value),
        Err(panic) => {
            tracing::error!(target: "knox::ocr", label, panic = ?panic, "FFI panic recovered");
            Err(PipelineError::PanicRecovered(format!("panic in {label}")))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::guard_unwind;

    #[test]
    fn guard_unwind_catches_panics() {
        let result: Result<(), _> = guard_unwind("test", || panic!("boom"));
        assert!(result.is_err());
    }
}
