"""
Benchmark 2 — RAM Limit vs Response Time
==========================================
Sweeps a series of cgroup memory limits, starts the API under each limit,
runs a fixed number of batches, records metrics, then stops the API before
moving to the next limit.

Must be run as root (cgroup writes + cgexec).

Usage
-----
sudo python3 bench_ramlimit.py \\
    --api-binary ./target/release/unipept-api \\
    --index-location /index \\
    --database-address postgres://... \\
    --port 8080 \\
    --mmap true \\
    --sa-type complete \\
    --peptide-file /data/peptides_1M.txt \\
    --ram-limits-gb 32 64 96 128 192 256 384 0 \\
    --num-batches 200 \\
    --batch-size 100 \\
    --output-dir results/

See BENCHMARK.md for full setup instructions.
"""

from __future__ import annotations

import argparse
import itertools
import os
import signal
import subprocess
import sys
import time
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

CGROUP_DIR = Path("/sys/fs/cgroup/unipept_bench")
CGROUP_PROCS = CGROUP_DIR / "cgroup.procs"
MEMORY_MAX = CGROUP_DIR / "memory.max"
DROP_CACHES = Path("/proc/sys/vm/drop_caches")


def _gb_to_bytes(gb: float) -> int:
    return int(gb * 1024 ** 3)


def _set_memory_max(limit_gb: float) -> None:
    """Write memory.max into the unipept_bench cgroup."""
    if not CGROUP_DIR.exists():
        raise FileNotFoundError(
            f"{CGROUP_DIR} not found — run 'sudo bash cgroups_setup.sh' first"
        )
    value = "max" if limit_gb == 0 else str(_gb_to_bytes(limit_gb))
    MEMORY_MAX.write_text(value)


def _drop_page_cache() -> None:
    """Write 1 to /proc/sys/vm/drop_caches (requires root)."""
    DROP_CACHES.write_text("1")


def _start_api(
    api_binary: str,
    index_location: str,
    database_address: str,
    port: int,
    mmap: bool,
) -> subprocess.Popen:
    """Launch the API process inside the cgroup via cgexec."""
    cmd = [
        "cgexec", "-g", "memory:unipept_bench",
        api_binary,
        "--index-location", index_location,
        "--database-address", database_address,
        "--port", str(port),
        "--mmap", "true" if mmap else "false",
    ]
    proc = subprocess.Popen(
        cmd,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
    )
    return proc


def _stop_api(proc: subprocess.Popen) -> None:
    if proc.poll() is None:
        proc.send_signal(signal.SIGTERM)
        try:
            proc.wait(timeout=15)
        except subprocess.TimeoutExpired:
            proc.kill()


def parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser(description="Benchmark 2: RAM Limit vs Response Time")
    p.add_argument("--api-binary", required=True)
    p.add_argument("--index-location", required=True)
    p.add_argument("--database-address", required=True)
    p.add_argument("--port", type=int, default=8080)
    p.add_argument("--mmap", type=lambda x: x.lower() != "false", default=True)
    p.add_argument("--sa-type", default="complete", choices=["complete", "sparse"])
    p.add_argument("--peptide-file", required=True)
    p.add_argument("--ram-limits-gb", nargs="+", type=float,
                   default=[32, 64, 96, 128, 192, 256, 384, 0],
                   help="RAM limits in GB (0 = unlimited)")
    p.add_argument("--num-batches", type=int, default=200)
    p.add_argument("--batch-size", type=int, default=100)
    p.add_argument("--equate-il", type=lambda x: x.lower() != "false", default=True)
    p.add_argument("--api-ready-timeout", type=int, default=300,
                   help="Seconds to wait for the API to become ready after start")
    p.add_argument("--output-dir", required=True, help="Directory for output .jsonl files")
    return p.parse_args()


def run_one_limit(
    *,
    limit_gb: float,
    args: argparse.Namespace,
    all_peptides: list,
    run_id: str,
    output_dir: Path,
) -> None:
    label = "unlimited" if limit_gb == 0 else f"{int(limit_gb)}gb"
    mmap_label = "mmap" if args.mmap else "nommap"
    out_path = output_dir / f"ramlimit_{label}_{mmap_label}.jsonl"

    print(f"\n[ramlimit] === limit={label}  mmap={args.mmap} ===")

    _set_memory_max(limit_gb)
    _drop_page_cache()
    print(f"[ramlimit] Memory limit set to {label}, page cache dropped.")

    api_url = f"http://localhost:{args.port}"
    proc = _start_api(
        args.api_binary,
        args.index_location,
        args.database_address,
        args.port,
        args.mmap,
    )
    api_pid = proc.pid
    print(f"[ramlimit] API started (pid={api_pid})")

    try:
        wait_for_api(api_url, timeout_s=args.api_ready_timeout)
    except TimeoutError as exc:
        print(f"[ramlimit] {exc} — skipping this limit.", file=sys.stderr)
        _stop_api(proc)
        return

    print(f"[ramlimit] API ready.  Running {args.num_batches} batches ...")

    peptide_cycle = itertools.cycle(all_peptides)
    cumulative = 0
    prev_minflt, prev_majflt = read_proc_stat_faults(api_pid)

    for batch_idx in range(args.num_batches):
        # Check if API died (OOM kill)
        if proc.poll() is not None:
            print(f"[ramlimit] API process died at batch {batch_idx} "
                  f"(exit={proc.returncode}) — partial results saved.", file=sys.stderr)
            break

        batch = [next(peptide_cycle) for _ in range(args.batch_size)]
        try:
            wall_time_s, status, n_results = post_pept2lca(
                api_url, batch, equate_il=args.equate_il
            )
        except Exception as exc:
            print(f"[ramlimit] batch {batch_idx}: {exc}", file=sys.stderr)
            if proc.poll() is not None:
                print("[ramlimit] API died — stopping sweep for this limit.", file=sys.stderr)
                break
            wall_time_s, status, n_results = float("nan"), 0, 0

        cumulative += len(batch)
        minflt, majflt = read_proc_stat_faults(api_pid)
        record = build_record(
            benchmark="ramlimit",
            mmap=args.mmap,
            sa_type=args.sa_type,
            batch_size=args.batch_size,
            api_url=api_url,
            equate_il=args.equate_il,
            run_id=run_id,
            ram_limit_gb=limit_gb if limit_gb != 0 else None,
            batch_index=batch_idx,
            cumulative_peptides=cumulative,
            peptides_in_batch=len(batch),
            wall_time_s=wall_time_s,
            http_status=status,
            results_returned=n_results,
            api_pid=api_pid,
            prev_minflt=prev_minflt,
            prev_majflt=prev_majflt,
            cgroup_path=str(CGROUP_DIR),
        )
        write_record(record, out_path)
        prev_minflt, prev_majflt = minflt, majflt

        if (batch_idx + 1) % 50 == 0:
            print(f"[ramlimit] {batch_idx + 1}/{args.num_batches} batches  "
                  f"wall={wall_time_s:.3f}s  majflt_delta={record['page_faults_major']}")

    _stop_api(proc)
    # Reset memory limit to unlimited after each run
    _set_memory_max(0)
    print(f"[ramlimit] Done with limit={label}.  Results: {out_path}")


def main() -> None:
    if os.geteuid() != 0:
        print("ERROR: bench_ramlimit.py must be run as root.", file=sys.stderr)
        sys.exit(1)

    args = parse_args()
    output_dir = Path(args.output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)
    run_id = make_run_id()

    all_peptides = load_peptides(args.peptide_file)
    print(f"[ramlimit] Loaded {len(all_peptides)} peptides.  run_id={run_id}")

    for limit_gb in args.ram_limits_gb:
        run_one_limit(
            limit_gb=limit_gb,
            args=args,
            all_peptides=all_peptides,
            run_id=run_id,
            output_dir=output_dir,
        )

    print("\n[ramlimit] All sweeps complete.")


if __name__ == "__main__":
    main()
