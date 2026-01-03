#!/usr/bin/env python3
"""
PDF-to-Markdown Extraction Report Generator

Generates comprehensive HTML and JSON reports from evaluation data.
"""

import json
import sys
from datetime import datetime
from pathlib import Path
from typing import Dict, List


def generate_html_report(report_json: Dict, output_path: Path) -> None:
    """Generate HTML report from JSON evaluation data."""

    html = f"""<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>PDF-to-Markdown Evaluation Report</title>
    <style>
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        body {{
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
            line-height: 1.6;
            color: #333;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            min-height: 100vh;
            padding: 20px;
        }}
        .container {{
            max-width: 1200px;
            margin: 0 auto;
            background: white;
            border-radius: 12px;
            box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
            overflow: hidden;
        }}
        header {{
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
            padding: 40px 20px;
            text-align: center;
        }}
        header h1 {{
            font-size: 2.5em;
            margin-bottom: 10px;
        }}
        header p {{
            font-size: 1.1em;
            opacity: 0.9;
        }}
        .content {{ padding: 40px; }}
        .metrics-grid {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
            gap: 20px;
            margin: 30px 0;
        }}
        .metric-card {{
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
            padding: 20px;
            border-radius: 8px;
            text-align: center;
        }}
        .metric-value {{
            font-size: 2.5em;
            font-weight: bold;
            margin: 10px 0;
        }}
        .metric-label {{
            font-size: 0.9em;
            opacity: 0.9;
        }}
        .status-excellent {{ background: #4CAF50; }}
        .status-good {{ background: #2196F3; }}
        .status-acceptable {{ background: #FF9800; }}
        .status-poor {{ background: #f44336; }}
        table {{
            width: 100%;
            border-collapse: collapse;
            margin: 30px 0;
        }}
        th {{
            background: #f5f5f5;
            padding: 12px;
            text-align: left;
            font-weight: 600;
            border-bottom: 2px solid #ddd;
        }}
        td {{
            padding: 12px;
            border-bottom: 1px solid #ddd;
        }}
        tr:hover {{ background: #f9f9f9; }}
        .chart {{
            margin: 30px 0;
            padding: 20px;
            background: #f9f9f9;
            border-radius: 8px;
        }}
        .progress-bar {{
            background: #ddd;
            height: 20px;
            border-radius: 10px;
            overflow: hidden;
            margin: 5px 0;
        }}
        .progress-fill {{
            background: linear-gradient(90deg, #4CAF50, #45a049);
            height: 100%;
            transition: width 0.3s ease;
            display: flex;
            align-items: center;
            justify-content: flex-end;
            padding-right: 5px;
            color: white;
            font-size: 0.8em;
        }}
        .footer {{
            background: #f5f5f5;
            padding: 20px;
            text-align: center;
            color: #666;
            border-top: 1px solid #ddd;
        }}
        section {{
            margin: 40px 0;
        }}
        h2 {{
            color: #667eea;
            margin-bottom: 20px;
            padding-bottom: 10px;
            border-bottom: 2px solid #ddd;
        }}
        .status-badge {{
            display: inline-block;
            padding: 4px 12px;
            border-radius: 20px;
            font-size: 0.85em;
            font-weight: 600;
            color: white;
            margin: 2px;
        }}
    </style>
</head>
<body>
    <div class="container">
        <header>
            <h1>📊 PDF-to-Markdown Extraction Evaluation</h1>
            <p>Comprehensive Assessment Report</p>
            <p style="font-size: 0.9em; margin-top: 10px;">Generated: {report_json.get('timestamp', 'Unknown')}</p>
        </header>
        
        <div class="content">
            <!-- Overall Metrics -->
            <section>
                <h2>Overall Performance</h2>
                <div class="metrics-grid">
                    <div class="metric-card">
                        <div class="metric-label">Overall Score</div>
                        <div class="metric-value">{report_json['overall_score']:.1f}%</div>
                    </div>
                    <div class="metric-card">
                        <div class="metric-label">Total Documents</div>
                        <div class="metric-value">{report_json['total_documents']}</div>
                    </div>
                    <div class="metric-card">
                        <div class="metric-label">Categories</div>
                        <div class="metric-value">{len(report_json['categories'])}</div>
                    </div>
                </div>
            </section>
            
            <!-- Category Results -->
            <section>
                <h2>Category Results</h2>
                <table>
                    <thead>
                        <tr>
                            <th>Category</th>
                            <th>Average Score</th>
                            <th>Documents</th>
                            <th>Excellent</th>
                            <th>Good</th>
                            <th>Acceptable</th>
                            <th>Poor</th>
                        </tr>
                    </thead>
                    <tbody>
"""

    for cat in report_json["categories"]:
        score_color = (
            "status-excellent"
            if cat["average_score"] >= 95
            else (
                "status-good"
                if cat["average_score"] >= 90
                else (
                    "status-acceptable" if cat["average_score"] >= 85 else "status-poor"
                )
            )
        )

        html += f"""                        <tr>
                            <td><strong>{cat['name']}</strong></td>
                            <td><span class="status-badge {score_color}">{cat['average_score']:.1f}%</span></td>
                            <td>{cat['document_count']}</td>
                            <td>{cat['excellent_count']}</td>
                            <td>{cat['good_count']}</td>
                            <td>{cat['acceptable_count']}</td>
                            <td>{cat['poor_count']}</td>
                        </tr>
"""

    html += """                    </tbody>
                </table>
            </section>
            
            <!-- Document Details -->
            <section>
                <h2>Document Metrics</h2>
                <table>
                    <thead>
                        <tr>
                            <th>Document</th>
                            <th>Category</th>
                            <th>Text Preservation</th>
                            <th>Format Preservation</th>
                            <th>Structural Fidelity</th>
                            <th>Status</th>
                        </tr>
                    </thead>
                    <tbody>
"""

    for doc in sorted(
        report_json["documents"], key=lambda x: (x["category"], x["name"])
    )[
        :50
    ]:  # Show first 50
        status_class = f"status-badge status-{doc['status'].lower()}"

        html += f"""                        <tr>
                            <td>{doc['name']}</td>
                            <td>{doc['category']}</td>
                            <td><div class="progress-bar"><div class="progress-fill" style="width: {doc['text_preservation']:.0f}%">{doc['text_preservation']:.0f}%</div></div></td>
                            <td><div class="progress-bar"><div class="progress-fill" style="width: {doc['formatting_preservation']:.0f}%">{doc['formatting_preservation']:.0f}%</div></div></td>
                            <td><div class="progress-bar"><div class="progress-fill" style="width: {doc['structural_fidelity']:.0f}%">{doc['structural_fidelity']:.0f}%</div></div></td>
                            <td><span class="{status_class}">{doc['status'].upper()}</span></td>
                        </tr>
"""

    html += """                    </tbody>
                </table>
            </section>
            
            <!-- Summary -->
            <section>
                <h2>Summary</h2>
                <div class="chart">
                    <p><strong>{}</strong></p>
                </div>
            </section>
        </div>
        
        <div class="footer">
            <p>EdgeQuake PDF Extraction Evaluation | Test Protocol v1.0</p>
            <p style="margin-top: 10px; font-size: 0.9em;">This report evaluates PDF-to-Markdown conversion fidelity through comprehensive diff analysis.</p>
        </div>
    </div>
</body>
</html>
""".format(
        report_json["summary"]
    )

    with open(output_path, "w") as f:
        f.write(html)

    print(f"✓ HTML report saved: {output_path}")


def generate_text_report(report_json: Dict, output_path: Path) -> None:
    """Generate detailed text report."""

    lines = [
        "=" * 70,
        "PDF-to-Markdown Extraction Evaluation Report",
        "=" * 70,
        "",
        f"Generated: {report_json.get('timestamp', 'Unknown')}",
        "",
        f"Overall Score: {report_json['overall_score']:.1f}%",
        f"Total Documents: {report_json['total_documents']}",
        f"Categories: {len(report_json['categories'])}",
        "",
        "-" * 70,
        "Category Performance",
        "-" * 70,
        "",
    ]

    for cat in report_json["categories"]:
        lines.append(
            f"{cat['name']:30s} {cat['average_score']:6.1f}%  "
            f"({cat['document_count']} docs: "
            f"{cat['excellent_count']} excellent, "
            f"{cat['good_count']} good, "
            f"{cat['acceptable_count']} acceptable, "
            f"{cat['poor_count']} poor)"
        )

    lines.append("")
    lines.append(report_json["summary"])
    lines.append("")
    lines.append("=" * 70)

    with open(output_path, "w") as f:
        f.write("\n".join(lines))

    print(f"✓ Text report saved: {output_path}")


def main():
    """Main entry point."""
    if len(sys.argv) < 2:
        print("Usage: report.py <evaluation_report.json>")
        sys.exit(1)

    report_path = Path(sys.argv[1])

    if not report_path.exists():
        print(f"Error: Report file not found: {report_path}")
        sys.exit(1)

    with open(report_path) as f:
        report = json.load(f)

    # Generate reports
    output_dir = report_path.parent

    generate_html_report(report, output_dir / "evaluation_report.html")
    generate_text_report(report, output_dir / "evaluation_report.txt")

    print(f"\nReports generated in {output_dir}/")


if __name__ == "__main__":
    main()
