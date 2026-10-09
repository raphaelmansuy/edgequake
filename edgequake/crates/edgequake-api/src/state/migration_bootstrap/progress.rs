//! Operator-visible migrate progress (SPEC-150).
//!
//! Progress must be readable without `RUST_LOG`. Serving never prints here —
//! callers gate on `migrate_cli_mode()`.

use std::time::Duration;

/// Approximate product release that first shipped a given schema max.
/// Derived from `scripts/spec150/epochs.toml` (schema-distinct epochs).
pub fn release_for_schema(max_version: i64) -> &'static str {
    match max_version {
        0 => "empty",
        1..=24 => "v0.2.0–v0.4.1",
        25 => "v0.5.1",
        26 => "v0.5.5–v0.6.0",
        27..=29 => "v0.7.0",
        30 => "v0.8.0–v0.9.4",
        31 => "v0.9.5–v0.9.6",
        32 => "v0.9.7–v0.9.19",
        33..=34 => "v0.10.0–v0.10.5",
        35 => "v0.10.6–v0.11.2",
        36 => "v0.11.3–v0.12.5",
        37 => "v0.12.6",
        38 => "v0.12.7–v0.12.11",
        39..=77 => "v0.13.0–v0.13.1",
        78 => "v0.13.2",
        79 => "v0.13.3",
        80..=81 => "v0.14.0–v0.15.1",
        82..=83 => "v0.16.0",
        84..=86 => "v0.17.0–v0.18.0",
        87..=89 => "v0.19.0",
        90..=94 => "v0.20.0",
        95 => "v0.20.1–v0.20.2",
        96..=97 => "v0.21.0",
        98 => "v0.21.1–v0.21.3",
        99..=105 => "v0.22.0",
        106..=141 => "v0.23.0",
        142 => "v0.24.0–v0.24.1",
        143..=144 => "v0.24.2–v0.24.3",
        145..=147 => "v0.24.4",
        148 => "v0.25.0",
        149 => "v0.26.0–v0.26.10",
        150..=159 => "v0.27.0",
        160 => "v0.28.0–v0.28.2",
        161 => "v0.28.3",
        162 => "v0.28.4–v0.28.5",
        163 => "v0.29.0",
        164..=165 => "v0.30.0",
        166 => "v0.31.0",
        167..=168 => "v0.32.0–v0.32.2",
        169 => "v0.33.0",
        _ => "newer-than-matrix",
    }
}

/// Empty-DB duration class hypothesis (SPEC-150 §07).
pub fn duration_class(pending_count: usize, from_schema: i64) -> &'static str {
    if pending_count == 0 {
        return "none";
    }
    if from_schema >= 149 && pending_count <= 20 {
        return "S (<30s empty DB)";
    }
    if from_schema >= 105 {
        return "M (30s–3min empty DB; longer with data)";
    }
    if from_schema >= 38 {
        return "L (3–15min empty DB; XL with large graphs)";
    }
    "XL (depends on rowcount; take a backup first)"
}

/// True when this version is known to run heavy DDL (SHARE / AGE rewrite).
pub fn is_heavy_step(version: i64) -> bool {
    if matches!(
        version,
        70 | 71 | 74 | 128 | 129 | 130 | 132 | 143 | 144 | 156 | 158
    ) {
        return true;
    }
    edgequake_migrate_manifest::load()
        .migration
        .iter()
        .find(|e| e.version == version)
        .is_some_and(|e| e.lock_class == "ddl_share")
}

/// Format a single progress line (golden-tested).
pub fn format_step_start(idx: usize, total: usize, version: i64, description: &str) -> String {
    let desc = if description.is_empty() {
        "(no description)"
    } else {
        description
    };
    let heavy = if is_heavy_step(version) {
        "  [heavy DDL — may take minutes on large graphs]"
    } else {
        ""
    };
    format!("[{idx:>3}/{total:>3}] {version} {desc} … applying{heavy}")
}

/// Format a completion line (golden-tested).
pub fn format_step_done(
    idx: usize,
    total: usize,
    version: i64,
    description: &str,
    elapsed: Duration,
) -> String {
    let desc = if description.is_empty() {
        "(no description)"
    } else {
        description
    };
    format!(
        "[{idx:>3}/{total:>3}] {version} {desc} … applied in {:.1}s",
        elapsed.as_secs_f64()
    )
}

/// Format the upgrade-path banner printed before apply.
pub fn format_upgrade_path(
    db_schema: Option<i64>,
    binary_schema: i64,
    binary_version: &str,
    pending_count: usize,
    irreversible_pending: &[i64],
) -> String {
    let db_v = db_schema.unwrap_or(0);
    let db_rel = release_for_schema(db_v);
    let bin_rel = release_for_schema(binary_schema);
    let class = duration_class(pending_count, db_v);
    let drops = if irreversible_pending.is_empty() {
        "none".to_string()
    } else {
        irreversible_pending
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    };
    format!(
        "UPGRADE PATH\n\
         \x20 database schema : {db_v} (≈ {db_rel})\n\
         \x20 binary schema   : {binary_schema} (v{binary_version} ≈ {bin_rel})\n\
         \x20 pending steps   : {pending_count}\n\
         \x20 irreversible    : {drops}\n\
         \x20 duration class  : {class}"
    )
}

/// Print upgrade path to stdout (CLI only).
pub fn print_upgrade_path(
    db_schema: Option<i64>,
    binary_schema: i64,
    binary_version: &str,
    pending_count: usize,
    irreversible_pending: &[i64],
) {
    println!();
    println!(
        "{}",
        format_upgrade_path(
            db_schema,
            binary_schema,
            binary_version,
            pending_count,
            irreversible_pending,
        )
    );
    println!();
}

/// Print step start to stdout (CLI only).
pub fn print_step_start(idx: usize, total: usize, version: i64, description: &str) {
    println!("{}", format_step_start(idx, total, version, description));
    let _ = std::io::Write::flush(&mut std::io::stdout());
}

/// Print step done to stdout (CLI only).
pub fn print_step_done(
    idx: usize,
    total: usize,
    version: i64,
    description: &str,
    elapsed: Duration,
) {
    println!(
        "{}",
        format_step_done(idx, total, version, description, elapsed)
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn progress_line_golden() {
        let s = format_step_start(12, 47, 148, "document_pages_layout");
        assert_eq!(s, "[ 12/ 47] 148 document_pages_layout … applying");
        let d = format_step_done(
            12,
            47,
            148,
            "document_pages_layout",
            Duration::from_millis(1200),
        );
        assert_eq!(d, "[ 12/ 47] 148 document_pages_layout … applied in 1.2s");
    }

    #[test]
    fn heavy_step_annotated() {
        let s = format_step_start(1, 1, 156, "graph_lineage_source_ids_backfill");
        assert!(s.contains("heavy DDL"), "{s}");
    }

    #[test]
    fn upgrade_path_mentions_versions() {
        let s = format_upgrade_path(Some(149), 168, "0.32.2", 19, &[125]);
        assert!(s.contains("database schema : 149"), "{s}");
        assert!(s.contains("binary schema   : 168"), "{s}");
        assert!(s.contains("irreversible    : 125"), "{s}");
        assert!(s.contains("v0.26.0–v0.26.10"), "{s}");
        assert!(s.contains("v0.32.0–v0.32.2"), "{s}");
    }

    #[test]
    fn release_map_covers_head_train() {
        assert_eq!(release_for_schema(168), "v0.32.0–v0.32.2");
        assert_eq!(release_for_schema(159), "v0.27.0");
        assert_eq!(release_for_schema(0), "empty");
    }
}
