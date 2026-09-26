//! Vision markdown assembly with page markers and optional drawing refs (SPEC-047 MV-20/21/24/28).

use std::collections::{BTreeMap, HashMap};

use crate::drawing_tags::{
    bind_figure_images_to_page_asset, caption_with_page_context, count_markdown_images_for_asset,
    dedupe_markdown_asset_images, finalize_page_asset_images, format_drawing_block,
    format_drawing_tag, format_inline_asset_image, inject_figure_local_images,
    insert_drawing_tag_after_first_image, is_drawing_eligible_asset_rel_path,
    markdown_has_durable_asset_image, page_chart_crop_rel_path, page_chart_drawing_item_id,
    page_drawing_item_id, page_figure_asset_rel_path, page_figure_drawing_item_id,
    EMPTY_VISION_PAGE_PLACEHOLDER,
};
use crate::embedded_images::WrittenFigureAsset;
use crate::page_marker::{PageMarkerWriter, PAGE_MARKER_PREFIX, PAGE_MARKER_SUFFIX};
use crate::region_assets::WrittenTableAsset;

fn page_marker(page_num: usize) -> String {
    PageMarkerWriter::write(page_num as u32)
}

/// Collect 1-indexed page numbers from `<!-- edgequake-page:N -->` markers.
pub fn page_numbers_from_markdown(markdown: &str) -> Vec<usize> {
    let mut pages = Vec::new();
    let mut rest = markdown;
    while let Some(idx) = rest.find(PAGE_MARKER_PREFIX) {
        let after = &rest[idx + PAGE_MARKER_PREFIX.len()..];
        if let Some(end) = after.find(PAGE_MARKER_SUFFIX) {
            if let Ok(n) = after[..end].trim().parse::<usize>() {
                if n > 0 && !pages.contains(&n) {
                    pages.push(n);
                }
            }
            rest = &after[end + PAGE_MARKER_SUFFIX.len()..];
        } else {
            break;
        }
    }
    if pages.is_empty() {
        pages.push(1);
    }
    pages.sort_unstable();
    pages
}

/// Enrich existing vision markdown so figure headings reference durable page assets.
///
/// First principle: only figure (`-fig-`) / chart (`-chart`) crops belong in the
/// markdown identity. Full-page `page-NNNN.png` is dual-pane PDF context, never
/// injected here. Does **not** emit `<drawing/>` (analyze is a separate stage).
///
/// When a page body already references a fig/chart asset, bind hallucinated
/// `![…](figN.png)` hrefs to that path. When a `Figure N` heading lacks an image
/// and no eligible asset is present in the body, prefer `page-NNNN-fig-01.png`
/// (callers that materialize assets should have written it).
pub fn enrich_markdown_with_viewer_assets(markdown: &str) -> String {
    if markdown.trim().is_empty() {
        return markdown.to_string();
    }
    let markers: Vec<(usize, usize)> = {
        let mut out = Vec::new();
        let mut search_from = 0;
        while let Some(rel) = markdown[search_from..].find(PAGE_MARKER_PREFIX) {
            let start = search_from + rel;
            let after_prefix = start + PAGE_MARKER_PREFIX.len();
            if let Some(suf) = markdown[after_prefix..].find(PAGE_MARKER_SUFFIX) {
                let num_str = markdown[after_prefix..after_prefix + suf].trim();
                if let Ok(page) = num_str.parse::<usize>() {
                    out.push((start, page));
                }
                search_from = after_prefix + suf + PAGE_MARKER_SUFFIX.len();
            } else {
                break;
            }
        }
        out
    };

    if markers.is_empty() {
        let rel = preferred_viewer_asset_rel(1, markdown);
        return enrich_page_body(1, markdown, &rel);
    }

    let mut enriched = String::with_capacity(markdown.len().saturating_add(256));
    if markers[0].0 > 0 {
        enriched.push_str(&markdown[..markers[0].0]);
    }
    for i in 0..markers.len() {
        let (start, page) = markers[i];
        let end = if i + 1 < markers.len() {
            markers[i + 1].0
        } else {
            markdown.len()
        };
        let chunk = &markdown[start..end];
        // Keep the marker line; enrich the body after it.
        let marker_line_end = chunk.find('\n').map(|n| n + 1).unwrap_or(chunk.len());
        let marker_part = &chunk[..marker_line_end];
        let body_raw = chunk[marker_line_end..].trim_end();
        let rel = preferred_viewer_asset_rel(page, body_raw);
        let body = enrich_page_body(page, body_raw, &rel);
        enriched.push_str(marker_part);
        enriched.push_str(body.trim_start());
        if i + 1 < markers.len() {
            enriched.push_str("\n\n");
        }
    }
    enriched
}

/// Prefer an existing fig/chart/table href in the body; never invent missing paths.
fn preferred_viewer_asset_rel(page: usize, body: &str) -> String {
    let _ = page;
    if let Some(existing) = first_drawing_eligible_href(body) {
        return existing;
    }
    // Never invent paths — missing files become broken images in the viewer.
    // Backend assemble / inject_on_disk writes real fig/table/chart assets when they exist.
    String::new()
}

fn first_drawing_eligible_href(markdown: &str) -> Option<String> {
    let mut i = 0;
    let bytes = markdown.as_bytes();
    while i < bytes.len() {
        if bytes[i] == b'!' && i + 1 < bytes.len() && bytes[i + 1] == b'[' {
            if let Some((end, _, url)) = crate::drawing_tags::parse_markdown_image_at(markdown, i) {
                if is_drawing_eligible_asset_rel_path(url) {
                    return Some(url.to_string());
                }
                i = end;
                continue;
            }
        }
        i += 1;
    }
    None
}

fn enrich_page_body(page: usize, body_raw: &str, rel: &str) -> String {
    let lower = body_raw.to_ascii_lowercase();
    // Never invent full-page viewer images for text-only pages.
    let should_inject = !rel.is_empty()
        && is_drawing_eligible_asset_rel_path(rel)
        && (lower.contains("figure ")
            || lower.contains("table ")
            || crate::chart_crop::page_markdown_suggests_chart(body_raw)
            || first_drawing_eligible_href(body_raw).is_some());
    let bound = bind_figure_images_to_page_asset(body_raw, rel);
    let mut body = if should_inject {
        inject_figure_local_images(&bound, rel)
    } else if first_drawing_eligible_href(body_raw).is_some() {
        // Still rebind hallucinated hrefs when an eligible target exists in-body.
        bound
    } else {
        body_raw.to_string()
    };
    if should_inject
        && !markdown_has_durable_asset_image(&body)
        && !body.trim().is_empty()
        && (lower.contains("figure ") || lower.contains("table "))
    {
        let alt = caption_with_page_context(page, &body, rel.contains("-chart"));
        body = format!("{}\n\n{}", format_inline_asset_image(&alt, rel), body);
    }
    if !rel.is_empty() {
        body = dedupe_markdown_asset_images(&body, rel);
    }
    body
}

/// Inject on-disk fig/table/chart assets into page bodies that mention them.
///
/// Used by include-from-pdf for already-converted docs. Never invents paths:
/// only writes markdown image refs when `{assets_root}/{rel}` exists.
pub fn inject_on_disk_region_assets(markdown: &str, assets_root: &std::path::Path) -> String {
    if markdown.trim().is_empty() {
        return markdown.to_string();
    }
    let discarded = crate::figure_filter::discarded_rel_paths_from_manifest(assets_root);
    let markdown = crate::figure_filter::strip_discarded_asset_lines(markdown, &discarded);
    if markdown.trim().is_empty() {
        return markdown;
    }
    let markers: Vec<(usize, usize)> = {
        let mut out = Vec::new();
        let mut search_from = 0;
        while let Some(rel) = markdown[search_from..].find(PAGE_MARKER_PREFIX) {
            let start = search_from + rel;
            let after_prefix = start + PAGE_MARKER_PREFIX.len();
            if let Some(suf) = markdown[after_prefix..].find(PAGE_MARKER_SUFFIX) {
                let num_str = markdown[after_prefix..after_prefix + suf].trim();
                if let Ok(page) = num_str.parse::<usize>() {
                    out.push((start, page));
                }
                search_from = after_prefix + suf + PAGE_MARKER_SUFFIX.len();
            } else {
                break;
            }
        }
        out
    };
    if markers.is_empty() {
        return inject_page_disk_assets(1, &markdown, assets_root, &discarded);
    }

    let mut out = String::with_capacity(markdown.len().saturating_add(256));
    if markers[0].0 > 0 {
        out.push_str(&markdown[..markers[0].0]);
    }
    for i in 0..markers.len() {
        let (start, page) = markers[i];
        let end = if i + 1 < markers.len() {
            markers[i + 1].0
        } else {
            markdown.len()
        };
        let chunk = &markdown[start..end];
        let marker_line_end = chunk.find('\n').map(|n| n + 1).unwrap_or(chunk.len());
        let marker_part = &chunk[..marker_line_end];
        let body_raw = chunk[marker_line_end..].trim_end();
        let body = inject_page_disk_assets(page, body_raw, assets_root, &discarded);
        out.push_str(marker_part);
        out.push_str(body.trim_start());
        if i + 1 < markers.len() {
            out.push_str("\n\n");
        }
    }
    out
}

fn inject_page_disk_assets(
    page: usize,
    body: &str,
    assets_root: &std::path::Path,
    discarded: &std::collections::HashSet<String>,
) -> String {
    // First principle: never keep markdown image hrefs that do not exist on disk.
    let body = strip_missing_on_disk_asset_images(body, assets_root);
    let lower = body.to_ascii_lowercase();
    let table_rel = crate::drawing_tags::page_table_asset_rel_path(page, 1);
    let fig_rel = page_figure_asset_rel_path(page, 1);
    let chart_rel = page_chart_crop_rel_path(page);
    let table_exists = assets_root.join(&table_rel).is_file() && !discarded.contains(&table_rel);
    let fig_exists = assets_root.join(&fig_rel).is_file() && !discarded.contains(&fig_rel);
    let chart_exists = assets_root.join(&chart_rel).is_file() && !discarded.contains(&chart_rel);
    let looks_like_table =
        lower.contains("table ") || body.lines().filter(|l| l.contains('|')).count() >= 3;

    // Prefer caption-anchored table crop over legacy chart PNGs.
    // W1-coexist (026): do NOT rewrite chart→fig — residual chart crops must
    // stay addressable so chart specialize can land numeric text alongside figs.
    let mut body = body;
    if table_exists && body.contains(&chart_rel) {
        body = rewrite_asset_hrefs(&body, &[&chart_rel], &table_rel);
    }

    if table_exists {
        if !markdown_has_durable_asset_image(&body) && looks_like_table {
            let alt = caption_with_page_context(page, &body, false);
            let injected = inject_figure_local_images(&body, &table_rel);
            if injected != body && markdown_has_durable_asset_image(&injected) {
                body = injected;
            } else {
                body = format!(
                    "{}\n\n{}",
                    format_inline_asset_image(&alt, &table_rel),
                    body
                );
            }
        } else if looks_like_table && !body.contains(&table_rel) {
            let injected = inject_figure_local_images(&body, &table_rel);
            if injected != body {
                body = injected;
            }
        }
    }

    if fig_exists
        && !body.contains(&fig_rel)
        && (lower.contains("figure ") || lower.contains("fig. "))
    {
        let injected = inject_figure_local_images(&body, &fig_rel);
        if injected != body {
            body = injected;
        } else if !body.contains(&fig_rel) {
            // Mention without a caption heading — still attach the real crop once.
            let alt = caption_with_page_context(page, &body, false);
            body = format!("{}\n\n{}", format_inline_asset_image(&alt, &fig_rel), body);
        }
    }

    if !markdown_has_durable_asset_image(&body) {
        let candidates: Vec<String> = if lower.contains("figure ") {
            let mut v = Vec::new();
            if fig_exists {
                v.push(fig_rel.clone());
            }
            if chart_exists {
                v.push(chart_rel.clone());
            }
            v
        } else if looks_like_table {
            let mut v = Vec::new();
            if table_exists {
                v.push(table_rel.clone());
            }
            v
        } else if crate::chart_crop::page_markdown_suggests_chart(&body) && chart_exists {
            vec![chart_rel.clone()]
        } else {
            Vec::new()
        };
        if let Some(rel) = candidates.into_iter().next() {
            let alt = caption_with_page_context(page, &body, rel.contains("-chart"));
            body = format!("{}\n\n{}", format_inline_asset_image(&alt, &rel), body);
        }
    }

    let mut rels: Vec<&str> = Vec::new();
    if fig_exists {
        rels.push(fig_rel.as_str());
    }
    if table_exists {
        rels.push(table_rel.as_str());
    }
    if chart_exists {
        rels.push(chart_rel.as_str());
    }
    finalize_page_asset_images(&body, &rels)
}

/// Drop `![…](assets/…)` lines whose target file is not on disk.
fn strip_missing_on_disk_asset_images(markdown: &str, assets_root: &std::path::Path) -> String {
    let mut out = String::with_capacity(markdown.len());
    for line in markdown.split_inclusive('\n') {
        let trimmed = line.trim();
        let keep = if let Some(start) = trimmed.find("](assets/") {
            let after = &trimmed[start + 2..]; // starts at "assets/..."
            if let Some(end) = after.find(')') {
                let rel = &after[..end];
                assets_root.join(rel).is_file()
            } else {
                true
            }
        } else {
            true
        };
        if keep {
            out.push_str(line);
        }
    }
    out
}

/// Replace known asset hrefs in markdown images / drawing tags with `to_rel`.
fn rewrite_asset_hrefs(markdown: &str, from_rels: &[&str], to_rel: &str) -> String {
    let mut out = markdown.to_string();
    for from in from_rels {
        if from.is_empty() || *from == to_rel {
            continue;
        }
        if out.contains(from) {
            out = out.replace(from, to_rel);
        }
    }
    out
}

/// One vision-converted page slice (1-indexed page number).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisionPageSlice {
    pub page_num: usize,
    pub markdown: String,
}

/// Build ordered page list covering `1..=total_pages`, filling gaps with empty markdown.
pub fn normalize_vision_pages(
    pages: &[VisionPageSlice],
    total_pages: usize,
    fallback_markdown: &str,
) -> Vec<VisionPageSlice> {
    let selected: Vec<usize> = (1..=total_pages.max(1)).collect();
    normalize_selected_vision_pages(pages, &selected, fallback_markdown)
}

/// Normalize only the pages owned by the current convert group.
///
/// Mixed modality groups must not synthesize placeholders for out-of-group
/// physical pages — those placeholders used to overwrite real content at stitch.
pub fn normalize_selected_vision_pages(
    pages: &[VisionPageSlice],
    selected_pages: &[usize],
    fallback_markdown: &str,
) -> Vec<VisionPageSlice> {
    let mut by_num: HashMap<usize, String> = pages
        .iter()
        .map(|p| (p.page_num, p.markdown.clone()))
        .collect();

    let mut selected: Vec<usize> = selected_pages.to_vec();
    selected.sort_unstable();
    selected.dedup();

    if selected.is_empty() {
        let mut nums: Vec<usize> = by_num.keys().copied().collect();
        nums.sort_unstable();
        if nums.is_empty() && !fallback_markdown.trim().is_empty() {
            return vec![VisionPageSlice {
                page_num: 1,
                markdown: fallback_markdown.trim().to_string(),
            }];
        }
        return nums
            .into_iter()
            .map(|page_num| VisionPageSlice {
                page_num,
                markdown: by_num.remove(&page_num).unwrap_or_default(),
            })
            .collect();
    }

    if by_num.is_empty() && !fallback_markdown.trim().is_empty() {
        by_num.insert(selected[0], fallback_markdown.trim().to_string());
    }

    selected
        .into_iter()
        .map(|page_num| VisionPageSlice {
            page_num,
            markdown: by_num.remove(&page_num).unwrap_or_default(),
        })
        .collect()
}

/// Assemble vision markdown: page markers, optional placeholder, optional `<drawing/>` per page.
///
/// `emit_drawing_refs=true` enables both viewer images and analyze tags (legacy).
pub fn assemble_vision_markdown(
    pages: &[VisionPageSlice],
    emit_drawing_refs: bool,
    id_prefix: Option<&str>,
) -> String {
    assemble_vision_markdown_with_policy(
        pages,
        emit_drawing_refs,
        emit_drawing_refs,
        id_prefix,
        None,
        None,
        None,
        false,
    )
}

/// Same as [`assemble_vision_markdown`] with optional per-page drawing asset overrides.
pub fn assemble_vision_markdown_with_overrides(
    pages: &[VisionPageSlice],
    emit_drawing_refs: bool,
    id_prefix: Option<&str>,
    drawing_path_overrides: Option<&HashMap<usize, String>>,
) -> String {
    assemble_vision_markdown_with_policy(
        pages,
        emit_drawing_refs,
        emit_drawing_refs,
        id_prefix,
        drawing_path_overrides,
        None,
        None,
        false,
    )
}

/// Viewer-first layout (MV-28) with optional analyze tags.
pub fn assemble_vision_markdown_with_options(
    pages: &[VisionPageSlice],
    emit_viewer_images: bool,
    emit_analyze_tags: bool,
    id_prefix: Option<&str>,
    drawing_path_overrides: Option<&HashMap<usize, String>>,
) -> String {
    assemble_vision_markdown_with_policy(
        pages,
        emit_viewer_images,
        emit_analyze_tags,
        id_prefix,
        drawing_path_overrides,
        None,
        None,
        false,
    )
}

/// Full assembly: viewer assets + figure-bounded analyze drawings (SPEC-047).
///
/// Analyze `<drawing/>` paths prefer embedded ImageXObjects (`page-NNNN-fig-MM.png`).
/// Full-page `page-NNNN.png` is viewer-only and is never used as the VLM analyze target
/// when figure assets (or chart crops) are available.
pub fn assemble_vision_markdown_with_figures(
    pages: &[VisionPageSlice],
    emit_viewer_images: bool,
    emit_analyze_tags: bool,
    id_prefix: Option<&str>,
    drawing_path_overrides: Option<&HashMap<usize, String>>,
    figures_by_page: Option<&HashMap<usize, Vec<WrittenFigureAsset>>>,
    tables_by_page: Option<&HashMap<usize, Vec<WrittenTableAsset>>>,
) -> String {
    assemble_vision_markdown_with_policy(
        pages,
        emit_viewer_images,
        emit_analyze_tags,
        id_prefix,
        drawing_path_overrides,
        figures_by_page,
        tables_by_page,
        false,
    )
}

/// SPEC-134 Slice E / LAW-134-20: `page_as_unit` suppresses fig/chart/table
/// inject and `<drawing/>` tiles (full-page raster is the VLM unit). Empty
/// Pass-A bodies never grow fragment hrefs either (empty-page retry).
#[allow(clippy::too_many_arguments)]
pub fn assemble_vision_markdown_with_policy(
    pages: &[VisionPageSlice],
    emit_viewer_images: bool,
    emit_analyze_tags: bool,
    id_prefix: Option<&str>,
    drawing_path_overrides: Option<&HashMap<usize, String>>,
    figures_by_page: Option<&HashMap<usize, Vec<WrittenFigureAsset>>>,
    tables_by_page: Option<&HashMap<usize, Vec<WrittenTableAsset>>>,
    page_as_unit: bool,
) -> String {
    let mut parts = Vec::with_capacity(pages.len());
    for page in pages {
        let mut section = page_marker(page.page_num);
        section.push('\n');

        let page_figures = figures_by_page
            .and_then(|m| m.get(&page.page_num))
            .cloned()
            .unwrap_or_default();
        let page_tables = tables_by_page
            .and_then(|m| m.get(&page.page_num))
            .cloned()
            .unwrap_or_default();
        let override_path = drawing_path_overrides.and_then(|m| m.get(&page.page_num));
        let is_chart_crop = override_path
            .is_some_and(|p| p.contains("-chart") && is_drawing_eligible_asset_rel_path(p));

        // Viewer: figure → table → chart crop — never full-page PNG.
        // Chart override stays available even when figs exist (W1-coexist).
        let viewer_rel: Option<String> = page_figures
            .first()
            .map(|f| f.rel_path.clone())
            .or_else(|| page_tables.first().map(|t| t.rel_path.clone()))
            .or_else(|| {
                override_path
                    .filter(|p| is_drawing_eligible_asset_rel_path(p))
                    .cloned()
            });
        let chart_override_rel: Option<&str> = override_path
            .filter(|p| p.contains("-chart") && is_drawing_eligible_asset_rel_path(p))
            .map(|s| s.as_str());

        let raw_body = page.markdown.trim();
        let suppress_fragments = page_as_unit || raw_body.is_empty();
        let mut body = if raw_body.is_empty() {
            EMPTY_VISION_PAGE_PLACEHOLDER.to_string()
        } else if emit_viewer_images && !suppress_fragments {
            let bind_rel = viewer_rel.as_deref().unwrap_or("");
            let mut b = if bind_rel.is_empty() {
                raw_body.to_string()
            } else {
                bind_figure_images_to_page_asset(raw_body, bind_rel)
            };
            // Inject each real crop next to its caption (figure vs table).
            for fig in &page_figures {
                b = inject_figure_local_images(&b, &fig.rel_path);
            }
            for table in &page_tables {
                b = inject_figure_local_images(&b, &table.rel_path);
            }
            if page_figures.is_empty() && page_tables.is_empty() {
                if let Some(ref rel) = viewer_rel {
                    b = inject_figure_local_images(&b, rel);
                }
            } else if page_tables.is_empty() {
                // W1-coexist: residual chart crop alongside figs (tables still win).
                if let Some(chart_rel) = chart_override_rel {
                    if !b.contains(chart_rel) {
                        let alt = caption_with_page_context(page.page_num, &b, true);
                        b = format!("{}\n\n{}", format_inline_asset_image(&alt, chart_rel), b);
                    }
                }
            }
            b
        } else {
            raw_body.to_string()
        };

        if emit_viewer_images && !suppress_fragments {
            if let Some(ref rel) = viewer_rel {
                if !markdown_has_durable_asset_image(&body) {
                    let alt = caption_with_page_context(page.page_num, &body, is_chart_crop);
                    body = format!("{}\n\n{}", format_inline_asset_image(&alt, rel), body);
                }
            }
            // G-prune SSOT: every crop still in figure_map/table_map must appear as an href.
            for fig in &page_figures {
                if count_markdown_images_for_asset(&body, &fig.rel_path) == 0 {
                    let alt = caption_with_page_context(page.page_num, &body, false);
                    body = format!(
                        "{}\n\n{}",
                        body,
                        format_inline_asset_image(&alt, &fig.rel_path)
                    );
                }
            }
            for table in &page_tables {
                if count_markdown_images_for_asset(&body, &table.rel_path) == 0 {
                    let alt = if table.label.is_empty() {
                        caption_with_page_context(page.page_num, &body, false)
                    } else {
                        table.label.clone()
                    };
                    body = format!(
                        "{}\n\n{}",
                        body,
                        format_inline_asset_image(&alt, &table.rel_path)
                    );
                }
            }
        }

        let mut dedupe_rels: Vec<&str> = page_figures
            .iter()
            .map(|f| f.rel_path.as_str())
            .chain(page_tables.iter().map(|t| t.rel_path.as_str()))
            .collect();
        if let Some(ref rel) = viewer_rel {
            if !dedupe_rels.contains(&rel.as_str()) {
                dedupe_rels.push(rel.as_str());
            }
        }
        if let Some(chart_rel) = chart_override_rel {
            // Tables own residual policy — do not keep chart in dedupe set.
            if page_tables.is_empty() && !dedupe_rels.contains(&chart_rel) {
                dedupe_rels.push(chart_rel);
            }
        }
        body = finalize_page_asset_images(&body, &dedupe_rels);

        if emit_analyze_tags && !suppress_fragments {
            if !page_figures.is_empty() {
                // One drawing per embedded ImageXObject — never the full page.
                for (i, fig) in page_figures.iter().enumerate() {
                    let item_id = page_figure_drawing_item_id(page.page_num, fig.index, id_prefix);
                    let caption = caption_with_page_context(page.page_num, &body, false);
                    if i == 0 {
                        if let Some(with_tag) = insert_drawing_tag_after_first_image(
                            &body,
                            &item_id,
                            &fig.rel_path,
                            Some(caption.as_str()),
                        ) {
                            body = with_tag;
                        } else {
                            body = format!(
                                "{}\n\n{}",
                                format_drawing_block(
                                    &item_id,
                                    &fig.rel_path,
                                    Some(caption.as_str()),
                                ),
                                body
                            );
                        }
                    } else {
                        body.push_str("\n\n");
                        body.push_str(&format_drawing_tag(
                            &item_id,
                            &fig.rel_path,
                            Some(caption.as_str()),
                        ));
                    }
                }
                for table in &page_tables {
                    let item_id = page_drawing_item_id(page.page_num, id_prefix);
                    body.push_str("\n\n");
                    body.push_str(&format_drawing_tag(
                        &item_id,
                        &table.rel_path,
                        Some(table.label.as_str()),
                    ));
                }
                // W1-coexist (026): residual chart crop must also be analyzed when
                // figs are present — otherwise crop-expand writes are dead for Acc.
                if let Some(crop_rel) = chart_override_rel {
                    // Tables still own the page for residual policy; skip chart
                    // analyze when a real table crop is already bound.
                    if page_tables.is_empty() {
                        let item_id = page_chart_drawing_item_id(page.page_num, id_prefix);
                        let caption = caption_with_page_context(page.page_num, &body, true);
                        body.push_str("\n\n");
                        body.push_str(&format_drawing_tag(
                            &item_id,
                            crop_rel,
                            Some(caption.as_str()),
                        ));
                    }
                }
                section.push_str(&body);
            } else if !page_tables.is_empty() {
                for (i, table) in page_tables.iter().enumerate() {
                    let item_id = page_drawing_item_id(page.page_num, id_prefix);
                    if i == 0 {
                        if let Some(with_tag) = insert_drawing_tag_after_first_image(
                            &body,
                            &item_id,
                            &table.rel_path,
                            Some(table.label.as_str()),
                        ) {
                            body = with_tag;
                        } else {
                            body = format!(
                                "{}\n\n{}",
                                format_drawing_block(
                                    &item_id,
                                    &table.rel_path,
                                    Some(table.label.as_str()),
                                ),
                                body
                            );
                        }
                    } else {
                        body.push_str("\n\n");
                        body.push_str(&format_drawing_tag(
                            &item_id,
                            &table.rel_path,
                            Some(table.label.as_str()),
                        ));
                    }
                }
                section.push_str(&body);
            } else if let Some(crop_rel) =
                override_path.filter(|p| is_drawing_eligible_asset_rel_path(p))
            {
                // Vector/chart page without ImageXObject: ink-cropped page region.
                let item_id = page_chart_drawing_item_id(page.page_num, id_prefix);
                let caption = caption_with_page_context(page.page_num, &body, true);
                if let Some(with_tag) = insert_drawing_tag_after_first_image(
                    &body,
                    &item_id,
                    crop_rel,
                    Some(caption.as_str()),
                ) {
                    section.push_str(&with_tag);
                } else {
                    section.push_str(&format_drawing_block(
                        &item_id,
                        crop_rel,
                        Some(caption.as_str()),
                    ));
                    section.push_str("\n\n");
                    section.push_str(&body);
                }
            } else {
                // No figure object and no crop: do not feed full-page raster to VLM.
                section.push_str(&body);
            }
        } else {
            section.push_str(&body);
        }
        parts.push(section);
    }
    parts.join("\n\n")
}

/// Merge per-group Vision markdown by `<!-- edgequake-page:N -->` order.
///
/// Homogeneous converts (`parts.len() <= 1`) are returned unchanged so trailing
/// crop-coverage comments stay byte-identical on the print Acc path.
///
/// When the same physical page appears in more than one part, prefer non-empty
/// real content over empty/placeholder sections so later groups cannot erase
/// earlier OCR.
pub fn stitch_page_markdown_in_order(parts: &[String]) -> String {
    if parts.len() <= 1 {
        return parts.first().cloned().unwrap_or_default();
    }
    let mut by_page: BTreeMap<usize, String> = BTreeMap::new();
    let mut tails = Vec::new();
    for part in parts {
        let sections = split_marked_page_sections(part);
        if sections.is_empty() {
            if !part.trim().is_empty() {
                tails.push(part.clone());
            }
            continue;
        }
        for (num, section) in sections {
            match by_page.get(&num) {
                Some(existing) => {
                    by_page.insert(num, prefer_page_section(existing, &section));
                }
                None => {
                    by_page.insert(num, section);
                }
            }
        }
    }
    let mut out = by_page.into_values().collect::<Vec<_>>().join("\n\n");
    for tail in tails {
        if !out.is_empty() {
            out.push_str("\n\n");
        }
        out.push_str(&tail);
    }
    out
}

fn section_body_is_empty_or_placeholder(section: &str) -> bool {
    let body = match section.find('\n') {
        Some(idx) => section[idx + 1..].trim(),
        None => "",
    };
    body.is_empty() || body == EMPTY_VISION_PAGE_PLACEHOLDER
}

fn prefer_page_section(existing: &str, incoming: &str) -> String {
    let existing_empty = section_body_is_empty_or_placeholder(existing);
    let incoming_empty = section_body_is_empty_or_placeholder(incoming);
    match (existing_empty, incoming_empty) {
        (false, true) => existing.to_string(),
        (true, false) => incoming.to_string(),
        // Both real or both empty: keep first (stable, one section per page).
        _ => existing.to_string(),
    }
}

fn split_marked_page_sections(markdown: &str) -> Vec<(usize, String)> {
    let mut starts: Vec<(usize, usize)> = Vec::new();
    let mut rest_idx = 0usize;
    while let Some(rel) = markdown[rest_idx..].find(PAGE_MARKER_PREFIX) {
        let idx = rest_idx + rel;
        let after = &markdown[idx + PAGE_MARKER_PREFIX.len()..];
        if let Some(end) = after.find(PAGE_MARKER_SUFFIX) {
            if let Ok(n) = after[..end].trim().parse::<usize>() {
                if n > 0 {
                    starts.push((idx, n));
                }
            }
            rest_idx = idx + PAGE_MARKER_PREFIX.len() + end + PAGE_MARKER_SUFFIX.len();
        } else {
            break;
        }
    }
    let mut out = Vec::with_capacity(starts.len());
    for (i, (start, num)) in starts.iter().enumerate() {
        let end = starts.get(i + 1).map(|(s, _)| *s).unwrap_or(markdown.len());
        out.push((*num, markdown[*start..end].trim_end().to_string()));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inline_images::scan_inline_image_refs;

    #[test]
    fn empty_page_keeps_marker_and_placeholder() {
        let pages = vec![VisionPageSlice {
            page_num: 3,
            markdown: String::new(),
        }];
        let md = assemble_vision_markdown(&pages, false, None);
        assert!(md.contains("<!-- edgequake-page:3 -->"));
        assert!(md.contains(EMPTY_VISION_PAGE_PLACEHOLDER));
    }

    #[test]
    fn page_as_unit_does_not_inject_figures_on_transcribed_page() {
        let pages = vec![VisionPageSlice {
            page_num: 1,
            markdown: "Handwritten title".into(),
        }];
        let mut figs = HashMap::new();
        figs.insert(
            1,
            vec![crate::embedded_images::WrittenFigureAsset {
                page_num: 1,
                index: 1,
                rel_path: "assets/page-0001-fig-01.png".into(),
                width: 40,
                height: 30,
                bbox: None,
            }],
        );
        let md = assemble_vision_markdown_with_policy(
            &pages,
            true,
            true,
            Some("ms"),
            None,
            Some(&figs),
            None,
            true,
        );
        assert!(md.contains("Handwritten title"));
        assert!(!md.contains("page-0001-fig-01.png"));
        assert!(!md.contains("<drawing"));
    }

    #[test]
    fn stitch_orders_mixed_convert_groups() {
        let print = "<!-- edgequake-page:2 -->\n\nPrint body\n".to_string();
        let ms = "<!-- edgequake-page:1 -->\n\nMS body\n".to_string();
        let out = stitch_page_markdown_in_order(&[print, ms]);
        assert!(out.find("MS body").unwrap() < out.find("Print body").unwrap());
        assert!(out.contains("<!-- edgequake-page:1 -->"));
        assert!(out.contains("<!-- edgequake-page:2 -->"));
    }

    #[test]
    fn stitch_single_part_is_identity() {
        let one = "<!-- edgequake-page:1 -->\n\nHello\n\n<!-- edgequake-crop-coverage: x -->\n";
        assert_eq!(stitch_page_markdown_in_order(&[one.to_string()]), one);
    }

    #[test]
    fn drawing_refs_are_scannable() {
        let pages = vec![
            VisionPageSlice {
                page_num: 1,
                markdown: "Revenue chart".into(),
            },
            VisionPageSlice {
                page_num: 2,
                markdown: String::new(),
            },
        ];
        let mut figs = HashMap::new();
        figs.insert(
            1,
            vec![crate::embedded_images::WrittenFigureAsset {
                page_num: 1,
                index: 1,
                rel_path: "assets/page-0001-fig-01.png".into(),
                width: 40,
                height: 30,
                bbox: None,
            }],
        );
        figs.insert(
            2,
            vec![crate::embedded_images::WrittenFigureAsset {
                page_num: 2,
                index: 1,
                rel_path: "assets/page-0002-fig-01.png".into(),
                width: 40,
                height: 30,
                bbox: None,
            }],
        );
        let md = assemble_vision_markdown_with_figures(
            &pages,
            true,
            true,
            Some("doc-1"),
            None,
            Some(&figs),
            None,
        );
        let refs = scan_inline_image_refs(&md);
        assert_eq!(refs.len(), 1, "empty Pass-A page must not get a fig href");
        assert_eq!(
            refs[0].asset_path.as_deref(),
            Some("assets/page-0001-fig-01.png")
        );
        // Viewer image precedes body text.
        assert!(md.contains("!["));
        assert!(md.find("![").unwrap() < md.find("Revenue chart").unwrap());
        assert!(md.contains(EMPTY_VISION_PAGE_PLACEHOLDER));
        assert!(!md.contains("page-0002-fig-01.png"));
    }

    #[test]
    fn chart_crop_override_points_drawing_at_crop_asset() {
        let pages = vec![VisionPageSlice {
            page_num: 1,
            markdown: "Revenue chart 12%".into(),
        }];
        let mut overrides = HashMap::new();
        overrides.insert(1usize, "assets/page-0001-chart.png".into());
        let md =
            assemble_vision_markdown_with_overrides(&pages, true, Some("doc"), Some(&overrides));
        let refs = scan_inline_image_refs(&md);
        assert_eq!(refs.len(), 1);
        assert_eq!(
            refs[0].asset_path.as_deref(),
            Some("assets/page-0001-chart.png")
        );
    }

    #[test]
    fn hallucinated_figure_image_binds_to_page_asset_once() {
        let pages = vec![VisionPageSlice {
            page_num: 1,
            markdown: "![Figure 1. Overview of the MMMU dataset](fig1.png)\n\nAbstract".into(),
        }];
        let mut figs = HashMap::new();
        figs.insert(
            1,
            vec![crate::embedded_images::WrittenFigureAsset {
                page_num: 1,
                index: 1,
                rel_path: "assets/page-0001-fig-01.png".into(),
                width: 40,
                height: 30,
                bbox: None,
            }],
        );
        let md = assemble_vision_markdown_with_figures(
            &pages,
            true,
            true,
            Some("mmmu"),
            None,
            Some(&figs),
            None,
        );
        assert!(
            md.contains("![Figure 1. Overview of the MMMU dataset](assets/page-0001-fig-01.png)")
        );
        assert!(!md.contains("(fig1.png)"));
        assert_eq!(md.matches("](assets/page-0001-fig-01.png)").count(), 1);
        assert!(md.contains("<drawing"));
        assert!(md.contains("path=\"assets/page-0001-fig-01.png\""));
    }

    #[test]
    fn pass_a_image_before_figure_caption_emits_single_viewer_image() {
        let pages = vec![VisionPageSlice {
            page_num: 3,
            markdown: r#"# 3 COLLEAGUE.SKILL System Overview

![COLLEAGUE.SKILL Expert Distillation Pipeline](fig1.png)

COLLEAGUE.SKILL Expert Distillation Pipeline

Figure 1: COLLEAGUE.SKILL architecture for automated person-grounded skill generation.
"#
            .into(),
        }];
        let mut figs = HashMap::new();
        figs.insert(
            3,
            vec![crate::embedded_images::WrittenFigureAsset {
                page_num: 3,
                index: 1,
                rel_path: "assets/page-0003-fig-01.png".into(),
                width: 400,
                height: 200,
                bbox: None,
            }],
        );
        let md = assemble_vision_markdown_with_figures(
            &pages,
            true,
            true,
            Some("colleague-doc"),
            None,
            Some(&figs),
            None,
        );
        assert_eq!(
            md.matches("](assets/page-0003-fig-01.png)").count(),
            1,
            "figure asset must appear exactly once: {md}"
        );
        assert!(!md.contains("(fig1.png)"));
        assert!(md.contains("Figure 1: COLLEAGUE.SKILL architecture"));
        assert!(md.contains("<drawing"));
    }

    #[test]
    fn bind_multiple_hallucinations_deduped_to_one_figure_image() {
        let pages = vec![VisionPageSlice {
            page_num: 1,
            markdown: "![Top](fig1.png)\n\n![Side](fig2.png)\n\nFigure 1: Overview\n".into(),
        }];
        let mut figs = HashMap::new();
        figs.insert(
            1,
            vec![crate::embedded_images::WrittenFigureAsset {
                page_num: 1,
                index: 1,
                rel_path: "assets/page-0001-fig-01.png".into(),
                width: 40,
                height: 30,
                bbox: None,
            }],
        );
        let md = assemble_vision_markdown_with_figures(
            &pages,
            true,
            false,
            Some("doc"),
            None,
            Some(&figs),
            None,
        );
        assert_eq!(
            md.matches("](assets/page-0001-fig-01.png)").count(),
            1,
            "{md}"
        );
    }

    #[test]
    fn figure_heading_gets_local_image_before_caption() {
        let pages = vec![VisionPageSlice {
            page_num: 1,
            markdown:
                "# Title\n\n## Figure 1: Autodata pipeline\n\nThe framework employs an agent.\n"
                    .into(),
        }];
        let mut figs = HashMap::new();
        figs.insert(
            1,
            vec![crate::embedded_images::WrittenFigureAsset {
                page_num: 1,
                index: 1,
                rel_path: "assets/page-0001-fig-01.png".into(),
                width: 40,
                height: 30,
                bbox: None,
            }],
        );
        let md = assemble_vision_markdown_with_figures(
            &pages,
            true,
            true,
            Some("doc"),
            None,
            Some(&figs),
            None,
        );
        let fig_pos = md.find("## Figure 1: Autodata pipeline").unwrap();
        let img_pos = md
            .find("![Figure 1: Autodata pipeline](assets/page-0001-fig-01.png)")
            .unwrap();
        let caption_pos = md.find("The framework employs").unwrap();
        assert!(fig_pos < img_pos);
        assert!(img_pos < caption_pos);
        let draw_pos = md.find("<drawing").unwrap();
        assert!(img_pos < draw_pos);
        assert!(draw_pos < caption_pos);
        assert_eq!(md.matches("](assets/page-0001-fig-01.png)").count(), 1);
        assert!(md.contains("path=\"assets/page-0001-fig-01.png\""));
    }

    #[test]
    fn viewer_images_without_analyze_tags_omit_drawing() {
        let pages = vec![VisionPageSlice {
            page_num: 1,
            markdown: "## Figure 1: Demo\n\nCaption text.".into(),
        }];
        let mut figs = HashMap::new();
        figs.insert(
            1,
            vec![crate::embedded_images::WrittenFigureAsset {
                page_num: 1,
                index: 1,
                rel_path: "assets/page-0001-fig-01.png".into(),
                width: 40,
                height: 30,
                bbox: None,
            }],
        );
        let md = assemble_vision_markdown_with_figures(
            &pages,
            true,
            false,
            Some("doc"),
            None,
            Some(&figs),
            None,
        );
        assert!(md.contains("![Figure 1: Demo](assets/page-0001-fig-01.png)"));
        assert!(!md.contains("<drawing"));
        assert!(!md.contains("assets/page-0001.png)"));
    }

    #[test]
    fn text_only_page_emits_no_viewer_image() {
        let pages = vec![VisionPageSlice {
            page_num: 1,
            markdown: "# Title\n\nAbstract with 15.3% improvement.".into(),
        }];
        let md = assemble_vision_markdown_with_figures(
            &pages,
            true,
            true,
            Some("doc"),
            None,
            None,
            None,
        );
        assert!(!md.contains("!["));
        assert!(!md.contains("<drawing"));
        assert!(!md.contains("assets/page-0001.png"));
    }

    #[test]
    fn enrich_does_not_invent_missing_figure_paths() {
        let md = "<!-- edgequake-page:1 -->\n# Title\n\n## Figure 1: Autodata pipeline\n\nThe framework employs an agent.\n";
        let out = enrich_markdown_with_viewer_assets(md);
        assert!(!out.contains("!["));
        assert!(!out.contains("assets/page-0001"));
        assert!(!out.contains("<drawing"));
    }

    #[test]
    fn enrich_keeps_existing_fig_href() {
        let md = "<!-- edgequake-page:1 -->\n![Figure 1: Autodata pipeline](assets/page-0001-fig-01.png)\n\nThe framework employs an agent.\n";
        let out = enrich_markdown_with_viewer_assets(md);
        assert!(out.contains("![Figure 1: Autodata pipeline](assets/page-0001-fig-01.png)"));
    }

    #[test]
    fn table_heading_gets_table_crop_not_full_page() {
        let pages = vec![VisionPageSlice {
            page_num: 6,
            markdown: "## Table 1: Pass rates\n\nDiscussion text.\n".into(),
        }];
        let mut tables = HashMap::new();
        tables.insert(
            6,
            vec![WrittenTableAsset {
                page_num: 6,
                index: 1,
                rel_path: "assets/page-0006-table-01.png".into(),
                width: 800,
                height: 200,
                label: "Table 1".into(),
                bbox: None,
            }],
        );
        let md = assemble_vision_markdown_with_figures(
            &pages,
            true,
            true,
            Some("doc"),
            None,
            None,
            Some(&tables),
        );
        assert!(md.contains("![Table 1: Pass rates](assets/page-0006-table-01.png)"));
        assert!(md.contains("path=\"assets/page-0006-table-01.png\""));
        assert!(!md.contains("assets/page-0006.png)"));
        assert!(!md.contains("-chart.png"));
    }

    #[test]
    fn inject_on_disk_rewrites_chart_to_table_crop() {
        let dir = tempfile::tempdir().unwrap();
        let assets = dir.path().join("assets");
        std::fs::create_dir_all(&assets).unwrap();
        // Minimal valid 1x1 PNG
        let png = {
            use image::{ImageBuffer, ImageFormat, Rgba};
            let img: ImageBuffer<Rgba<u8>, _> = ImageBuffer::from_pixel(1, 1, Rgba([0, 0, 0, 255]));
            let mut buf = std::io::Cursor::new(Vec::new());
            img.write_to(&mut buf, ImageFormat::Png).unwrap();
            buf.into_inner()
        };
        std::fs::write(assets.join("page-0006-table-01.png"), &png).unwrap();
        std::fs::write(assets.join("page-0006-chart.png"), &png).unwrap();
        let md =
            "<!-- edgequake-page:6 -->\n![Table 1](assets/page-0006-chart.png)\n\n## Pass rates\n";
        let out = inject_on_disk_region_assets(md, dir.path());
        assert!(
            out.contains("assets/page-0006-table-01.png"),
            "expected table crop rewrite, got:\n{out}"
        );
        assert!(!out.contains("assets/page-0006-chart.png"));
    }

    #[test]
    fn inject_on_disk_strips_missing_fig_hrefs() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("assets")).unwrap();
        let md =
            "<!-- edgequake-page:3 -->\n![Figure 1](assets/page-0003-fig-01.png)\n\nBody text.\n";
        let out = inject_on_disk_region_assets(md, dir.path());
        assert!(
            !out.contains("assets/page-0003-fig-01.png"),
            "missing fig href must be stripped, got:\n{out}"
        );
        assert!(out.contains("Body text"));
    }

    #[test]
    fn inject_on_disk_strips_discarded_manifest_hrefs() {
        let dir = tempfile::tempdir().unwrap();
        let assets = dir.path().join("assets");
        std::fs::create_dir_all(&assets).unwrap();
        let png = {
            use image::{ImageBuffer, ImageFormat, Rgba};
            let img: ImageBuffer<Rgba<u8>, _> = ImageBuffer::from_pixel(1, 1, Rgba([0, 0, 0, 255]));
            let mut buf = std::io::Cursor::new(Vec::new());
            img.write_to(&mut buf, ImageFormat::Png).unwrap();
            buf.into_inner()
        };
        std::fs::write(assets.join("page-0001-fig-01.png"), &png).unwrap();
        std::fs::write(assets.join("page-0001-fig-02.png"), &png).unwrap();
        crate::figure_filter::write_manifest(
            dir.path(),
            &[
                crate::figure_filter::FigureFilterResult {
                    rel_path: "assets/page-0001-fig-01.png".into(),
                    page_num: 1,
                    label: String::new(),
                    kind: crate::figure_filter::FigureKind::Diagram,
                    is_figure: true,
                    description: "kept".into(),
                },
                crate::figure_filter::FigureFilterResult {
                    rel_path: "assets/page-0001-fig-02.png".into(),
                    page_num: 1,
                    label: String::new(),
                    kind: crate::figure_filter::FigureKind::Logo,
                    is_figure: false,
                    description: String::new(),
                },
            ],
        )
        .unwrap();
        let md = "<!-- edgequake-page:1 -->\n![Figure](assets/page-0001-fig-01.png)\n![Logo](assets/page-0001-fig-02.png)\n";
        let out = inject_on_disk_region_assets(md, dir.path());
        assert!(out.contains("assets/page-0001-fig-01.png"));
        assert!(
            !out.contains("assets/page-0001-fig-02.png"),
            "discarded logo must not be re-injected, got:\n{out}"
        );
    }

    #[test]
    fn enrich_skips_text_only_pages() {
        let md = "<!-- edgequake-page:1 -->\n# Learning the ARTS\n\n15.3% relative improvement.\n";
        let out = enrich_markdown_with_viewer_assets(md);
        assert!(!out.contains("!["));
        assert!(!out.contains("assets/page-0001.png"));
    }

    #[test]
    fn page_numbers_from_markers() {
        let md = "<!-- edgequake-page:1 -->\na\n\n<!-- edgequake-page:3 -->\nb";
        assert_eq!(page_numbers_from_markdown(md), vec![1, 3]);
    }

    #[test]
    fn assemble_emits_both_fig_and_table_on_same_page() {
        let pages = vec![VisionPageSlice {
            page_num: 7,
            markdown: "## Figure 2: Loop\n\n## Table 2: Results\n\nText.\n".into(),
        }];
        let mut figs = HashMap::new();
        figs.insert(
            7,
            vec![crate::embedded_images::WrittenFigureAsset {
                page_num: 7,
                index: 1,
                rel_path: "assets/page-0007-fig-01.png".into(),
                width: 40,
                height: 30,
                bbox: None,
            }],
        );
        let mut tables = HashMap::new();
        tables.insert(
            7,
            vec![WrittenTableAsset {
                page_num: 7,
                index: 1,
                rel_path: "assets/page-0007-table-01.png".into(),
                width: 80,
                height: 40,
                label: "Table 2".into(),
                bbox: None,
            }],
        );
        let md = assemble_vision_markdown_with_figures(
            &pages,
            true,
            true,
            Some("doc"),
            None,
            Some(&figs),
            Some(&tables),
        );
        assert!(md.contains("assets/page-0007-fig-01.png"));
        assert!(md.contains("assets/page-0007-table-01.png"));
        assert!(!md.contains("assets/page-0007.png)"));
        assert!(!md.contains("-chart.png"));
        assert!(md.contains("path=\"assets/page-0007-fig-01.png\""));
        assert!(md.contains("path=\"assets/page-0007-table-01.png\""));
    }

    #[test]
    fn assemble_emits_chart_alongside_fig_override() {
        // 026 W1-coexist: residual chart crop must get a drawing tag even when
        // an embedded fig exists (crop-expand otherwise writes dead assets).
        let pages = vec![VisionPageSlice {
            page_num: 1,
            markdown: "## Figure 1: Overview\n\nBody.\n".into(),
        }];
        let mut figs = HashMap::new();
        figs.insert(
            1,
            vec![crate::embedded_images::WrittenFigureAsset {
                page_num: 1,
                index: 1,
                rel_path: "assets/page-0001-fig-01.png".into(),
                width: 40,
                height: 30,
                bbox: None,
            }],
        );
        let mut overrides = HashMap::new();
        overrides.insert(1usize, "assets/page-0001-chart.png".into());
        let md = assemble_vision_markdown_with_figures(
            &pages,
            true,
            true,
            Some("doc"),
            Some(&overrides),
            Some(&figs),
            None,
        );
        assert!(md.contains("assets/page-0001-fig-01.png"));
        assert!(
            md.contains("assets/page-0001-chart.png"),
            "chart crop must coexist with fig: {md}"
        );
        assert!(
            md.contains("im-doc-page-0001-chart") || md.contains("page-0001-chart"),
            "chart drawing item id must be present: {md}"
        );
    }

    #[test]
    fn inject_keeps_chart_alongside_fig() {
        let dir = tempfile::tempdir().unwrap();
        let assets = dir.path().join("assets");
        std::fs::create_dir_all(&assets).unwrap();
        // Minimal placeholders — inject only checks is_file().
        std::fs::write(assets.join("page-0001-fig-01.png"), b"\x89PNG").unwrap();
        std::fs::write(assets.join("page-0001-chart.png"), b"\x89PNG").unwrap();
        let md = "<!-- edgequake-page:1 -->\n![Figure 1](assets/page-0001-fig-01.png)\n\n![Chart](assets/page-0001-chart.png)\n\nBody.\n";
        let out = inject_on_disk_region_assets(md, dir.path());
        assert!(
            out.contains("assets/page-0001-fig-01.png"),
            "fig kept: {out}"
        );
        assert!(
            out.contains("assets/page-0001-chart.png"),
            "chart must not be rewritten to fig: {out}"
        );
    }

    #[test]
    fn assemble_table_blocks_chart_override() {
        let pages = vec![VisionPageSlice {
            page_num: 6,
            markdown: "## Table 1: Rates\n\nBody.\n".into(),
        }];
        let mut tables = HashMap::new();
        tables.insert(
            6,
            vec![WrittenTableAsset {
                page_num: 6,
                index: 1,
                rel_path: "assets/page-0006-table-01.png".into(),
                width: 80,
                height: 40,
                label: "Table 1".into(),
                bbox: None,
            }],
        );
        let mut overrides = HashMap::new();
        overrides.insert(6usize, "assets/page-0006-chart.png".into());
        let md = assemble_vision_markdown_with_figures(
            &pages,
            true,
            true,
            Some("doc"),
            Some(&overrides),
            None,
            Some(&tables),
        );
        assert!(md.contains("assets/page-0006-table-01.png"));
        assert!(!md.contains("assets/page-0006-chart.png"));
    }

    #[test]
    fn inject_keeps_fig_when_rewriting_chart_to_table() {
        let dir = tempfile::tempdir().unwrap();
        let assets = dir.path().join("assets");
        std::fs::create_dir_all(&assets).unwrap();
        let png = {
            use image::{ImageBuffer, ImageFormat, Rgba};
            let img: ImageBuffer<Rgba<u8>, _> = ImageBuffer::from_pixel(1, 1, Rgba([0, 0, 0, 255]));
            let mut buf = std::io::Cursor::new(Vec::new());
            img.write_to(&mut buf, ImageFormat::Png).unwrap();
            buf.into_inner()
        };
        std::fs::write(assets.join("page-0007-fig-01.png"), &png).unwrap();
        std::fs::write(assets.join("page-0007-table-01.png"), &png).unwrap();
        std::fs::write(assets.join("page-0007-chart.png"), &png).unwrap();
        let md = "<!-- edgequake-page:7 -->\n![Figure 2](assets/page-0007-fig-01.png)\n\n## Figure 2\n\n![x](assets/page-0007-chart.png)\n\n## Table 2: Results\n";
        let out = inject_on_disk_region_assets(md, dir.path());
        assert!(
            out.contains("assets/page-0007-fig-01.png"),
            "fig must remain: {out}"
        );
        assert!(
            out.contains("assets/page-0007-table-01.png"),
            "table must appear: {out}"
        );
        assert!(
            !out.contains("assets/page-0007-chart.png"),
            "chart must go: {out}"
        );
    }

    #[test]
    fn inject_empty_markdown_is_noop() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(inject_on_disk_region_assets("", dir.path()), "");
        assert_eq!(inject_on_disk_region_assets("   ", dir.path()), "   ");
    }

    #[test]
    fn inject_strips_missing_chart_href() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("assets")).unwrap();
        let md = "<!-- edgequake-page:2 -->\n![Chart](assets/page-0002-chart.png)\n\nProse.\n";
        let out = inject_on_disk_region_assets(md, dir.path());
        assert!(!out.contains("assets/page-0002-chart.png"));
        assert!(out.contains("Prose."));
    }

    #[test]
    fn normalize_fills_missing_page_gaps() {
        let pages = vec![VisionPageSlice {
            page_num: 2,
            markdown: "only page 2".into(),
        }];
        let normalized = normalize_vision_pages(&pages, 3, "");
        assert_eq!(normalized.len(), 3);
        assert_eq!(normalized[0].page_num, 1);
        assert!(normalized[0].markdown.is_empty());
        assert_eq!(normalized[1].markdown, "only page 2");
        assert_eq!(normalized[2].page_num, 3);
    }

    #[test]
    fn normalize_selected_skips_out_of_group_pages() {
        let pages = vec![VisionPageSlice {
            page_num: 2,
            markdown: "print page".into(),
        }];
        let selected = vec![1usize, 2, 5];
        let normalized = normalize_selected_vision_pages(&pages, &selected, "");
        assert_eq!(normalized.len(), 3);
        assert_eq!(
            normalized.iter().map(|p| p.page_num).collect::<Vec<_>>(),
            vec![1, 2, 5]
        );
        assert!(normalized[0].markdown.is_empty());
        assert_eq!(normalized[1].markdown, "print page");
        assert!(!normalized.iter().any(|p| p.page_num == 3));
    }

    #[test]
    fn stitch_prefers_real_content_over_placeholder() {
        let print = "<!-- edgequake-page:1 -->\nReal print content\n\n<!-- edgequake-page:2 -->\nMore print\n";
        let manuscript = format!(
            "<!-- edgequake-page:1 -->\n{EMPTY_VISION_PAGE_PLACEHOLDER}\n\n<!-- edgequake-page:3 -->\nManuscript body\n"
        );
        let stitched = stitch_page_markdown_in_order(&[print.to_string(), manuscript]);
        assert!(stitched.contains("Real print content"));
        assert!(!stitched.contains(&format!(
            "<!-- edgequake-page:1 -->\n{EMPTY_VISION_PAGE_PLACEHOLDER}"
        )));
        assert!(stitched.contains("Manuscript body"));
        assert!(stitched.contains("More print"));
        let nums = page_numbers_from_markdown(&stitched);
        assert_eq!(nums, vec![1, 2, 3]);
    }

    #[test]
    fn mixed_13_12_selection_emits_each_page_once() {
        let print_pages: Vec<usize> = (1..=13).collect();
        let manuscript_pages: Vec<usize> = (14..=25).collect();
        let print_slices: Vec<VisionPageSlice> = print_pages
            .iter()
            .map(|&n| VisionPageSlice {
                page_num: n,
                markdown: format!("print-{n}"),
            })
            .collect();
        let ms_slices: Vec<VisionPageSlice> = manuscript_pages
            .iter()
            .map(|&n| VisionPageSlice {
                page_num: n,
                markdown: format!("ms-{n}"),
            })
            .collect();
        let print_md = assemble_vision_markdown(
            &normalize_selected_vision_pages(&print_slices, &print_pages, ""),
            false,
            None,
        );
        let ms_md = assemble_vision_markdown(
            &normalize_selected_vision_pages(&ms_slices, &manuscript_pages, ""),
            false,
            None,
        );
        // Simulate the old bug: full-document normalize would inject placeholders.
        let buggy_ms =
            assemble_vision_markdown(&normalize_vision_pages(&ms_slices, 25, ""), false, None);
        let buggy = stitch_page_markdown_in_order(&[print_md.clone(), buggy_ms]);
        // Without prefer_page_section, page 1 would be placeholder — with it, print wins.
        assert!(buggy.contains("print-1"));
        let good = stitch_page_markdown_in_order(&[print_md, ms_md]);
        let nums = page_numbers_from_markdown(&good);
        assert_eq!(nums.len(), 25);
        assert_eq!(nums, (1..=25).collect::<Vec<_>>());
        assert!(good.contains("print-13"));
        assert!(good.contains("ms-14"));
        assert!(good.contains("ms-25"));
    }
}
