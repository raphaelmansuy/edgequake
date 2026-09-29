"""Print stylesheet for the highload report (plain string; no f-string brace escaping)."""

CSS = """
@page { size: A4; margin: 14mm 13mm 16mm 13mm;
  @bottom-left { content: "EdgeQuake · SPEC-153 · Higher user workload"; font-size: 7.5pt; color: #64748b; }
  @bottom-right { content: "page " counter(page) " / " counter(pages); font-size: 7.5pt; color: #64748b; }
}
* { box-sizing: border-box; }
body { font-family: Helvetica, Arial, sans-serif; color: #0f172a; font-size: 10pt; line-height: 1.5; margin: 0; }
h1,h2,h3 { font-family: Georgia, "Times New Roman", serif; margin: 0 0 .4em; line-height: 1.2; }
h1 { font-size: 23pt; color: #f8fafc; }
h2 { font-size: 14.5pt; margin-top: 1.3em; padding-bottom: .25em; border-bottom: 2px solid #0f766e; break-after: avoid; }
h3 { font-size: 11pt; color: #0f766e; margin-top: 1em; break-after: avoid; }
p { margin: 0 0 .65em; }
.hero { background: linear-gradient(135deg,#0f172a 0%,#134e4a 55%,#0f766e 100%); color:#f8fafc;
  padding: 20px 22px; border-radius: 8px; margin-bottom: 12px; }
.hero p { color:#cbd5e1; font-size: 10.5pt; }
.eyebrow { text-transform: uppercase; letter-spacing:.13em; font-size: 8pt; color:#99f6e4; font-weight:700; margin-bottom:8px; }
.meta { display:grid; grid-template-columns: 1fr 1fr 1fr; gap: 7px 14px; margin-top: 12px; font-size: 8.6pt; }
.meta div { border-top: 1px solid rgba(255,255,255,.2); padding-top: 5px; }
.meta strong { display:block; color:#99f6e4; font-size: 7pt; text-transform:uppercase; letter-spacing:.07em; }
.kpis { display:grid; grid-template-columns: repeat(4,1fr); gap: 8px; margin: 10px 0 12px; }
.kpi { background:#f8fafc; border:1px solid #e2e8f0; border-top: 4px solid #0f766e; border-radius: 6px; padding: 9px 10px; }
.kpi.amber { border-top-color:#b45309; } .kpi.sky { border-top-color:#0284c7; } .kpi.purple { border-top-color:#7c3aed; }
.kpi .label { font-size: 7pt; text-transform: uppercase; letter-spacing:.07em; color:#475569; font-weight:700; }
.kpi .value { font-family: Georgia, serif; font-size: 20pt; line-height: 1.1; margin: 3px 0; }
.kpi .hint { font-size: 8pt; color:#64748b; line-height: 1.35; }
.callout { background:#f0fdfa; border-left: 4px solid #0f766e; padding: 10px 13px; margin: 10px 0; border-radius: 0 6px 6px 0; break-inside: avoid; }
.callout.warn { background:#fff7ed; border-left-color:#b45309; }
.callout.info { background:#eff6ff; border-left-color:#0284c7; }
.callout ul { margin: .3em 0 .2em 1.1em; padding: 0; } .callout li { margin-bottom: .3em; }
figure { margin: 8px 0 12px; break-inside: avoid; }
figure img { width: 100%; height: auto; display: block; }
figcaption { font-size: 8.4pt; color:#475569; margin-top: 3px; line-height: 1.4; }
figcaption b { color:#0f172a; }
table { width:100%; border-collapse: collapse; margin: 6px 0 12px; font-size: 8.8pt; break-inside: avoid; }
th, td { border-bottom: 1px solid #e2e8f0; padding: 5px 7px; text-align:left; vertical-align: top; }
th { background:#f1f5f9; font-size: 7.6pt; text-transform: uppercase; letter-spacing:.04em; color:#475569; }
td.num, th.num { text-align:right; font-variant-numeric: tabular-nums; }
td.k { width: 30%; color:#475569; font-weight: 600; }
tr.ok td:last-child { color:#166534; font-weight:700; } tr.bad td:last-child { color:#b45309; font-weight:700; }
table.glance td:last-child { font-weight: 400; color:#0f172a; } table.glance tr.bad td:last-child { color:#b45309; }
table.glance td:first-child { width: 8%; } table.glance td:nth-child(3) { width: 34%; }
tr.bad { background:#fffbeb; }
.mono { font-family: ui-monospace, Menlo, monospace; font-size: 8pt; }
.muted { color:#64748b; font-size: 8.6pt; }
.steps { counter-reset: s; list-style: none; margin: .4em 0 .8em; padding: 0; }
.steps li { counter-increment: s; position: relative; padding: 6px 8px 6px 38px; margin-bottom: 5px;
  background:#f8fafc; border:1px solid #e2e8f0; border-radius: 6px; break-inside: avoid; }
.steps li::before { content: counter(s); position:absolute; left: 8px; top: 6px; width: 22px; height: 22px;
  border-radius: 50%; background:#0f766e; color:#fff; font-weight:700; text-align:center; line-height: 22px; font-size: 9pt; }
.two { display:grid; grid-template-columns: 1fr 1fr; gap: 12px; }
.gloss dt { font-weight: 700; color:#0f766e; margin-top: .5em; } .gloss dd { margin: 0 0 .2em; color:#334155; }
.pb { break-before: page; }
"""
