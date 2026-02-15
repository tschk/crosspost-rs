//! Image processing utilities: compression, MIME detection, dimension reading, and validation.

use crate::error::{Error, Result};
use crate::types::ImageEmbed;
use image::ImageReader;
use std::io::Cursor;

/// Compress image data based on MIME type.
///
/// When the `image-compression` feature is disabled, returns data unchanged.
#[cfg(feature = "image-compression")]
pub fn compress_image(data: &[u8], mime: &str) -> Result<Vec<u8>> {
    match mime {
        "image/jpeg" => compress_jpeg(data, 85),
        "image/png" => compress_png(data),
        _ => Ok(data.to_vec()),
    }
}

/// No-op compression when the `image-compression` feature is disabled.
#[cfg(not(feature = "image-compression"))]
pub fn compress_image(data: &[u8], _mime: &str) -> Result<Vec<u8>> {
    Ok(data.to_vec())
}

/// Resize image if either dimension exceeds max_dimension, then compress.
#[cfg(feature = "image-compression")]
pub fn compress_and_resize(data: &[u8], mime: &str, max_dimension: u32) -> Result<Vec<u8>> {
    // Check dimensions before full decode to prevent image bombs
    let reader = ImageReader::new(Cursor::new(data))
        .with_guessed_format()
        .map_err(|e| Error::Platform(format!("Failed to read image: {}", e)))?;

    let (width, height) = reader
        .into_dimensions()
        .map_err(|e| Error::Platform(format!("Failed to get image dimensions: {}", e)))?;

    // Limit to 100 megapixels (e.g. 10,000 x 10,000) to prevent DoS
    if width as u64 * height as u64 > 100_000_000 {
        return Err(Error::Validation(
            "Image dimensions too large (exceeds 100 megapixels)".to_string(),
        ));
    }

    // Re-create reader for decoding since into_dimensions consumes it
    let img = ImageReader::new(Cursor::new(data))
        .with_guessed_format()
        .map_err(|e| Error::Platform(format!("Failed to read image: {}", e)))?
        .decode()
        .map_err(|e| Error::Platform(format!("Failed to decode image: {}", e)))?;

    let (w, h) = (img.width(), img.height());
    let resized = if w > max_dimension || h > max_dimension {
        img.resize(
            max_dimension,
            max_dimension,
            image::imageops::FilterType::Lanczos3,
        )
    } else {
        img
    };

    let mut buf = Cursor::new(Vec::new());
    match mime {
        "image/png" => {
            resized
                .write_to(&mut buf, image::ImageFormat::Png)
                .map_err(|e| Error::Platform(format!("Failed to encode PNG: {}", e)))?;
            compress_png(buf.get_ref())
        }
        "image/jpeg" => {
            resized
                .write_to(&mut buf, image::ImageFormat::Jpeg)
                .map_err(|e| Error::Platform(format!("Failed to encode JPEG: {}", e)))?;
            compress_jpeg(buf.get_ref(), 85)
        }
        _ => {
            resized
                .write_to(&mut buf, image::ImageFormat::Png)
                .map_err(|e| Error::Platform(format!("Failed to encode image: {}", e)))?;
            Ok(buf.into_inner())
        }
    }
}

#[cfg(feature = "image-compression")]
fn compress_jpeg(data: &[u8], quality: u8) -> Result<Vec<u8>> {
    let img = ImageReader::new(Cursor::new(data))
        .with_guessed_format()
        .map_err(|e| Error::Platform(format!("Failed to read JPEG: {}", e)))?
        .decode()
        .map_err(|e| Error::Platform(format!("Failed to decode JPEG: {}", e)))?;

    let rgb = img.to_rgb8();
    let (width, height) = (rgb.width() as usize, rgb.height() as usize);

    let mut comp = mozjpeg::Compress::new(mozjpeg::ColorSpace::JCS_RGB);
    comp.set_size(width, height);
    comp.set_quality(quality as f32);
    let mut comp = comp
        .start_compress(Vec::new())
        .map_err(|e| Error::Platform(format!("mozjpeg compress error: {}", e)))?;
    comp.write_scanlines(rgb.as_raw())
        .map_err(|e| Error::Platform(format!("mozjpeg scanline error: {}", e)))?;
    let compressed = comp
        .finish()
        .map_err(|e| Error::Platform(format!("mozjpeg finish error: {}", e)))?;

    Ok(compressed)
}

#[cfg(feature = "image-compression")]
fn compress_png(data: &[u8]) -> Result<Vec<u8>> {
    let opts = oxipng::Options::from_preset(2);
    oxipng::optimize_from_memory(data, &opts).or_else(|_| Ok(data.to_vec()))
}

/// Detect MIME type from image bytes
pub fn detect_mime_type(data: &[u8]) -> Result<String> {
    let kind = infer::get(data)
        .ok_or_else(|| Error::Validation("Unable to detect image type".to_string()))?;

    match kind.mime_type() {
        "image/png" | "image/jpeg" | "image/gif" => Ok(kind.mime_type().to_string()),
        other => Err(Error::Validation(format!(
            "Unsupported image type: {}. Only PNG, JPEG, and GIF are supported.",
            other
        ))),
    }
}

/// Get image dimensions from bytes
pub fn image_dimensions(data: &[u8]) -> Result<(u32, u32)> {
    let reader = ImageReader::new(Cursor::new(data))
        .with_guessed_format()
        .map_err(|e| Error::Platform(format!("Failed to read image: {}", e)))?;

    let dimensions = reader
        .into_dimensions()
        .map_err(|e| Error::Platform(format!("Failed to get image dimensions: {}", e)))?;

    Ok(dimensions)
}

/// Maximum size for a single image (20 MB)
const MAX_IMAGE_SIZE: usize = 20 * 1024 * 1024;

/// Maximum total size for all images combined (50 MB)
const MAX_TOTAL_IMAGE_SIZE: usize = 50 * 1024 * 1024;

/// Maximum image dimensions (65536 x 65536 = ~4 gigapixels)
const MAX_IMAGE_DIMENSION: u32 = 65536;

/// Validate image data (type, count, size, dimensions)
pub fn validate_images(images: &[ImageEmbed]) -> Result<()> {
    if images.len() > 4 {
        return Err(Error::Validation(
            "Maximum of 4 images per post".to_string(),
        ));
    }

    let mut total_size: usize = 0;

    for img in images {
        // Byte size check
        if img.data.len() > MAX_IMAGE_SIZE {
            return Err(Error::Validation(format!(
                "Image too large: {} bytes (max {} bytes)",
                img.data.len(),
                MAX_IMAGE_SIZE
            )));
        }
        total_size += img.data.len();

        // MIME type check
        if let Some(ref mime) = img.mime_type {
            match mime.as_str() {
                "image/png" | "image/jpeg" | "image/gif" => {}
                other => {
                    return Err(Error::Validation(format!(
                        "Unsupported image type: {}",
                        other
                    )));
                }
            }
        } else {
            detect_mime_type(&img.data)?;
        }

        // Dimension check (prevents image bombs: small file, huge decoded size)
        if let Ok((w, h)) = image_dimensions(&img.data) {
            if w > MAX_IMAGE_DIMENSION || h > MAX_IMAGE_DIMENSION {
                return Err(Error::Validation(format!(
                    "Image dimensions too large: {}x{} (max {}x{})",
                    w, h, MAX_IMAGE_DIMENSION, MAX_IMAGE_DIMENSION
                )));
            }
        }
    }

    if total_size > MAX_TOTAL_IMAGE_SIZE {
        return Err(Error::Validation(format!(
            "Total image size too large: {} bytes (max {} bytes)",
            total_size, MAX_TOTAL_IMAGE_SIZE
        )));
    }

    Ok(())
}
