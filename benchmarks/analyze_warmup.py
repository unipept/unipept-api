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
import numpy as np
import pandas as pd


ROLLING_WINDOW = 20   # batches for rolling statistics
CV_THRESHOLD = 0.05   # coefficient of variation threshold for "stable"


def load_jsonl(path: str) -> pd.DataFrame:
    records = []
    with open(path) as fh:
        for line in fh:
            line = line.strip()
            if line:
                records.append(json.loads(line))
    if not records:
        raise ValueError(f"No records found in {path}")
    df = pd.json_normalize(records)
    return df


def find_stability_index(series: pd.Series, window: int, cv_thresh: float) -> int | None:
    """Return the first index where the rolling CV drops below cv_thresh."""
    rolling = series.rolling(window)
    cv = rolling.std() / rolling.mean()
    stable = cv[cv < cv_thresh]
    if stable.empty:
        return None
    return int(stable.index[0])


def plot_warmup(dfs: list[tuple[str, pd.DataFrame]], out_path: Path) -> None:
    fig, (ax1, ax2) = plt.subplots(2, 1, figsize=(12, 8), sharex=True)

    for label, df in dfs:
        x = df["cumulative_peptides"]
        y = df["wall_time_s"]

        rolling_mean = y.rolling(ROLLING_WINDOW, min_periods=1).mean()
        ax1.plot(x, rolling_mean, label=f"{label} (rolling avg)", linewidth=1.5)

        stab_idx = find_stability_index(y, ROLLING_WINDOW, CV_THRESHOLD)
        if stab_idx is not None:
            ax1.axvline(
                x=int(df["cumulative_peptides"].iloc[stab_idx]),
                linestyle="--",
                alpha=0.6,
                label=f"{label} stable at {int(df['cumulative_peptides'].iloc[stab_idx]):,} pep",
            )

        ax2.bar(
            x,
            df["page_faults_major"],
            width=(x.iloc[1] - x.iloc[0]) * 0.8 if len(x) > 1 else 1,
            alpha=0.5,
            label=label,
        )

    ax1.set_ylabel("Response time (s)")
    ax1.set_title(f"Warmup: rolling-avg response time (window={ROLLING_WINDOW} batches)")
    ax1.legend(fontsize=8)
    ax1.grid(True, alpha=0.3)

    ax2.set_ylabel("Major page faults (per batch)")
    ax2.set_xlabel("Cumulative peptides processed")
    ax2.set_title("Major page faults per batch")
    ax2.legend(fontsize=8)
    ax2.grid(True, alpha=0.3)

    fig.tight_layout()
    out_path.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(out_path, dpi=150)
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
