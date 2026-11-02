# SPEC-048 Screenshot Analysis

Generated: 2026-11-02T05:12:19.574Z

## S01-idle

- Header shows no Working/Busy pill
- No ingestion banner
- Completed row only — AC idle invariant
- File: `S01-idle.png`

## S02-working-parity

- ActiveRunsPanel owns working narrative (banner demoted)
- Headline is stage-specific (Extracting Entities)
- Phase strip extract=active; wire extracting=active
- No row stage under ActiveRuns (LAW-IS3)
- Working pill visible; completed row muted
- Dropzone quiet while Working (SPEC-155)
- File: `S02-working-parity.png`

## S03-server-stepper

- ActiveRunsPanel visible
- Full UnifiedStage timeline: prior done, extracting active, later pending
- Step detail shows 42/351 chunks
- Overall collapsed while stage meter owns N/M (LAW-IS2)
- Client 4-step legend not required (DEF-10 morph)
- File: `S03-server-stepper.png`

## S03b-converting-vision-figures

- Headline shows Converting PDF · 5/17 during Vision LLM figure analyze
- Step detail N/M for converting stage
- Converting step active in server stepper
- File: `S03b-converting-vision-figures.png`

## S04-queued

- Pill shows Queued (not Working/Busy) — AC-01 queued-only
- Banner in queued mode
- Stepper shows Queued admission chip (not fake Uploading active)
- File: `S04-queued.png`

## S05-stuck

- Stuck / needs attention banner when pending without workers
- ActiveRunsPanel keeps stuck per-doc cards (SPEC-051 zone)
- Reprocess CTA may be present
- File: `S05-stuck.png`

## S05b-fresh-upload-queued

- Fresh upload shows amber Queued — never red Needs attention
- Feedback zone narrates queue; toolbar banner demoted
- Chanel_Loop-style pending without tasks yet is normal queue
- File: `S05b-fresh-upload-queued.png`

## S06-pipeline-dialog

- Pipeline status dialog opened from Working pill
- Dialog progress matches banner (12% Extracting Entities)
- No backend-unavailable toast ( /live mocked )
- File: `S06-pipeline-dialog.png`

## S07-embedding-detail

- Embedding active with 80/200 detail
- Prior stages done including extracting/gleaning/merging
- File: `S07-embedding-detail.png`

## S08-merge-mode

- mode=merge: early stages skipped
- Merging active with entity counts
- AC-07 mode badge visible
- File: `S08-merge-mode.png`

## S09-failed-extract

- Failed mid-extract: ActiveRunsPanel cleared
- Failure visible on document row for retry
- File: `S09-failed-extract.png`

## S10-markdown-skip-convert

- Non-PDF: converting omitted from timeline
- Chunking active with 3/10 detail
- File: `S10-markdown-skip-convert.png`
