"""
Analyze Benchmark 1 — Cache Warmup Stability
=============================================
Reads one or more warmup .jsonl files and produces:
  - Rolling-average response time vs cumulative peptides
  - Stability point annotation (rolling CV < 5%)
  - Major page faults per batch
  - Saved to plots/warmup.png

Usage
-----
python3 analyze_warmup.py results/warmup_nommap.jsonl results/warmup_mmap.jsonl \\
    --output plots/warmup.png
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

import matplotlib.pyplot as plt
import matplotlib.ticker as mticker
import numpy as np
import pandas as pd


ROLLING_WINDOW = 20   # batches for rolling statistics
CV_THRESHOLD   = 0.05  # coefficient of variation threshold for "stable"

LINE_COLORS = ["#2196F3", "#E53935", "#43A047", "#FB8C00"]
FILL_COLORS = ["#90CAF9", "#EF9A9A", "#A5D6A7", "#FFCC80"]


def _apply_style() -> None:
    plt.rcParams.update({
        "figure.facecolor":  "#FFFFFF",
        "axes.facecolor":    "#FFFFFF",
        "axes.edgecolor":    "#CCCCCC",
        "axes.linewidth":    0.8,
        "axes.spines.top":   False,
        "axes.spines.right": False,
        "grid.color":        "#E0E0E0",
        "grid.linewidth":    0.6,
        "font.family":       "sans-serif",
        "font.size":         11,
        "axes.titlesize":    13,
        "axes.titleweight":  "bold",
        "axes.labelsize":    11,
        "xtick.labelsize":   10,
        "ytick.labelsize":   10,
        "legend.fontsize":   10,
        "legend.framealpha": 0.9,
        "legend.edgecolor":  "#CCCCCC",
    })


def load_jsonl(path: str) -> pd.DataFrame:
    records = []
    with open(path) as fh:
        for line in fh:
            line = line.strip()
            if line:
                records.append(json.loads(line))
    if not records:
        raise ValueError(f"No records found in {path}")
    return pd.json_normalize(records)


def find_stability_index(series: pd.Series, window: int, cv_thresh: float) -> int | None:
    """Return the first index where the rolling CV drops below cv_thresh."""
    rolling = series.rolling(window)
    cv      = rolling.std() / rolling.mean()
    stable  = cv[cv < cv_thresh]
    if stable.empty:
        return None
    return int(stable.index[0])


def _fmt_millions(v: float, _) -> str:
    if v >= 1_000_000:
        return f"{v/1_000_000:.1f}M"
    if v >= 1_000:
        return f"{v/1_000:.0f}k"
    return str(int(v))


def plot_warmup(dfs: list[tuple[str, pd.DataFrame]], out_path: Path) -> None:
    _apply_style()

    fig, (ax1, ax2) = plt.subplots(
        2, 1, figsize=(13, 9), sharex=True, facecolor="#FFFFFF",
    )
    fig.subplots_adjust(hspace=0.12)

    bar_width_default = None

    for idx, (label, df) in enumerate(dfs):
        color      = LINE_COLORS[idx % len(LINE_COLORS)]
        fill_color = FILL_COLORS[idx % len(FILL_COLORS)]

        x              = df["cumulative_peptides"]
        y              = df["wall_time_s"]
        rolling_mean   = y.rolling(ROLLING_WINDOW, min_periods=1).mean()
        rolling_std    = y.rolling(ROLLING_WINDOW, min_periods=1).std().fillna(0)

        ax1.plot(
            x, rolling_mean,
            color=color, linewidth=2, label=f"{label} (rolling avg)",
        )
        ax1.fill_between(
            x,
            rolling_mean - rolling_std,
            rolling_mean + rolling_std,
            color=fill_color, alpha=0.30,
        )

        stab_idx = find_stability_index(y, ROLLING_WINDOW, CV_THRESHOLD)
        if stab_idx is not None:
            stab_x = int(df["cumulative_peptides"].iloc[stab_idx])
            stab_y = rolling_mean.iloc[stab_idx]
            ax1.axvline(stab_x, linestyle=":", linewidth=1.5, color=color, alpha=0.7)
            ax1.annotate(
                f"stable\n{stab_x:,}",
                xy=(stab_x, stab_y),
                xytext=(12, 6),
                textcoords="offset points",
                fontsize=8,
                color=color,
                arrowprops=dict(arrowstyle="-", color=color, alpha=0.5),
            )

        if bar_width_default is None and len(x) > 1:
            bar_width_default = (x.iloc[1] - x.iloc[0]) * 0.7

        bar_width = bar_width_default if bar_width_default is not None else 1
        ax2.bar(
            x,
            df["page_faults_major"],
            width=bar_width,
            color=fill_color,
            edgecolor=color,
            linewidth=0.4,
            alpha=0.75,
            label=label,
        )

    for ax in (ax1, ax2):
        ax.set_facecolor("#FFFFFF")
        ax.grid(True, axis="y", linestyle="--")
        for spine in ("top", "right"):
            ax.spines[spine].set_visible(False)
        ax.xaxis.set_major_formatter(mticker.FuncFormatter(_fmt_millions))

    ax1.set_ylabel("Response time (s)", labelpad=8)
    ax1.set_title(f"Cache Warmup — Rolling-Average Response Time  (window = {ROLLING_WINDOW} batches)")
    ax1.legend(loc="upper right", ncol=1)
    ax1.yaxis.set_minor_locator(mticker.AutoMinorLocator())

    ax2.set_ylabel("Major page faults / batch", labelpad=8)
    ax2.set_xlabel("Cumulative peptides processed", labelpad=8)
    ax2.set_title("Major Page Faults per Batch")
    ax2.legend(loc="upper right")
    ax2.yaxis.set_major_formatter(mticker.FuncFormatter(lambda v, _: f"{v:,.0f}"))

    fig.tight_layout()
    out_path.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(out_path, dpi=150, bbox_inches="tight")
    print(f"[analyze_warmup] Plot saved to {out_path}")


def parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser(description="Analyze Benchmark 1: Cache Warmup Stability")
    p.add_argument("jsonl_files", nargs="+", help="One or more warmup .jsonl result files")
    p.add_argument("--output", default="plots/warmup.png")
    p.add_argument("--rolling-window", type=int, default=ROLLING_WINDOW)
    return p.parse_args()


def main() -> None:
    args = parse_args()
    dfs = []
    for path in args.jsonl_files:
        df = load_jsonl(path)
        label = Path(path).stem
        dfs.append((label, df))
        print(f"[analyze_warmup] Loaded {len(df)} records from {path}")

    plot_warmup(dfs, Path(args.output))


if __name__ == "__main__":
    main()
