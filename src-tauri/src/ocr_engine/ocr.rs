use std::ffi::{CString, NulError};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr::NonNull;

use crate::ocr_engine::error::PipelineError;

pub struct TessApi {
    api: NonNull<tesseract_sys::TessBaseAPI>,
}

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
        let text = unsafe { std::ffi::CStr::from_ptr(ptr) }
            .to_str()
            .map_err(|_| PipelineError::FfiOcr("invalid UTF-8 from Tesseract".to_string()))?
            .to_string();
        unsafe { tesseract_sys::TessDeleteText(ptr) };
        Ok(text)
    }

    pub fn set_page_seg_mode(&self, mode: tesseract_sys::PageSegMode) -> Result<(), PipelineError> {
        guard_unwind("TessBaseAPISetPageSegMode", || unsafe {
            tesseract_sys::TessBaseAPISetPageSegMode(self.api.as_ptr(), mode as u32)
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
        Err(_) => Err(PipelineError::PanicRecovered(format!("panic in {label}"))),
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
