//! SPEC-180 08 "Limits enforced before HTTP": an image is checked, and re-encoded when
//! needed, before it reaches the embedding endpoint.
//!
//! | input | result |
//! |-------|--------|
//! | PNG, JPEG, static WebP within budget | sent as is |
//! | over the byte budget | JPEG q85, then smaller, else `image_too_large` |
//! | side above 16384 px | `image_too_large` |
//! | GIF, animated WebP | `image_animated_unsupported` |
//! | bytes the decoder rejects | `image_decode_failed` |

use edgequake_core::multimodal_embed::{embed_codes, EmbedError};
use image::{codecs::jpeg::JpegEncoder, DynamicImage, ImageFormat};

/// Largest accepted side, in pixels.
pub const MAX_SIDE_PX: u32 = 16_384;
/// Token cost the budget counts per image (SPEC-180 08).
pub const IMAGE_TOKENS: usize = 258;
/// Most images one input may carry.
pub const MAX_IMAGES_PER_INPUT: usize = 6;

const JPEG_QUALITY: u8 = 85;
/// Downscale steps tried when q85 alone is still over budget.
const SHRINK_STEPS: [f32; 4] = [0.75, 0.5, 0.35, 0.25];

/// An image ready to send.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedImage {
    pub mime: &'static str,
    pub bytes: Vec<u8>,
}

fn err(code: &'static str, message: impl Into<String>) -> EmbedError {
    EmbedError::new(code, message)
}

fn sniff(bytes: &[u8]) -> Option<ImageFormat> {
    image::guess_format(bytes).ok()
}

/// A WebP file holds an animation when its container declares the `ANIM` chunk.
fn webp_is_animated(bytes: &[u8]) -> bool {
    bytes.windows(4).take(256).any(|w| w == b"ANIM")
}

/// True when the bytes start like an image the sniffer knows.
pub fn looks_like_image(bytes: &[u8]) -> bool {
    sniff(bytes).is_some()
}

fn mime_of(format: ImageFormat) -> Option<&'static str> {
    match format {
        ImageFormat::Png => Some("image/png"),
        ImageFormat::Jpeg => Some("image/jpeg"),
        ImageFormat::WebP => Some("image/webp"),
        _ => None,
    }
}

fn encode_jpeg(image: &DynamicImage) -> Result<Vec<u8>, EmbedError> {
    let rgb = image.to_rgb8();
    let mut out = Vec::new();
    JpegEncoder::new_with_quality(&mut out, JPEG_QUALITY)
        .encode_image(&rgb)
        .map_err(|e| {
            err(
                embed_codes::IMAGE_DECODE_FAILED,
                format!("JPEG encode failed: {e}"),
            )
        })?;
    Ok(out)
}

fn too_large(max_bytes: usize) -> EmbedError {
    err(
        embed_codes::IMAGE_TOO_LARGE,
        format!("image stays above {max_bytes} bytes after re-encoding"),
    )
}

/// Re-encode to JPEG, shrinking until the bytes fit `max_bytes`.
fn fit_budget(image: &DynamicImage, max_bytes: usize) -> Result<Vec<u8>, EmbedError> {
    let mut jpeg = encode_jpeg(image)?;
    if jpeg.len() <= max_bytes {
        return Ok(jpeg);
    }
    for factor in SHRINK_STEPS {
        let width = ((image.width() as f32) * factor).round().max(1.0) as u32;
        let height = ((image.height() as f32) * factor).round().max(1.0) as u32;
        let smaller = image.resize(width, height, image::imageops::FilterType::Triangle);
        jpeg = encode_jpeg(&smaller)?;
        if jpeg.len() <= max_bytes {
            return Ok(jpeg);
        }
    }
    Err(too_large(max_bytes))
}

/// Check one image against the limits. `max_bytes == 0` disables the byte budget.
pub fn prepare_image(bytes: &[u8], max_bytes: usize) -> Result<PreparedImage, EmbedError> {
    let format = sniff(bytes).ok_or_else(|| {
        err(
            embed_codes::IMAGE_DECODE_FAILED,
            "the bytes are not a known image",
        )
    })?;
    if format == ImageFormat::Gif || (format == ImageFormat::WebP && webp_is_animated(bytes)) {
        return Err(err(
            embed_codes::IMAGE_ANIMATED,
            "animated images are not supported: embedding the first frame would surprise you",
        ));
    }
    let decoded = image::load_from_memory(bytes).map_err(|e| {
        err(
            embed_codes::IMAGE_DECODE_FAILED,
            format!("the image cannot be decoded: {e}"),
        )
    })?;
    if decoded.width() > MAX_SIDE_PX || decoded.height() > MAX_SIDE_PX {
        return Err(err(
            embed_codes::IMAGE_TOO_LARGE,
            format!("image side exceeds {MAX_SIDE_PX} px"),
        ));
    }
    let within_budget = max_bytes == 0 || bytes.len() <= max_bytes;
    if let (true, Some(mime)) = (within_budget, mime_of(format)) {
        return Ok(PreparedImage {
            mime,
            bytes: bytes.to_vec(),
        });
    }
    // Over budget, or a format the endpoint does not take (BMP, TIFF, ...): JPEG.
    let jpeg = if max_bytes == 0 {
        encode_jpeg(&decoded)?
    } else {
        fit_budget(&decoded, max_bytes)?
    };
    Ok(PreparedImage {
        mime: "image/jpeg",
        bytes: jpeg,
    })
}

/// Token estimate of one input: 258 per image plus one per four characters of text.
pub fn estimate_tokens(image_count: usize, text_chars: usize) -> usize {
    image_count * IMAGE_TOKENS + text_chars.div_ceil(4)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgb};

    fn noise_png(width: u32, height: u32) -> Vec<u8> {
        let img = ImageBuffer::from_fn(width, height, |x, y| {
            let v = (x.wrapping_mul(2654435761) ^ y.wrapping_mul(40503)) as u8;
            Rgb([v, v.wrapping_add(80), v.wrapping_mul(3)])
        });
        let mut out = Vec::new();
        DynamicImage::ImageRgb8(img)
            .write_to(&mut std::io::Cursor::new(&mut out), ImageFormat::Png)
            .expect("png");
        out
    }

    #[test]
    fn small_png_is_sent_unchanged() {
        let png = noise_png(16, 16);
        let prepared = prepare_image(&png, 3_500_000).expect("ok");
        assert_eq!(prepared.mime, "image/png");
        assert_eq!(prepared.bytes, png);
    }

    #[test]
    fn oversized_png_becomes_a_jpeg_inside_the_budget() {
        let png = noise_png(400, 400);
        assert!(png.len() > 20_000);
        let prepared = prepare_image(&png, 20_000).expect("re-encoded");
        assert_eq!(prepared.mime, "image/jpeg");
        assert!(prepared.bytes.len() <= 20_000);
    }

    #[test]
    fn an_impossible_budget_is_image_too_large() {
        let png = noise_png(200, 200);
        let e = prepare_image(&png, 10).unwrap_err();
        assert_eq!(e.code, embed_codes::IMAGE_TOO_LARGE);
    }

    #[test]
    fn gif_is_refused_as_animated() {
        let e = prepare_image(b"GIF89a\x01\x00\x01\x00\x00\x00\x00;", 0).unwrap_err();
        assert_eq!(e.code, embed_codes::IMAGE_ANIMATED);
    }

    #[test]
    fn animated_webp_is_refused() {
        let mut bytes = b"RIFF\x00\x00\x00\x00WEBPVP8X\x0a\x00\x00\x00\x02\x00\x00\x00\x00\x00\x00\x00\x00\x00ANIM".to_vec();
        bytes.extend_from_slice(&[0; 16]);
        let e = prepare_image(&bytes, 0).unwrap_err();
        assert_eq!(e.code, embed_codes::IMAGE_ANIMATED);
    }

    #[test]
    fn garbage_is_a_decode_failure() {
        assert_eq!(
            prepare_image(b"not an image at all", 0).unwrap_err().code,
            embed_codes::IMAGE_DECODE_FAILED
        );
        let mut truncated = noise_png(64, 64);
        truncated.truncate(60);
        assert_eq!(
            prepare_image(&truncated, 0).unwrap_err().code,
            embed_codes::IMAGE_DECODE_FAILED
        );
    }

    #[test]
    fn zero_budget_disables_the_byte_limit() {
        let png = noise_png(300, 300);
        let prepared = prepare_image(&png, 0).expect("ok");
        assert_eq!(prepared.mime, "image/png");
    }

    #[test]
    fn token_estimate_matches_the_contract() {
        assert_eq!(estimate_tokens(1, 0), 258);
        assert_eq!(estimate_tokens(0, 9), 3);
        assert_eq!(estimate_tokens(2, 8), 516 + 2);
    }
}
