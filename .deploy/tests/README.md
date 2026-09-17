# Deploy tests

The scripts in `.deploy/` have no other test coverage, and what they get wrong is the kind of thing
a mock gets wrong too. So these suites run against a real systemd and a real HAProxy, both at the
versions production runs.

```bash
.deploy/tests/run-tests.sh            # every suite, about seven minutes
.deploy/tests/run-tests.sh server     # install.sh and deploy.sh
.deploy/tests/run-tests.sh haproxy    # haproxy.sh
.deploy/tests/run-tests.sh rollout    # rollout.sh
```

Docker is the only requirement. The server suite runs its container `--privileged` so systemd can
boot; nothing else needs it.

## What each suite covers

| Suite | Runs against | Covers |
| --- | --- | --- |
| `server/suite.sh` | systemd 255 on Ubuntu 24.04, with a user manager and lingering | the first install, a deploy with no privilege, `check` and each failure it reports, the memory arm per variant, the binary swap under signal, rollback, the interrupt paths, and the per-host `READY_TIMEOUT` with the unit watch that makes a long one safe |
| `loadbalancer/haproxy-suite.sh` | HAProxy 2.8 | reading `show stat` by field name, multi-backend targets, the drain cycle, a transitional `UP 1/100` status, backups |
| `loadbalancer/rollout-suite.sh` | HAProxy 2.8 with two backends over three servers | the four phases, the lock, ordering, preflight, each failure-resolution path, cleanup, the record, an interrupt during an install, and the deadline each server asks for |

## Why containers rather than mocks

Every bug these suites caught was one a mock would have hidden:

* `systemctl --user` needs `XDG_RUNTIME_DIR`, which only exists when lingering is really enabled.
* `AmbientCapabilities=` makes a *user* unit fail to start, with exit 218. Nothing in the
  documentation says so.
* HAProxy's `status` field is not one word: it reads `UP 1/100` while a server is being checked back
  in, which shifted every field after it.
* A hard link is refused when the caller does not own the file, because `fs.protected_hardlinks` is
  on by default.
* `Type=exec` reports the exec, not readiness, so systemd calls a binary that exits at once started
  and a health wait watching only the port sits there until the deadline.

## Writing a case

`lib.sh` holds the assertions:

```bash
section "the case these assertions belong to"
check "what it should do"  "$(what it did)"  "what it should have been"
check_true "the file is there" test -f /some/path
check_absent "it did not roll back" '^ROLLBACK' /tmp/ssh.log
```

`check` never stops the suite, so one run reports everything that is wrong rather than the first
thing. `summary` at the end sets the exit status and repeats every failure, because a run is long
enough that the first one has scrolled away by the time it ends.

**Head every group with `section`, not `echo`.** A failure prints the section it is in, which is
what makes `FAIL exit non-zero` identify one case rather than one of twelve. A heading printed with
`echo` records nothing, so its failures would name no case.

**Use `check_absent` to assert something did not happen.** `grep -c` against `0` also passes when
the pattern is mistyped, when the file was never written, and when the command under test produced
no output at all — so it is the shape most likely to assert nothing. `check_absent` requires the
evidence to exist first. The one place a plain `check` is still right is where an empty file *is*
the assertion.

Two rules worth keeping:

1. **Make a new case fail first.** Every case here was run against the unfixed code and observed to
   fail. A case that has never failed is a case that may be asserting nothing — the swap test is the
   clearest example, and it drives the old two-move pair to prove the point.
2. **Reset what you changed.** `reset_fleet` in the rollout suite puts every server back in the pool
   before a case starts. Without it one case's leftovers fail the next, and the failure points at
   the wrong thing.

## In CI

`.github/workflows/deploy-tests.yml` runs them, in two jobs: the server suite, and the three load
balancer suites. It is path-filtered on `.deploy/**`, so a pull request that changes only Rust does
not spend several minutes proving nothing.

**Do not make it a required check in the ruleset.** A path-filtered workflow never starts on a pull
request that does not match, and a required check that never starts waits forever. `audit.yml` is
filtered the same way and carries the same warning.

`ci.yml` still runs `shellcheck` over every script here, on every pull request, because that needs
no container and catches the class of mistake that does not need one.
