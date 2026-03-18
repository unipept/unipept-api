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

    seen: set[str] = set()
    peptides: list[str] = []

    print(f"Sampling peptides from {args.protein_file} ...", flush=True)
    while len(peptides) < args.target_count:
        with open(args.protein_file) as fh:
            for line in fh:
                protein = line.strip()
                if len(protein) < args.min_len:
                    continue
                length = rng.randint(args.min_len, min(args.max_len, len(protein)))
                start = rng.randint(0, len(protein) - length)
                pep = protein[start:start + length]
                if pep not in seen:
                    seen.add(pep)
                    peptides.append(pep)
                if len(peptides) >= args.target_count:
                    break

    rng.shuffle(peptides)
    out = Path(args.output)
    out.write_text("\n".join(peptides) + "\n")
    print(f"Wrote {len(peptides):,} unique warmup peptides to {out}", flush=True)
    if len(peptides) < args.target_count:
        print(f"WARNING: only reached {len(peptides):,} unique peptides "
              f"(target was {args.target_count:,})", file=sys.stderr)

if __name__ == "__main__":
    main()
