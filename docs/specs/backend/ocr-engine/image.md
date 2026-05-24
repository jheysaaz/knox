# Image Processing Module Spec

## Functions

### `preprocess(image: &GrayImage, settings: &OcrSettings) -> ProcessedImage`
Pipeline: denoise → otsu_threshold → is_binarizable?
  YES: binarize → morphology → deskew → re-threshold → bitonal
  NO: deskew → grayscale output

### `apply_denoise(image, level: u8) -> GrayImage`
- level 0: no-op
- level ≤ 2: median filter 3x3, repeated `level` times
- level > 2: median filter 5x5, repeated `level` times

### `apply_binarization(image, settings) -> GrayImage`
- Otsu: compute otsu threshold, apply
- BradleyRoth: adaptive threshold with block_size=denoise_level*4+7
- Fixed: threshold at settings.fixed_threshold

### `is_binarizable(image, threshold) -> bool`
- Count black pixels (pixel ≤ threshold)
- Returns true if black ratio is 2%-70%

### `otsu_binarize_with_threshold(image, threshold) -> GrayImage`
- Pixels > threshold → white (255), else → black (0)

### `morphological_open_close(image) -> GrayImage`
- open: erode → dilate
- close: dilate → erode
- Combined: open(dilate(erode(open(image))))

### `deskew_radon(image) -> GrayImage`
- Estimate skew via Radon transform on downscaled image
- Search angles -5° to +5° in 0.1° steps
- Rotate by -angle
- Uses parallel iteration via rayon

### `deskew_hough(image) -> GrayImage`
- Canny edge detection
- Hough line transform
- Average angles of lines within ±20°
- Rotate by -mean_angle

### `to_bitonal_1bpp(image) -> (data, width, height)`
- Pack 8 pixels per byte (MSB first)
- Black pixel → 1 bit set

### `downsample_to_dpi(image, target_dpi) -> GrayImage`
- Scale relative to 300 DPI baseline
- Clamp between 0.25x and 4x

### `estimate_skew_angle_radon(image) -> f32`
- Downscale to max 800px
- Radon projection at angles -5° to +5° (0.1° steps)
- Return angle with maximum projection variance

## Acceptance Criteria
- Denoise level 0 returns identical image
- Otsu binarization produces binary output
- Fixed threshold at 255 produces all-white image
- Threshold at 0 produces all-black image
- is_binarizable returns false for all-white image
- Bitonal packing round-trips correctly
- Deskew with Disabled returns original
- Downsample at 300 DPI is no-op
- Radon rotation at 0° returns near-identical image
