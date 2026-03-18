# Unipept API Benchmark Suite

## Overview

This benchmark suite characterises the performance of the Unipept API's memory-mapped index loading
(controlled by the `--mmap` CLI flag).  The index consists of three binary files
(`sa.bin`, `proteins.bin`, `mapping.bin`) totalling ~400 GB.  Three benchmarks are provided:

| # | Name | Purpose |
|---|------|---------|
| 1 | **Cache Warmup Stability** | How many peptides must be processed before cold-start latency stabilises after a page-cache flush |
| 2 | **RAM Limit vs Response Time** | At what RAM budget does mmap gracefully degrade vs in-memory loading OOM-killing the process |
| 3 | **Complete vs Sparse SA** | End-to-end latency comparison between the complete and sparse suffix arrays, with and without mmap |

All benchmarks call `POST /api/v2/pept2lca` and record per-batch metrics to
[JSON Lines](https://jsonlines.org/) (`.jsonl`) files.  Separate analysis scripts generate
publication-quality plots from the raw files.

---

## Prerequisites

### Build the API

```bash
cargo build --release
# Binary: ./target/release/unipept-api
```

### Python environment

```bash
cd benchmarks/
python3 -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt
```

### Server requirements

- Linux with cgroup v2 (required for Benchmark 2 only)
- `cgroup-tools` package (`cgexec`): `apt-get install cgroup-tools`
- Root access for page-cache drops and cgroup writes
- A plain-text peptide list (one per line) accessible on the server, e.g. `/data/peptides_1M.txt`
- Sufficient disk and RAM: the full index is ~400 GB; you need at least as much RAM as your
  smallest cgroup limit when testing in-memory mode

### CLI flags reference

```
./target/release/unipept-api \
    --index-location <path>      # Directory containing sa.bin, proteins.bin, mapping.bin
    --database-address <dsn>     # PostgreSQL DSN or equivalent
    --port <port>                # HTTP port (default used in examples: 8080)
    --mmap <true|false>          # Load index via mmap (true) or fully into RAM (false)
```

---

## Benchmark 1: Cache Warmup Stability

### What it measures

After a page-cache flush, the OS must page in index data on demand.  This causes elevated response
times and major page faults in the first N requests.  This benchmark measures:

- How response time evolves over successive batches
- The **stability point**: the cumulative peptide count at which rolling coefficient of variation
  of response time drops below 5%
- The rate of major page faults, which correlates with OS paging activity

### Server setup (manual steps)

Run these commands on the server **before** each invocation of the benchmark script:

```bash
# 1. Flush the page cache
sudo sync && echo 1 | sudo tee /proc/sys/vm/drop_caches

# 2. Start the API (example: in-memory mode)
./target/release/unipept-api \
    --index-location /index \
    --database-address "postgres://user:pass@localhost/unipept" \
    --port 8080 \
    --mmap false &
API_PID=$!
```

### Running the script

```bash
# Run without mmap (in-memory loading)
python3 bench_warmup.py \
    --peptide-file /data/peptides_1M.txt \
    --api-pid $API_PID \
    --batch-size 100 \
    --num-batches 500 \
    --mmap false \
    --sa-type complete \
    --output results/warmup_nommap.jsonl

# Restart API with mmap, drop caches, then run again
kill $API_PID
sudo sync && echo 1 | sudo tee /proc/sys/vm/drop_caches
./target/release/unipept-api \
    --index-location /index \
    --database-address "postgres://user:pass@localhost/unipept" \
    --port 8080 \
    --mmap true &
API_PID=$!

python3 bench_warmup.py \
    --peptide-file /data/peptides_1M.txt \
    --api-pid $API_PID \
    --batch-size 100 \
    --num-batches 500 \
    --mmap true \
    --sa-type complete \
    --output results/warmup_mmap.jsonl
```

All options:

| Flag | Default | Description |
|------|---------|-------------|
| `--peptide-file` | required | Path to newline-delimited peptide list |
| `--api-pid` | required | PID of the running API process |
| `--api-url` | `http://localhost:8080` | API base URL |
| `--batch-size` | `100` | Peptides per request |
| `--num-batches` | `500` | Total batches to send |
| `--mmap` | `false` | Metadata: whether API was started with `--mmap true` |
| `--sa-type` | `complete` | Metadata: `complete` or `sparse` |
| `--equate-il` | `true` | Pass `equate_il` to the API |
| `--cgroup-path` | none | Path to cgroup directory for `memory.current` sampling |
| `--output` | required | Output `.jsonl` file |

### Interpreting the output

Run `analyze_warmup.py` (see [Generating Plots](#generating-plots)).  The top panel shows the
rolling-average response time; a vertical dashed line marks the stability point.  The bottom
panel shows per-batch major page faults — these should drop to near zero once the working set is
warm.

Key questions to answer:
- Does in-memory loading (`--mmap false`) converge faster or slower than mmap?
- At what cumulative peptide count is latency predictable?

---

## Benchmark 2: RAM Limit vs Response Time

### What it measures

This benchmark sweeps a series of memory limits using a Linux cgroup.  For each limit the API is
started inside the cgroup, a fixed number of batches are sent, and metrics are recorded.  The goal
is to find the **minimum RAM budget** that still gives acceptable latency:

- With `--mmap false`, the API attempts to load the entire ~400 GB index into RAM; limits below
  ~400 GB will cause an OOM kill (the partial `.jsonl` file is still valid and useful).
- With `--mmap true`, the OS page-fault mechanism allows the API to survive any RAM limit, but
  latency increases as the working set exceeds available RAM.

### One-time cgroup setup

Run once as root on the server:

```bash
sudo bash cgroups_setup.sh
```

This creates `/sys/fs/cgroup/unipept_bench`, enables the memory controller, and verifies that
`cgexec` is installed.

### Running the sweep

`bench_ramlimit.py` is fully automated: it sets the cgroup limit, drops the page cache, starts the
API, runs batches, stops the API, and repeats for each limit.  **Must be run as root.**

```bash
# mmap enabled
sudo python3 bench_ramlimit.py \
    --api-binary ./target/release/unipept-api \
    --index-location /index \
    --database-address "postgres://user:pass@localhost/unipept" \
    --port 8080 \
    --mmap true \
    --sa-type complete \
    --peptide-file /data/peptides_1M.txt \
    --ram-limits-gb 32 64 96 128 192 256 384 0 \
    --num-batches 200 \
    --batch-size 100 \
    --output-dir results/

# mmap disabled (expect OOM kills at lower limits)
sudo python3 bench_ramlimit.py \
    --api-binary ./target/release/unipept-api \
    --index-location /index \
    --database-address "postgres://user:pass@localhost/unipept" \
    --port 8080 \
    --mmap false \
    --sa-type complete \
    --peptide-file /data/peptides_1M.txt \
    --ram-limits-gb 32 64 96 128 192 256 384 0 \
    --num-batches 200 \
    --batch-size 100 \
    --output-dir results/
```

Output files are named `results/ramlimit_<limit>_<mmap|nommap>.jsonl`.

All options:

| Flag | Default | Description |
|------|---------|-------------|
| `--api-binary` | required | Path to the compiled API binary |
| `--index-location` | required | Index directory |
| `--database-address` | required | Database DSN |
| `--port` | `8080` | API HTTP port |
| `--mmap` | `true` | `--mmap` value passed to the API |
| `--sa-type` | `complete` | Metadata only |
| `--peptide-file` | required | Peptide list |
| `--ram-limits-gb` | `32 64 96 128 192 256 384 0` | RAM limits to sweep (0 = unlimited) |
| `--num-batches` | `200` | Batches per RAM limit |
| `--batch-size` | `100` | Peptides per batch |
| `--api-ready-timeout` | `300` | Seconds to wait for API startup per limit |
| `--output-dir` | required | Directory for output files |

### Interpreting the output

Run `analyze_ramlimit.py` (see [Generating Plots](#generating-plots)).  The top panel shows
median and p95 latency vs RAM limit for mmap and no-mmap.  The bottom panel shows average major
page faults.  Missing data points indicate an OOM kill.

Key questions to answer:
- What is the minimum RAM that keeps p95 latency below an acceptable threshold with mmap?
- At what limit does page-fault overhead start dominating latency?

---

## Benchmark 3: Complete vs Sparse Suffix Array

### What it measures

This benchmark compares four combinations:

| Combination | Index directory | `--mmap` flag |
|-------------|----------------|---------------|
| complete × mmap | `/index-complete/` | `true` |
| complete × no-mmap | `/index-complete/` | `false` |
| sparse × mmap | `/index-sparse/` | `true` |
| sparse × no-mmap | `/index-sparse/` | `false` |

The sparse SA is smaller on disk (fewer entries); this means faster load and lower page-fault
pressure but potentially different LCA result coverage.  The benchmark isolates the **latency**
and **paging** dimensions.

### Index directory layout

Both index directories must exist on the server before running:

```
/index-complete/
    sa.bin
    proteins.bin
    mapping.bin

/index-sparse/
    sa.bin
    proteins.bin
    mapping.bin
```

### Running the four combinations

Run these commands in sequence.  After each run, kill the API, drop the page cache, and start the
API for the next combination.

```bash
# --- Combination 1: complete SA, mmap ---
sudo sync && echo 1 | sudo tee /proc/sys/vm/drop_caches
./target/release/unipept-api \
    --index-location /index-complete \
    --database-address "postgres://user:pass@localhost/unipept" \
    --port 8080 --mmap true &
API_PID=$!
python3 bench_sa_compare.py \
    --api-pid $API_PID --mmap true --sa-type complete \
    --peptide-file /data/peptides_1M.txt \
    --num-batches 200 --batch-size 100 \
    --output results/sa_complete_mmap.jsonl
kill $API_PID

# --- Combination 2: complete SA, no-mmap ---
sudo sync && echo 1 | sudo tee /proc/sys/vm/drop_caches
./target/release/unipept-api \
    --index-location /index-complete \
    --database-address "postgres://user:pass@localhost/unipept" \
    --port 8080 --mmap false &
API_PID=$!
python3 bench_sa_compare.py \
    --api-pid $API_PID --mmap false --sa-type complete \
    --peptide-file /data/peptides_1M.txt \
    --num-batches 200 --batch-size 100 \
    --output results/sa_complete_nommap.jsonl
kill $API_PID

# --- Combination 3: sparse SA, mmap ---
sudo sync && echo 1 | sudo tee /proc/sys/vm/drop_caches
./target/release/unipept-api \
    --index-location /index-sparse \
    --database-address "postgres://user:pass@localhost/unipept" \
    --port 8080 --mmap true &
API_PID=$!
python3 bench_sa_compare.py \
    --api-pid $API_PID --mmap true --sa-type sparse \
    --peptide-file /data/peptides_1M.txt \
    --num-batches 200 --batch-size 100 \
    --output results/sa_sparse_mmap.jsonl
kill $API_PID

# --- Combination 4: sparse SA, no-mmap ---
sudo sync && echo 1 | sudo tee /proc/sys/vm/drop_caches
./target/release/unipept-api \
    --index-location /index-sparse \
    --database-address "postgres://user:pass@localhost/unipept" \
    --port 8080 --mmap false &
API_PID=$!
python3 bench_sa_compare.py \
    --api-pid $API_PID --mmap false --sa-type sparse \
    --peptide-file /data/peptides_1M.txt \
    --num-batches 200 --batch-size 100 \
    --output results/sa_sparse_nommap.jsonl
kill $API_PID
```

All options for `bench_sa_compare.py`:

| Flag | Default | Description |
|------|---------|-------------|
| `--api-pid` | required | PID of the running API |
| `--api-url` | `http://localhost:8080` | API base URL |
| `--mmap` | `true` | Metadata: mmap setting of the running API |
| `--sa-type` | required | `complete` or `sparse` |
| `--peptide-file` | required | Peptide list |
| `--num-batches` | `200` | Total batches |
| `--batch-size` | `100` | Peptides per batch |
| `--equate-il` | `true` | Passed to the API |
| `--cgroup-path` | none | Optional cgroup dir for memory sampling |
| `--output` | required | Output `.jsonl` file |

### Interpreting the output

Run `analyze_sa_compare.py` (see [Generating Plots](#generating-plots)).  Two side-by-side
boxplots show response time and major page fault distributions for all four combinations.  Summary
statistics (median, p95, avg major faults) are also printed to stdout.

Key questions to answer:
- Does the sparse SA give meaningfully lower latency end-to-end despite fewer matches?
- Does the mmap overhead (page faults) outweigh the faster startup of the sparse SA?

---

## Output File Format

Each `.jsonl` file contains one JSON object per line.  All fields:

```json
{
  "meta": {
    "benchmark": "warmup",
    "mmap": false,
    "sa_type": "complete",
    "batch_size": 100,
    "api_url": "http://localhost:8080",
    "endpoint": "/api/v2/pept2lca",
    "equate_il": true,
    "tryptic": false,
    "run_id": "2026-03-18T14:00:00Z",
    "ram_limit_gb": null,
    "hostname": "benchserver01"
  },
  "batch_index": 42,
  "cumulative_peptides": 4200,
  "peptides_in_batch": 100,
  "wall_time_s": 0.341,
  "http_status": 200,
  "results_returned": 97,
  "process_rss_bytes": 85899345920,
  "process_vms_bytes": 102400000000,
  "page_faults_minor": 12042,
  "page_faults_major": 3,
  "cgroup_memory_current_bytes": null,
  "timestamp": "2026-03-18T14:05:22Z"
}
```

| Field | Description |
|-------|-------------|
| `meta.*` | Run-level metadata (constant across all records in the file) |
| `batch_index` | 0-based batch counter |
| `cumulative_peptides` | Total peptides sent so far (including this batch) |
| `wall_time_s` | Client-side elapsed time from request send to full response received |
| `http_status` | HTTP status code returned by the API |
| `results_returned` | Number of items in the JSON response array |
| `process_rss_bytes` | Resident set size of the API process at record time |
| `process_vms_bytes` | Virtual memory size of the API process |
| `page_faults_minor` | Per-batch delta of minor faults from `/proc/<pid>/stat` field 10 |
| `page_faults_major` | Per-batch delta of major (I/O) faults from `/proc/<pid>/stat` field 12 |
| `cgroup_memory_current_bytes` | Current bytes used by the cgroup (null if no cgroup) |
| `timestamp` | ISO-8601 UTC timestamp of the record |

---

## Generating Plots

All analysis scripts write plots to the `plots/` directory (created automatically).

```bash
# Benchmark 1 — warmup stability
python3 analyze_warmup.py \
    results/warmup_nommap.jsonl \
    results/warmup_mmap.jsonl \
    --output plots/warmup.png

# Benchmark 2 — RAM limit sweep
python3 analyze_ramlimit.py \
    --input-dir results/ \
    --output plots/ramlimit.png

# Benchmark 3 — SA type comparison
python3 analyze_sa_compare.py \
    results/sa_complete_mmap.jsonl \
    results/sa_complete_nommap.jsonl \
    results/sa_sparse_mmap.jsonl \
    results/sa_sparse_nommap.jsonl \
    --output plots/sa_compare.png
```

### Quick syntax check (no server required)

```bash
python3 -m py_compile bench_common.py
python3 -m py_compile bench_warmup.py
python3 -m py_compile bench_ramlimit.py
python3 -m py_compile bench_sa_compare.py
python3 -m py_compile analyze_warmup.py
python3 -m py_compile analyze_ramlimit.py
python3 -m py_compile analyze_sa_compare.py
```
