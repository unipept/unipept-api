"""
Analyze Benchmark 2 — RAM Limit vs Response Time
=================================================
Reads all ramlimit_*.jsonl files from a directory and produces:
  - Median and p95 response time vs RAM limit (mmap=true/false as separate lines)
  - Average major page faults vs RAM limit
  - Saved to plots/ramlimit.png

Usage
-----
python3 analyze_ramlimit.py --input-dir results/ --output plots/ramlimit.png
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

import matplotlib.pyplot as plt
import numpy as np
import pandas as pd


def load_jsonl(path: Path) -> pd.DataFrame:
    records = []
    with open(path) as fh:
        for line in fh:
            line = line.strip()
            if line:
                records.append(json.loads(line))
    if not records:
        return pd.DataFrame()
    return pd.json_normalize(records)


def load_all(input_dir: Path) -> pd.DataFrame:
    frames = []
    for p in sorted(input_dir.glob("ramlimit_*.jsonl")):
        df = load_jsonl(p)
        if not df.empty:
            frames.append(df)
            print(f"[analyze_ramlimit] Loaded {len(df)} records from {p.name}")
    if not frames:
        raise FileNotFoundError(f"No ramlimit_*.jsonl files found in {input_dir}")
    return pd.concat(frames, ignore_index=True)


def _ram_label(gb: float | None) -> str:
    if gb is None or (isinstance(gb, float) and np.isnan(gb)):
        return "unlimited"
    return f"{int(gb)} GB"


def plot_ramlimit(df: pd.DataFrame, out_path: Path) -> None:
    # Replace NaN ram_limit_gb with a sentinel for grouping
    df = df.copy()
    df["meta.ram_limit_gb"] = df["meta.ram_limit_gb"].fillna(0)

    # Drop failed requests for latency stats
    valid = df[df["wall_time_s"].notna() & (df["http_status"] == 200)]

    groups = valid.groupby(["meta.mmap", "meta.ram_limit_gb"])
    stats = groups["wall_time_s"].agg(
        median="median",
        p95=lambda s: s.quantile(0.95),
    ).reset_index()
    fault_stats = valid.groupby(["meta.mmap", "meta.ram_limit_gb"])["page_faults_major"].mean().reset_index()
    fault_stats.rename(columns={"page_faults_major": "avg_majflt"}, inplace=True)

    fig, (ax1, ax2) = plt.subplots(2, 1, figsize=(12, 8))

    for mmap_val in [True, False]:
        subset = stats[stats["meta.mmap"] == mmap_val].sort_values("meta.ram_limit_gb")
        x_labels = [_ram_label(v) for v in subset["meta.ram_limit_gb"]]
        x = range(len(x_labels))
        label_prefix = "mmap" if mmap_val else "no-mmap"

        ax1.plot(x, subset["median"], marker="o", label=f"{label_prefix} median")
        ax1.plot(x, subset["p95"], marker="s", linestyle="--", label=f"{label_prefix} p95")

        fsub = fault_stats[fault_stats["meta.mmap"] == mmap_val].sort_values("meta.ram_limit_gb")
        ax2.plot(range(len(fsub)), fsub["avg_majflt"], marker="^", label=label_prefix)

    ax1.set_xticks(range(len(x_labels)))
    ax1.set_xticklabels(x_labels, rotation=30, ha="right")
    ax1.set_ylabel("Response time (s)")
    ax1.set_title("Response time vs RAM limit")
    ax1.legend()
    ax1.grid(True, alpha=0.3)

    ax2.set_ylabel("Avg major page faults per batch")
    ax2.set_xlabel("RAM limit")
    ax2.set_title("Major page faults vs RAM limit")
    ax2.legend()
    ax2.grid(True, alpha=0.3)

    fig.tight_layout()
    out_path.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(out_path, dpi=150)
    print(f"[analyze_ramlimit] Plot saved to {out_path}")


def parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser(description="Analyze Benchmark 2: RAM Limit vs Response Time")
    p.add_argument("--input-dir", default="results/", help="Directory containing ramlimit_*.jsonl files")
    p.add_argument("--output", default="plots/ramlimit.png")
    return p.parse_args()


def main() -> None:
    args = parse_args()
    df = load_all(Path(args.input_dir))
    print(f"[analyze_ramlimit] Total records: {len(df)}")
    plot_ramlimit(df, Path(args.output))


if __name__ == "__main__":
    main()
