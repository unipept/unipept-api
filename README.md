# Unipept API
![codecov](https://img.shields.io/codecov/c/github/unipept/unipept-api/develop)

This package is an implementation of the Unipept API that's being used by the Unipept Web Application and Desktop application to succesfully perform analysis of metaproteomics samples.

## Overview of existing endpoints
This is an exhaustive list of all endpoints that are exposed by this API

### Public endpoints
#### API v1
* `/api/v1/pept2taxa`
* `/api/v1/pept2lca`
* `/api/v1/taxa2lca`
* `/api/v1/pept2prot`
* `/api/v1/pept2funct`
* `/api/v1/pept2ec`
* `/api/v1/pept2go`
* `/api/v1/pept2interpro`
* `/api/v1/taxa2tree`
* `/api/v1/peptinfo`
* `/api/v1/taxonomy`
* `/api/v1/messages`

#### API v2
* `/api/v2/pept2taxa`
* `/api/v2/pept2lca`
* `/api/v2/taxa2lca`
* `/api/v2/pept2prot`x
* `/api/v2/pept2funct`
* `/api/v2/pept2ec`
* `/api/v2/pept2go`
* `/api/v2/pept2interpro`
* `/api/v2/taxa2tree`
* `/api/v2/peptinfo`
* `/api/v2/taxonomy`
* `/api/v2/messages`

### Private endpoints
* `/private_api/goterms`
* `/private_api/ecnumbers`
* `/private_api/interpros`
* `/private_api/taxa`
* `/private_api/taxa2rank`
* `/private_api/proteins`
* `/private_api/metadata`
* `/mpa/pept2data`
* `/datasets/sampledata`

## Choosing a storage backend

The index has two implementations of every structure — one holding owned memory, one borrowing a
memory mapping — and which one a binary uses is decided **when it is compiled**, not by a flag.
The features are forwarded to `sa-server`, which names one concrete type per structure; nothing
in the search path branches on them.

| build | what it does |
| --- | --- |
| *(no features)* | everything preloaded into owned memory. This is the default. |
| `--features mmap` | everything memory-mapped. **This is the production build.** |
| `--features mmap,preloaded-text` | mapped, except the protein text |
| `--features mmap,preloaded-proteins` | mapped, except the protein metadata |
| `--features mmap,preloaded-mapping` | mapped, except the suffix-to-protein mapping |

The three `preloaded-*` features combine freely, giving nine configurations in all. Each is a
no-op without `mmap`, where everything is preloaded already. There is no `preloaded-sa`: the
suffix array follows `mmap` and is roughly 72% of the index, so it dominates residency either way.

```
cargo build --release --features mmap
```

The running server reports what it was built with at startup, since there is no other way to
tell:

```
Index storage backend: sa=mmap text=preloaded proteins=mmap mapping=mmap
```

### Which one to use

Figures below are from [`sa-index/BENCHMARKS.md`](https://github.com/unipept/unipept-index) in
the index repository, measured on one machine against the 223 GB UniProt index. Read the shape,
not the absolute numbers.

**If the whole index is guaranteed resident in RAM**, preloading pays: against plain `mmap`,
`preloaded-proteins` is +22.1%, `preloaded-text,preloaded-proteins` is +46.1%, and the fully
preloaded build is +57.6%.

**If it is not, use plain `mmap`.** Preloaded memory is anonymous and cannot be reclaimed under
pressure. From the first memory ceiling that binds, no preloading arm beats `mmap` by more than
the noise floor, `preloaded-proteins` is behind it at every ceiling, and the fully preloaded
build is OOM-killed at every ceiling tested. Below roughly a third of the index the preloaded
arms do not degrade gracefully — they collapse, at tens of times `mmap`'s fault rate.

Since the choice is compiled in, it is a deployment decision rather than something a restart can
correct. When the memory ceiling might move, `mmap` is the safe build.

### RAYON_NUM_THREADS

Independent of the backend, and the largest single effect in the index's benchmark record. Under
a memory ceiling, raising the thread count well above the core count buys back +65.8% to +94.9%,
because a major page fault blocks its thread and each faulting thread otherwise idles a core.
With the index fully resident the same setting *costs* about 10%. A deployment knob, not a
default.

## Developing the Unipept API
You can use the included devcontainer in order to start working on this API.
The devcontainer will automatically download the most recent version of the Unipept Index built from SwissProt.
Follow these steps in order to easily work on the Unipept API in the devcontainer:

* You first have to build the binaries by running `cargo build --release`.
* Make the directory where we can store the logfiles for a running instance of the Unipept API: `mkdir -p /var/log/unipept-api`.
* Finally, the Unipept API can be started with this command: `./target/release/unipept-api -i "/unipept-index-data" -d "http://localhost:9200" -p 80 > /var/log/unipept-api/api.log 2> /var/log/unipept-api/api_error.log`.
