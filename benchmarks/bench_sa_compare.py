"""
Benchmark 3 — Complete vs Sparse Suffix Array
===============================================
Records per-batch metrics for one (sa_type, mmap) combination.
The API must already be running when this script starts.

Run this script four times (once per combination), restarting the API
between runs and dropping the page cache each time.

Usage
-----
# Start API (example for complete SA, mmap enabled):
#   sudo sync && echo 1 | sudo tee /proc/sys/vm/drop_caches
#   ./target/release/unipept-api --index-location /index-complete \
#       --database-address postgres://... --port 8080 --mmap true &
#   API_PID=$!

python3 bench_sa_compare.py \\
    --api-pid $API_PID \\
    --mmap true \\
    --sa-type complete \\
    --peptide-file /data/peptides_1M.txt \\
    --num-batches 200 \\
    --batch-size 100 \\
    --output results/sa_complete_mmap.jsonl

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
    p = argparse.ArgumentParser(description="Benchmark 3: Complete vs Sparse SA comparison")
    p.add_argument("--api-pid", required=True, type=int)
    p.add_argument("--api-url", default="http://localhost:8080")
    p.add_argument("--mmap", type=lambda x: x.lower() != "false", default=True,
                   help="Whether the API was started with --mmap true (metadata only)")
    p.add_argument("--sa-type", required=True, choices=["complete", "sparse"],
                   help="Suffix-array type of the running index")
    p.add_argument("--peptide-file", required=True)
    p.add_argument("--num-batches", type=int, default=200)
    p.add_argument("--batch-size", type=int, default=100)
    p.add_argument("--equate-il", type=lambda x: x.lower() != "false", default=True)
    p.add_argument("--cgroup-path", default=None)
    p.add_argument("--output", required=True)
    return p.parse_args()


def main() -> None:
    args = parse_args()
    out_path = Path(args.output)
    run_id = make_run_id()

    print(f"[sa_compare] run_id={run_id}  sa_type={args.sa_type}  mmap={args.mmap}  "
          f"pid={args.api_pid}  batches={args.num_batches}")

    print(f"[sa_compare] Waiting for API at {args.api_url} ...")
    wait_for_api(args.api_url)
    print("[sa_compare] API is ready.")

    all_peptides = load_peptides(args.peptide_file)
    print(f"[sa_compare] Loaded {len(all_peptides)} peptides.")

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
            print(f"[sa_compare] batch {batch_idx}: {exc}", file=sys.stderr)
            wall_time_s, status, n_results = float("nan"), 0, 0

        cumulative += len(batch)
        minflt, majflt = read_proc_stat_faults(args.api_pid)
        record = build_record(
            benchmark="sa_compare",
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
            print(f"[sa_compare] {batch_idx + 1}/{args.num_batches} batches  "
                  f"wall={wall_time_s:.3f}s  majflt_delta={record['page_faults_major']}")

    print(f"[sa_compare] Done. Results written to {out_path}")


if __name__ == "__main__":
    main()
