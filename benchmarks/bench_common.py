"""Shared utilities for the Unipept API benchmark suite."""

from __future__ import annotations

import json
import os
import socket
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, List, Optional

import psutil
import requests

# ---------------------------------------------------------------------------
# HTTP helpers
# ---------------------------------------------------------------------------

def wait_for_api(api_url: str, timeout_s: int = 120, poll_interval_s: float = 2.0) -> None:
    """Block until the API responds to a health probe or timeout is reached."""
    deadline = time.monotonic() + timeout_s
    while time.monotonic() < deadline:
        try:
            r = requests.get(f"{api_url}/api/v2/pept2lca", timeout=5)
            # Any HTTP response (even 4xx) means the server is up.
            return
        except requests.exceptions.ConnectionError:
            time.sleep(poll_interval_s)
    raise TimeoutError(f"API at {api_url} did not become ready within {timeout_s}s")


def post_pept2lca(
    api_url: str,
    peptides: List[str],
    equate_il: bool = True,
    timeout_s: float = 120.0,
) -> tuple[float, int, int]:
    """
    POST to /api/v2/pept2lca and return (wall_time_s, http_status, results_returned).
    """
    payload = {"input": peptides, "equate_il": equate_il}
    t0 = time.monotonic()
    resp = requests.post(
        f"{api_url}/api/v2/pept2lca",
        json=payload,
        headers={"Content-Type": "application/json"},
        timeout=timeout_s,
    )
    wall_time_s = time.monotonic() - t0
    results_returned = len(resp.json()) if resp.ok else 0
    return wall_time_s, resp.status_code, results_returned


# ---------------------------------------------------------------------------
# Process / cgroup metrics
# ---------------------------------------------------------------------------

def read_proc_stat_faults(pid: int) -> tuple[int, int]:
    """
    Read (minflt, majflt) from /proc/<pid>/stat.
    Fields are 0-indexed: field 9 = minflt, field 11 = majflt.
    """
    with open(f"/proc/{pid}/stat") as fh:
        fields = fh.read().split()
    return int(fields[9]), int(fields[11])


def read_cgroup_memory(cgroup_path: str) -> Optional[int]:
    """Return memory.current in bytes, or None if path does not exist."""
    p = Path(cgroup_path) / "memory.current"
    if not p.exists():
        return None
    return int(p.read_text().strip())


def get_process_mem(pid: int) -> tuple[int, int]:
    """Return (rss_bytes, vms_bytes) for *pid*."""
    proc = psutil.Process(pid)
    mem = proc.memory_info()
    return mem.rss, mem.vms


# ---------------------------------------------------------------------------
# Record building
# ---------------------------------------------------------------------------

def build_record(
    *,
    benchmark: str,
    mmap: bool,
    sa_type: str,
    batch_size: int,
    api_url: str,
    equate_il: bool,
    run_id: str,
    ram_limit_gb: Optional[float],
    batch_index: int,
    cumulative_peptides: int,
    peptides_in_batch: int,
    wall_time_s: float,
    http_status: int,
    results_returned: int,
    api_pid: int,
    prev_minflt: int,
    prev_majflt: int,
    cgroup_path: Optional[str],
) -> Dict[str, Any]:
    """Assemble one benchmark record dictionary."""
    rss, vms = get_process_mem(api_pid)
    minflt, majflt = read_proc_stat_faults(api_pid)

    return {
        "meta": {
            "benchmark": benchmark,
            "mmap": mmap,
            "sa_type": sa_type,
            "batch_size": batch_size,
            "api_url": api_url,
            "endpoint": "/api/v2/pept2lca",
            "equate_il": equate_il,
            "tryptic": False,
            "run_id": run_id,
            "ram_limit_gb": ram_limit_gb,
            "hostname": socket.gethostname(),
        },
        "batch_index": batch_index,
        "cumulative_peptides": cumulative_peptides,
        "peptides_in_batch": peptides_in_batch,
        "wall_time_s": wall_time_s,
        "http_status": http_status,
        "results_returned": results_returned,
        "process_rss_bytes": rss,
        "process_vms_bytes": vms,
        "page_faults_minor": minflt - prev_minflt,
        "page_faults_major": majflt - prev_majflt,
        "cgroup_memory_current_bytes": read_cgroup_memory(cgroup_path) if cgroup_path else None,
        "timestamp": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
    }


def write_record(record: Dict[str, Any], out_path: Path) -> None:
    """Append *record* as a JSON line to *out_path* (creates file if needed)."""
    out_path.parent.mkdir(parents=True, exist_ok=True)
    with out_path.open("a") as fh:
        fh.write(json.dumps(record) + "\n")


# ---------------------------------------------------------------------------
# Peptide loading
# ---------------------------------------------------------------------------

def load_peptides(peptide_file: str) -> List[str]:
    """Load peptides from a plain-text file (one peptide per line)."""
    with open(peptide_file) as fh:
        peptides = [line.strip() for line in fh if line.strip()]
    if not peptides:
        raise ValueError(f"No peptides found in {peptide_file}")
    return peptides


def make_run_id() -> str:
    """Return an ISO-8601 UTC timestamp string suitable as a run identifier."""
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
