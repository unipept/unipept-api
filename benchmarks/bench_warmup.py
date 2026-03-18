"""
Benchmark 1 — Cache Warmup Stability
=====================================
Sends repeated batches of peptides to the API and records per-batch metrics
until the response time stabilises.  The API must already be running when
this script starts; the page cache should have been dropped beforehand.

Usage
-----
python3 bench_warmup.py \
    --peptide-file /data/peptides_1M.txt \
    --api-pid $API_PID \
    --batch-size 100 \
    --num-batches 500 \
    --mmap false \
    --sa-type complete \
    --output results/warmup_nommap.jsonl

See BENCHMARK.md for full setup instructions.
"""

from __future__ import annotations

import argparse
import itertools
import sys
from pathlib import Path

from bench_common import (
    build_record,
    load_peptides,
    make_run_id,
    post_pept2lca,
    read_proc_stat_faults,
    wait_for_api,
    write_record,
)


def parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser(description="Benchmark 1: Cache Warmup Stability")
    p.add_argument("--peptide-file", required=True, help="Plain-text peptide list (one per line)")
    p.add_argument("--api-pid", required=True, type=int, help="PID of the running API process")
    p.add_argument("--api-url", default="http://localhost:8080", help="Base URL of the API")
    p.add_argument("--batch-size", type=int, default=100)
    p.add_argument("--num-batches", type=int, default=500)
    p.add_argument("--equate-il", type=lambda x: x.lower() != "false", default=True)
    p.add_argument("--mmap", type=lambda x: x.lower() != "false", default=False,
                   help="Whether the API was started with --mmap true (metadata only)")
    p.add_argument("--sa-type", default="complete", choices=["complete", "sparse"],
                   help="Suffix-array type used by the running API (metadata only)")
    p.add_argument("--cgroup-path", default=None,
                   help="Path to cgroup dir (e.g. /sys/fs/cgroup/unipept_bench)")
    p.add_argument("--output", required=True, help="Output .jsonl file path")
    return p.parse_args()


def main() -> None:
    args = parse_args()
    out_path = Path(args.output)
    run_id = make_run_id()

    print(f"[warmup] run_id={run_id}  pid={args.api_pid}  batches={args.num_batches}  "
          f"batch_size={args.batch_size}  mmap={args.mmap}")

    print(f"[warmup] Waiting for API at {args.api_url} ...")
    wait_for_api(args.api_url)
    print("[warmup] API is ready.")

    all_peptides = load_peptides(args.peptide_file)
    print(f"[warmup] Loaded {len(all_peptides)} peptides from {args.peptide_file}")

    # Cycle over peptides so we can run as many batches as requested regardless
    # of how many unique peptides are available.
    peptide_cycle = itertools.cycle(all_peptides)

    cumulative = 0
    prev_minflt, prev_majflt = read_proc_stat_faults(args.api_pid)

    for batch_idx in range(args.num_batches):
        batch = [next(peptide_cycle) for _ in range(args.batch_size)]

        try:
            wall_time_s, status, n_results = post_pept2lca(
                args.api_url, batch, equate_il=args.equate_il
            )
        except Exception as exc:
            print(f"[warmup] batch {batch_idx}: HTTP error — {exc}", file=sys.stderr)
            wall_time_s, status, n_results = float("nan"), 0, 0

        cumulative += len(batch)

        minflt, majflt = read_proc_stat_faults(args.api_pid)
        record = build_record(
            benchmark="warmup",
            mmap=args.mmap,
            sa_type=args.sa_type,
            batch_size=args.batch_size,
            api_url=args.api_url,
            equate_il=args.equate_il,
            run_id=run_id,
            ram_limit_gb=None,
            batch_index=batch_idx,
            cumulative_peptides=cumulative,
            peptides_in_batch=len(batch),
            wall_time_s=wall_time_s,
            http_status=status,
            results_returned=n_results,
            api_pid=args.api_pid,
            prev_minflt=prev_minflt,
            prev_majflt=prev_majflt,
            cgroup_path=args.cgroup_path,
        )
        write_record(record, out_path)
        prev_minflt, prev_majflt = minflt, majflt

        if (batch_idx + 1) % 50 == 0:
            print(f"[warmup] {batch_idx + 1}/{args.num_batches} batches done  "
                  f"cumulative={cumulative}  last_wall={wall_time_s:.3f}s  "
                  f"majflt_delta={record['page_faults_major']}")

    print(f"[warmup] Done. Results written to {out_path}")


if __name__ == "__main__":
    main()
