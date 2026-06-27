//! Asset byte loader for drawing tags (LightRAG sidecar assets dir subset).

use std::path::{Path, PathBuf};

use edgequake_pdf::inline_images::InlineImageRef;

/// Resolved image payload ready for VLM gates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedImageAsset {
    pub bytes: Vec<u8>,
    pub mime_type: String,
}

/// Load image bytes: inline data-URI first, then filesystem `path` under `base_dir`.
pub fn resolve_image_asset(
    image_ref: &InlineImageRef,
    base_dir: Option<&Path>,
) -> Result<ResolvedImageAsset, String> {
    if !image_ref.bytes.is_empty() {
        let mime = if image_ref.mime_type.is_empty() {
            guess_mime_from_bytes(&image_ref.bytes)
        } else {
            image_ref.mime_type.clone()
        };
        return Ok(ResolvedImageAsset {
            bytes: image_ref.bytes.clone(),
            mime_type: mime,
        });
    }

    let rel_path = image_ref
        .asset_path
        .as_deref()
        .filter(|p| !p.is_empty())
        .ok_or_else(|| "missing inline bytes and asset path".to_string())?;

    let base = base_dir.ok_or_else(|| format!("asset path '{rel_path}' but no base_dir"))?;
    let full = sanitize_asset_path(base, rel_path)?;
    let bytes = std::fs::read(&full).map_err(|e| format!("failed to read asset {full:?}: {e}"))?;
    if bytes.is_empty() {
        return Err(format!("empty asset file {full:?}"));
    }

    let mime = if image_ref.mime_type.is_empty() {
        guess_mime_from_path(&full).unwrap_or_else(|| guess_mime_from_bytes(&bytes))
    } else {
        image_ref.mime_type.clone()
    };

    Ok(ResolvedImageAsset {
        bytes,
        mime_type: mime,
    })
}

fn sanitize_asset_path(base: &Path, rel: &str) -> Result<PathBuf, String> {
    let joined = base.join(rel);
    let canonical_base = base
        .canonicalize()
        .map_err(|e| format!("invalid asset base {base:?}: {e}"))?;
    let canonical = joined
        .canonicalize()
        .map_err(|e| format!("asset not found {joined:?}: {e}"))?;
    if !canonical.starts_with(&canonical_base) {
        return Err(format!("asset path escapes base: {rel}"));
    }
    Ok(canonical)
}

fn guess_mime_from_bytes(bytes: &[u8]) -> String {
    if bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
        "image/png".into()
    } else if bytes.starts_with(&[0xFF, 0xD8]) {
        "image/jpeg".into()
    } else if bytes.starts_with(b"RIFF") {
        "image/webp".into()
    } else {
        "image/png".into()
    }
}

fn guess_mime_from_path(path: &Path) -> Option<String> {
    match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "png" => Some("image/png".into()),
        "jpg" | "jpeg" => Some("image/jpeg".into()),
        "webp" => Some("image/webp".into()),
        "gif" => Some("image/gif".into()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn loads_inline_bytes_without_base_dir() {
        let image_ref = InlineImageRef {
            item_id: "x".into(),
            matched: String::new(),
            mime_type: "image/png".into(),
            bytes: vec![0x89, 0x50, 0x4E, 0x47],
            asset_path: None,
            start: 0,
            end: 0,
            caption: None,
            footnote: None,
        };
        let asset = resolve_image_asset(&image_ref, None).unwrap();
        assert_eq!(asset.mime_type, "image/png");
    }

    #[test]
    fn loads_asset_from_base_dir() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("assets/chart.png");
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        let mut f = std::fs::File::create(&file).unwrap();
        f.write_all(&[0x89, 0x50, 0x4E, 0x47]).unwrap();

        let image_ref = InlineImageRef {
            item_id: "im-1".into(),
            matched: String::new(),
            mime_type: String::new(),
            bytes: Vec::new(),
            asset_path: Some("assets/chart.png".into()),
            start: 0,
            end: 0,
            caption: None,
            footnote: None,
        };
        let asset = resolve_image_asset(&image_ref, Some(dir.path())).unwrap();
        assert!(!asset.bytes.is_empty());
    }
}
