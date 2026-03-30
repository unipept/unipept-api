"""
Analyze Benchmark 3 — Complete vs Sparse Suffix Array
======================================================
Reads all four sa_*.jsonl result files and produces side-by-side boxplots of:
  - Response time distribution
  - Major page fault distribution
Across all (sa_type × mmap) combinations.  Saved to plots/sa_compare.png.

Usage
-----
python3 analyze_sa_compare.py \\
    results/sa_complete_mmap.jsonl \\
    results/sa_complete_nommap.jsonl \\
    results/sa_sparse_mmap.jsonl \\
    results/sa_sparse_nommap.jsonl \\
    --output plots/sa_compare.png
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

import matplotlib.pyplot as plt
import matplotlib.ticker as mticker
import numpy as np
import pandas as pd


PALETTE = ["#2196F3", "#90CAF9", "#E53935", "#EF9A9A"]
MEDIAN_COLOR = "#212121"


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


def combination_label(df: pd.DataFrame) -> str:
    sa   = df["meta.sa_type"].iloc[0]
    mmap = df["meta.mmap"].iloc[0]
    return f"{sa}\n{'mmap' if mmap else 'no-mmap'}"


def _style_boxplot(bp: dict, colors: list[str]) -> None:
    for patch, color in zip(bp["boxes"], colors):
        patch.set_facecolor(color)
        patch.set_alpha(0.80)
        patch.set_linewidth(0.8)
    for median in bp["medians"]:
        median.set_color(MEDIAN_COLOR)
        median.set_linewidth(2)
    for whisker in bp["whiskers"]:
        whisker.set_linewidth(0.8)
        whisker.set_color("#666666")
    for cap in bp["caps"]:
        cap.set_linewidth(0.8)
        cap.set_color("#666666")
    for flier in bp["fliers"]:
        flier.set_marker(".")
        flier.set_markersize(4)
        flier.set_alpha(0.4)
        flier.set_color("#888888")


def _annotate_medians(ax: plt.Axes, bp: dict, data: list[np.ndarray]) -> None:
    for i, (line, arr) in enumerate(zip(bp["medians"], data), start=1):
        med = np.median(arr)
        ax.text(
            i, med,
            f" {med:.2f}",
            va="center", ha="left",
            fontsize=8, color=MEDIAN_COLOR, fontweight="bold",
        )


def plot_sa_compare(datasets: list[tuple[str, pd.DataFrame]], out_path: Path) -> None:
    _apply_style()

    labels = [lbl for lbl, _ in datasets]

    wall_data = [
        df.loc[df["wall_time_s"].notna() & (df["http_status"] == 200), "wall_time_s"].values
        for _, df in datasets
    ]
    fault_data = [
        df.loc[df["wall_time_s"].notna(), "page_faults_major"].values
        for _, df in datasets
    ]

    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 7), facecolor="#FFFFFF")
    fig.subplots_adjust(wspace=0.32)

    bp1 = ax1.boxplot(
        wall_data, labels=labels, patch_artist=True,
        notch=False, widths=0.55,
    )
    _style_boxplot(bp1, PALETTE)
    _annotate_medians(ax1, bp1, wall_data)

    ax1.set_ylabel("Response time (s)", labelpad=8)
    ax1.set_title("Response Time Distribution\nby SA type × mmap")
    ax1.grid(True, axis="y", linestyle="--")
    ax1.set_facecolor("#FFFFFF")
    for spine in ("top", "right"):
        ax1.spines[spine].set_visible(False)

    bp2 = ax2.boxplot(
        fault_data, labels=labels, patch_artist=True,
        notch=False, widths=0.55,
    )
    _style_boxplot(bp2, PALETTE)

    ax2.set_ylabel("Major page faults / batch", labelpad=8)
    ax2.set_title("Major Page Fault Distribution\nby SA type × mmap")
    ax2.grid(True, axis="y", linestyle="--")
    ax2.set_facecolor("#FFFFFF")
    ax2.yaxis.set_major_formatter(mticker.FuncFormatter(lambda v, _: f"{v:,.0f}"))
    for spine in ("top", "right"):
        ax2.spines[spine].set_visible(False)

    # Legend patches
    from matplotlib.patches import Patch
    handles = [Patch(facecolor=c, alpha=0.8, label=lbl.replace("\n", " / "))
               for c, lbl in zip(PALETTE, labels)]
    fig.legend(
        handles=handles,
        loc="lower center",
        ncol=len(handles),
        bbox_to_anchor=(0.5, -0.04),
        framealpha=0.9,
        edgecolor="#CCCCCC",
    )

    # Print summary statistics
    print("\n[analyze_sa_compare] Summary statistics:")
    print(f"{'Combination':<25} {'median_time_s':>14} {'p95_time_s':>12} {'avg_majflt':>12}")
    print("-" * 65)
    for (lbl, df) in datasets:
        valid  = df[df["wall_time_s"].notna() & (df["http_status"] == 200)]
        med    = valid["wall_time_s"].median()
        p95    = valid["wall_time_s"].quantile(0.95)
        avg_mf = valid["page_faults_major"].mean()
        print(f"{lbl.replace(chr(10), '/'):<25} {med:>14.3f} {p95:>12.3f} {avg_mf:>12.1f}")

    fig.tight_layout(rect=[0, 0.05, 1, 1])
    out_path.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(out_path, dpi=150, bbox_inches="tight")
    print(f"\n[analyze_sa_compare] Plot saved to {out_path}")


def parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser(description="Analyze Benchmark 3: SA type comparison")
    p.add_argument("jsonl_files", nargs="+")
    p.add_argument("--output", default="plots/sa_compare.png")
    return p.parse_args()


def main() -> None:
    args = parse_args()
    datasets = []
    for path in args.jsonl_files:
        df    = load_jsonl(path)
        label = combination_label(df)
        datasets.append((label, df))
        print(f"[analyze_sa_compare] Loaded {len(df)} records from {path}  ({label.replace(chr(10), '/')})")

    plot_sa_compare(datasets, Path(args.output))


if __name__ == "__main__":
    main()
