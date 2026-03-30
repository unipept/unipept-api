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
import matplotlib.ticker as mticker
import numpy as np
import pandas as pd


PALETTE = {
    "mmap":    {"primary": "#2196F3", "secondary": "#90CAF9"},
    "no-mmap": {"primary": "#E53935", "secondary": "#EF9A9A"},
}


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
    _apply_style()

    df = df.copy()
    df["meta.ram_limit_gb"] = df["meta.ram_limit_gb"].fillna(0)

    valid = df[df["wall_time_s"].notna() & (df["http_status"] == 200)]

    groups = valid.groupby(["meta.mmap", "meta.ram_limit_gb"])
    stats = groups["wall_time_s"].agg(
        median="median",
        p95=lambda s: s.quantile(0.95),
    ).reset_index()
    fault_stats = (
        valid.groupby(["meta.mmap", "meta.ram_limit_gb"])["page_faults_major"]
        .mean()
        .reset_index()
        .rename(columns={"page_faults_major": "avg_majflt"})
    )

    all_limits = sorted(stats["meta.ram_limit_gb"].unique())
    x_labels   = [_ram_label(v) for v in all_limits]
    limit_to_x = {v: i for i, v in enumerate(all_limits)}

    fig, (ax1, ax2) = plt.subplots(2, 1, figsize=(12, 9), facecolor="#FFFFFF")
    fig.subplots_adjust(hspace=0.38)

    for mmap_val, key in [(True, "mmap"), (False, "no-mmap")]:
        colors = PALETTE[key]
        subset = stats[stats["meta.mmap"] == mmap_val].sort_values("meta.ram_limit_gb")
        x = [limit_to_x[v] for v in subset["meta.ram_limit_gb"]]

        ax1.plot(
            x, subset["median"],
            marker="o", markersize=6, linewidth=2,
            color=colors["primary"], label=f"{key} — median",
        )
        ax1.plot(
            x, subset["p95"],
            marker="s", markersize=6, linewidth=2, linestyle="--",
            color=colors["primary"], alpha=0.6, label=f"{key} — p95",
        )
        ax1.fill_between(
            x, subset["median"], subset["p95"],
            color=colors["secondary"], alpha=0.25,
        )

        fsub = fault_stats[fault_stats["meta.mmap"] == mmap_val].sort_values("meta.ram_limit_gb")
        fx   = [limit_to_x[v] for v in fsub["meta.ram_limit_gb"]]
        ax2.plot(
            fx, fsub["avg_majflt"],
            marker="^", markersize=7, linewidth=2,
            color=colors["primary"], label=key,
        )
        ax2.fill_between(
            fx, 0, fsub["avg_majflt"],
            color=colors["secondary"], alpha=0.20,
        )

    for ax in (ax1, ax2):
        ax.set_xticks(range(len(x_labels)))
        ax.set_xticklabels(x_labels, rotation=30, ha="right")
        ax.grid(True, axis="y", linestyle="--")
        ax.set_facecolor("#FFFFFF")
        for spine in ("top", "right"):
            ax.spines[spine].set_visible(False)

    ax1.set_ylabel("Response time (s)", labelpad=8)
    ax1.set_title("Response Time vs RAM Limit")
    ax1.legend(ncol=2, loc="upper right")
    ax1.yaxis.set_minor_locator(mticker.AutoMinorLocator())

    ax2.set_ylabel("Avg major page faults / batch", labelpad=8)
    ax2.set_xlabel("RAM limit", labelpad=8)
    ax2.set_title("Major Page Faults vs RAM Limit")
    ax2.legend(loc="upper right")
    ax2.yaxis.set_major_formatter(mticker.FuncFormatter(lambda v, _: f"{v:,.0f}"))

    fig.tight_layout()
    out_path.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(out_path, dpi=150, bbox_inches="tight")
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
