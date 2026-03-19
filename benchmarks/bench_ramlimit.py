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
import random
import signal
import subprocess
import sys
import tempfile
import time
from pathlib import Path

from bench_common import (
    build_record,
    load_peptides,
    make_run_id,
    post_pept2lca,
    read_cgroup_memory,
    read_proc_stat_faults,
    wait_for_api,
    write_record,
)
from typing import Optional

CGROUP_DIR = Path("/sys/fs/cgroup/unipept_bench")
CGROUP_PROCS = CGROUP_DIR / "cgroup.procs"
MEMORY_MAX = CGROUP_DIR / "memory.max"
DROP_CACHES = Path("/proc/sys/vm/drop_caches")


def _fmt_elapsed(seconds: float) -> str:
    s = int(seconds)
    if s < 60:
        return f"{s}s"
    elif s < 3600:
        m, s = divmod(s, 60)
        return f"{m}m {s}s"
    else:
        h, rem = divmod(s, 3600)
        m = rem // 60
        return f"{h}h {m}m"


def _read_memory_limit_bytes(cgroup_dir: Path) -> Optional[int]:
    text = (cgroup_dir / "memory.max").read_text().strip()
    return None if text == "max" else int(text)


# UniProt Swiss-Prot amino acid frequencies (approximate %)
_AA = list("ACDEFGHIKLMNPQRSTVWY")
_AA_WEIGHTS = [8.25, 1.37, 5.45, 6.75, 3.86, 7.07, 2.27, 5.96, 5.84, 9.66,
               2.42, 4.06, 4.70, 3.93, 5.53, 6.56, 5.34, 1.08, 2.92, 6.87]

def _random_peptide() -> str:
    return "".join(random.choices(_AA, weights=_AA_WEIGHTS, k=random.randint(7, 25)))


def _warmup_cache(
    api_url: str,
    batch_size: int,
    equate_il: bool,
    cgroup_dir: Path,
    limit_label: str,
    warmup_peptide_file: Path | None = None,
) -> None:
    """
    Run unrecorded query batches of peptides until cgroup memory.current plateaus.
    Plateau = max−min of last WINDOW readings < 1 % of the memory limit.
    Skipped if no memory limit (unlimited).
    """
    limit_bytes = _read_memory_limit_bytes(cgroup_dir)
    if limit_bytes is None:
        print(f"[ramlimit] No memory limit — skipping cache warmup.")
        return

    WINDOW = 5          # number of readings to track
    THRESHOLD = 0.01    # 1 % of limit
    CHECK_INTERVAL = 45 # seconds between RSS readings
    MIN_CHECKS = 3      # minimum readings before plateau can trigger

    rss_history: list[int] = []
    start_time = time.monotonic()
    last_check = start_time

    print(
        f"[ramlimit] Warming up ...  rss = ? / {limit_bytes / 1e9:.1f} GB limit  "
        f"(elapsed {_fmt_elapsed(0)})",
        flush=True,
    )

    fh = open(warmup_peptide_file) if warmup_peptide_file else None

    while True:
        if fh:
            batch = []
            while len(batch) < batch_size:
                line = fh.readline()
                if not line:
                    fh.seek(0)
                    continue
                line = line.rstrip("\n")
                if line:
                    batch.append(line)
        else:
            batch = [_random_peptide() for _ in range(batch_size)]

        try:
            post_pept2lca(api_url, batch, equate_il=equate_il)
        except Exception:
            pass  # warmup best-effort; real errors will surface in the benchmark

        now = time.monotonic()
        if now - last_check < CHECK_INTERVAL:
            continue
        last_check = now

        rss = read_cgroup_memory(str(cgroup_dir))
        if rss is None:
            continue
        rss_history.append(rss)
        if len(rss_history) > WINDOW:
            rss_history.pop(0)

        elapsed = now - start_time
        print(
            f"[ramlimit] Warming up ...  "
            f"rss = {rss / 1e9:.1f} GB / {limit_bytes / 1e9:.1f} GB limit  "
            f"(elapsed {_fmt_elapsed(elapsed)})",
            flush=True,
        )

        if rss >= limit_bytes * (1 - THRESHOLD):
            print(f"[ramlimit] Cache full — starting benchmark.", flush=True)
            break

        if len(rss_history) >= MIN_CHECKS:
            delta = max(rss_history) - min(rss_history)
            if delta < THRESHOLD * limit_bytes:
                print(f"[ramlimit] Cache stable — starting benchmark.", flush=True)
                break

    if fh:
        fh.close()


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
        "--mmap" if mmap else "",
    ]
    stderr_log = Path(tempfile.mktemp(suffix=".api_stderr.log"))
    proc = subprocess.Popen(
        cmd,
        stdout=subprocess.DEVNULL,
        stderr=stderr_log.open("w"),
    )
    proc._stderr_log = stderr_log
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
    p.add_argument("--api-ready-timeout", type=int, default=3600,
                   help="Seconds to wait for the API to become ready after start")
    p.add_argument("--output-dir", required=True, help="Directory for output .jsonl files")
    p.add_argument("--warmup-peptide-file", default=None,
                   help="Pre-built warmup peptide file (one peptide per line). "
                        "If omitted, random peptides are generated instead.")
    return p.parse_args()


def run_one_limit(
    *,
    limit_gb: float,
    args: argparse.Namespace,
    all_peptides: list,
    run_id: str,
    output_dir: Path,
    warmup_peptide_file: Path | None = None,
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
        wait_for_api(api_url, timeout_s=args.api_ready_timeout, proc=proc)
    except TimeoutError as exc:
        print(f"[ramlimit] {exc} — skipping this limit.", file=sys.stderr)
        _stop_api(proc)
        return
    except RuntimeError as exc:
        print(f"[ramlimit] {exc} — skipping this limit.", file=sys.stderr)
        _stop_api(proc)
        return

    print(f"[ramlimit] API ready.  Running {args.num_batches} batches ...")

    _warmup_cache(
        api_url=api_url,
        batch_size=10_000,
        equate_il=args.equate_il,
        cgroup_dir=CGROUP_DIR,
        limit_label=label,
        warmup_peptide_file=warmup_peptide_file,
    )

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

        if (batch_idx + 1) % (args.num_batches // 10) == 0:
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

    warmup_peptide_file = Path(args.warmup_peptide_file) if args.warmup_peptide_file else None
    if warmup_peptide_file:
        print(f"[ramlimit] Using warmup peptide file: {warmup_peptide_file}")

    for limit_gb in args.ram_limits_gb:
        run_one_limit(
            limit_gb=limit_gb,
            args=args,
            all_peptides=all_peptides,
            run_id=run_id,
            output_dir=output_dir,
            warmup_peptide_file=warmup_peptide_file,
        )

    print("\n[ramlimit] All sweeps complete.")


if __name__ == "__main__":
    main()
