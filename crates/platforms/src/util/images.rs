use crosspost_core::{Error, Result};
use image::ImageReader;
use std::io::Cursor;

/// Compress image data based on MIME type.
/// JPEG: re-encode via mozjpeg at quality 85.
/// PNG: optimize via oxipng (lossless).
/// GIF: pass through unchanged.
pub fn compress_image(data: &[u8], mime: &str) -> Result<Vec<u8>> {
    match mime {
        "image/jpeg" => compress_jpeg(data, 85),
        "image/png" => compress_png(data),
        _ => Ok(data.to_vec()), // GIF and others: pass through
    }
}

/// Resize image if either dimension exceeds max_dimension, then compress.
pub fn compress_and_resize(data: &[u8], mime: &str, max_dimension: u32) -> Result<Vec<u8>> {
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

    // Encode to the same format, then compress
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
            // GIF or other: just write as PNG (resize loses animation anyway)
            resized
                .write_to(&mut buf, image::ImageFormat::Png)
                .map_err(|e| Error::Platform(format!("Failed to encode image: {}", e)))?;
            Ok(buf.into_inner())
        }
    }
}

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

fn compress_png(data: &[u8]) -> Result<Vec<u8>> {
    let opts = oxipng::Options::from_preset(2);
    oxipng::optimize_from_memory(data, &opts).or_else(|_| {
        // If optimization fails, return original
        Ok(data.to_vec())
    })
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
    let reader = image::ImageReader::new(std::io::Cursor::new(data))
        .with_guessed_format()
        .map_err(|e| Error::Platform(format!("Failed to read image: {}", e)))?;

    let dimensions = reader
        .into_dimensions()
        .map_err(|e| Error::Platform(format!("Failed to get image dimensions: {}", e)))?;

    Ok(dimensions)
}

/// Validate image data (type, count)
pub fn validate_images(images: &[crate::platform_trait::ImageEmbed]) -> Result<()> {
    if images.len() > 4 {
        return Err(Error::Validation(
            "Maximum of 4 images per post".to_string(),
        ));
    }

    for img in images {
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
    }

    Ok(())
}
