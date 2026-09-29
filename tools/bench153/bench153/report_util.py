"""Tiny HTML helpers shared by report sections."""

from __future__ import annotations

import html


def esc(x) -> str:
    return html.escape("" if x is None else str(x))


def ms_s(ms) -> str:
    if ms is None:
        return "—"
    v = float(ms)
    return f"{v / 1000:.1f} s" if v >= 1000 else f"{v:.0f} ms"


def pct(x) -> str:
    return f"{100 * (x or 0):.0f}%"


def fig(charts: dict, key: str, caption: str) -> str:
    c = charts.get(key)
    if not c:
        return ""
    return (f'<figure><img src="{c["data_uri"]}" alt="{esc(key)}"/>'
            f"<figcaption>{caption}</figcaption></figure>")


def rows(items: list[tuple[str, str]]) -> str:
    return "".join(f"<tr><td class='k'>{esc(k)}</td><td>{v}</td></tr>" for k, v in items)
