#!/usr/bin/env python3
"""
Build a warmup peptide dataset from real protein sequences.

Usage:
    python3 build_warmup_peptides.py \\
        --protein-file /data/proteins.txt \\
        --output warmup_peptides.txt \\
        --target-count 100000 \\
        --min-len 7 --max-len 25 --seed 42
"""
import argparse, random, sys
from pathlib import Path

def main() -> None:
    p = argparse.ArgumentParser()
    p.add_argument("--protein-file", required=True)
    p.add_argument("--output", required=True)
    p.add_argument("--target-count", type=int, default=100_000)
    p.add_argument("--min-len", type=int, default=7)
    p.add_argument("--max-len", type=int, default=25)
    p.add_argument("--seed", type=int, default=42)
    args = p.parse_args()

    rng = random.Random(args.seed)

    print(f"Loading proteins from {args.protein_file} ...", flush=True)
    with open(args.protein_file) as fh:
        proteins = [line.strip() for line in fh if line.strip()]
    # Filter out proteins shorter than min_len
    proteins = [p for p in proteins if len(p) >= args.min_len]
    print(f"  {len(proteins):,} usable proteins", flush=True)

    seen: set[str] = set()
    peptides: list[str] = []

    # Sample until we hit target_count unique peptides.
    # Each iteration picks a random protein, then a random valid window.
    max_attempts = args.target_count * 20
    attempts = 0
    while len(peptides) < args.target_count and attempts < max_attempts:
        attempts += 1
        protein = rng.choice(proteins)
        length = rng.randint(args.min_len, min(args.max_len, len(protein)))
        start = rng.randint(0, len(protein) - length)
        pep = protein[start:start + length]
        if pep not in seen:
            seen.add(pep)
            peptides.append(pep)

    rng.shuffle(peptides)
    out = Path(args.output)
    out.write_text("\n".join(peptides) + "\n")
    print(f"Wrote {len(peptides):,} unique warmup peptides to {out}", flush=True)
    if len(peptides) < args.target_count:
        print(f"WARNING: only reached {len(peptides):,} unique peptides "
              f"(target was {args.target_count:,})", file=sys.stderr)

if __name__ == "__main__":
    main()
