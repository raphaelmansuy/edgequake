"""Shared matplotlib style + saving (vector SVG for the PDF, 300-DPI PNG on disk)."""

from __future__ import annotations

import base64
from pathlib import Path

PNG_DPI = 300

TEAL, TEAL_DK, TEAL_LT = "#0f766e", "#134e4a", "#ccfbf1"
AMBER, RED, SKY, SKY_DK = "#b45309", "#dc2626", "#0ea5e9", "#0369a1"
NAVY, SLATE, MIST, LINE = "#0f172a", "#475569", "#f8fafc", "#e2e8f0"
GREEN, PURPLE = "#16a34a", "#7c3aed"


def setup():
    import matplotlib

    matplotlib.use("Agg")
    import matplotlib.pyplot as plt

    plt.rcParams.update({
        "font.family": "DejaVu Sans", "font.size": 10,
        "axes.facecolor": MIST, "figure.facecolor": "white", "axes.edgecolor": "#cbd5e1",
        "axes.labelcolor": NAVY, "axes.titleweight": "bold", "axes.titlesize": 12,
        "axes.spines.top": False, "axes.spines.right": False,
        "xtick.color": SLATE, "ytick.color": SLATE, "text.color": NAVY,
        "grid.color": LINE, "grid.linestyle": "-", "grid.linewidth": 0.9,
        "legend.frameon": False, "svg.fonttype": "path", "svg.hashsalt": "bench153",
        "savefig.facecolor": "white", "figure.dpi": 100,
    })
    return plt


def save(fig, plt, charts_dir: Path, name: str) -> dict[str, str]:
    """Write <name>.png (300 DPI) and <name>.svg; return paths + an SVG data-URI for HTML."""
    charts_dir.mkdir(parents=True, exist_ok=True)
    png, svg = charts_dir / f"{name}.png", charts_dir / f"{name}.svg"
    fig.savefig(png, dpi=PNG_DPI, bbox_inches="tight", pad_inches=0.12)
    fig.savefig(svg, format="svg", bbox_inches="tight", pad_inches=0.12)
    plt.close(fig)
    b64 = base64.b64encode(svg.read_bytes()).decode("ascii")
    return {"png": f"charts/{png.name}", "svg": f"charts/{svg.name}",
            "data_uri": f"data:image/svg+xml;base64,{b64}"}


def secs(ms) -> float:
    return (ms or 0) / 1000.0
