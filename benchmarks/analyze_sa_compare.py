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
import numpy as np
import pandas as pd


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
    sa = df["meta.sa_type"].iloc[0]
    mmap = df["meta.mmap"].iloc[0]
    return f"{sa}\n{'mmap' if mmap else 'no-mmap'}"


def plot_sa_compare(datasets: list[tuple[str, pd.DataFrame]], out_path: Path) -> None:
    labels = [lbl for lbl, _ in datasets]

    wall_data = [
        df.loc[df["wall_time_s"].notna() & (df["http_status"] == 200), "wall_time_s"].values
        for _, df in datasets
    ]
    fault_data = [
        df.loc[df["wall_time_s"].notna(), "page_faults_major"].values
        for _, df in datasets
    ]

    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 6))

    bp1 = ax1.boxplot(wall_data, labels=labels, patch_artist=True, notch=False)
    ax1.set_ylabel("Response time (s)")
    ax1.set_title("Response time distribution\nper SA type × mmap combination")
    ax1.grid(True, axis="y", alpha=0.3)

    colors = ["#4878d0", "#ee854a", "#6acc65", "#d65f5f"]
    for patch, color in zip(bp1["boxes"], colors):
        patch.set_facecolor(color)
        patch.set_alpha(0.7)

    bp2 = ax2.boxplot(fault_data, labels=labels, patch_artist=True, notch=False)
    ax2.set_ylabel("Major page faults (per batch)")
    ax2.set_title("Major page fault distribution\nper SA type × mmap combination")
    ax2.grid(True, axis="y", alpha=0.3)

    for patch, color in zip(bp2["boxes"], colors):
        patch.set_facecolor(color)
        patch.set_alpha(0.7)

    # Print summary statistics
    print("\n[analyze_sa_compare] Summary statistics:")
    print(f"{'Combination':<25} {'median_time_s':>14} {'p95_time_s':>12} {'avg_majflt':>12}")
    print("-" * 65)
    for (lbl, df) in datasets:
        valid = df[df["wall_time_s"].notna() & (df["http_status"] == 200)]
        med = valid["wall_time_s"].median()
        p95 = valid["wall_time_s"].quantile(0.95)
        avg_mf = valid["page_faults_major"].mean()
        print(f"{lbl.replace(chr(10), '/'):<25} {med:>14.3f} {p95:>12.3f} {avg_mf:>12.1f}")

    fig.tight_layout()
    out_path.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(out_path, dpi=150)
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
        df = load_jsonl(path)
        label = combination_label(df)
        datasets.append((label, df))
        print(f"[analyze_sa_compare] Loaded {len(df)} records from {path}  ({label.replace(chr(10), '/')})")

    plot_sa_compare(datasets, Path(args.output))


if __name__ == "__main__":
    main()
