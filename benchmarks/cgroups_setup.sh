#!/usr/bin/env bash
# cgroups_setup.sh — One-time cgroup v2 setup for Unipept API benchmarks.
# Must be run as root on a Linux machine with cgroup v2 mounted at /sys/fs/cgroup.
#
# Usage:
#   sudo bash cgroups_setup.sh
#
# What it does:
#   1. Verifies cgroup v2 is available.
#   2. Creates the /sys/fs/cgroup/unipept_bench subtree.
#   3. Enables the memory controller for the subtree.
#   4. Disables swap to ensure memory limits are hard.
#   5. Verifies cgexec is installed (from cgroup-tools).

set -euo pipefail

CGROUP_ROOT="/sys/fs/cgroup"
BENCH_CGROUP="${CGROUP_ROOT}/unipept_bench"

# ---------------------------------------------------------------------------
# 1. Check cgroup v2
# ---------------------------------------------------------------------------
if ! grep -q "cgroup2" /proc/mounts 2>/dev/null; then
    echo "ERROR: cgroup v2 does not appear to be mounted at ${CGROUP_ROOT}."
    echo "       Ensure your kernel and init system use unified cgroup hierarchy."
    exit 1
fi
echo "[cgroups_setup] cgroup v2 detected."

# ---------------------------------------------------------------------------
# 2. Create the benchmark cgroup subtree
# ---------------------------------------------------------------------------
if [ -d "${BENCH_CGROUP}" ]; then
    echo "[cgroups_setup] ${BENCH_CGROUP} already exists — skipping creation."
else
    mkdir -p "${BENCH_CGROUP}"
    echo "[cgroups_setup] Created ${BENCH_CGROUP}."
fi

# ---------------------------------------------------------------------------
# 3. Enable the memory controller
#    The memory controller must be delegated from the parent cgroup.
# ---------------------------------------------------------------------------
PARENT_CONTROLLERS="${CGROUP_ROOT}/cgroup.subtree_control"
if grep -q "memory" "${PARENT_CONTROLLERS}"; then
    echo "[cgroups_setup] Memory controller already enabled in root subtree."
else
    echo "+memory" > "${PARENT_CONTROLLERS}"
    echo "[cgroups_setup] Enabled memory controller in root subtree."
fi

# Verify the bench cgroup has the memory controller available
if [ -f "${BENCH_CGROUP}/memory.max" ]; then
    echo "[cgroups_setup] memory.max is available in ${BENCH_CGROUP}."
else
    echo "ERROR: memory.max not found in ${BENCH_CGROUP}."
    echo "       Try enabling the memory controller: echo '+memory' > ${CGROUP_ROOT}/cgroup.subtree_control"
    exit 1
fi

# Set initial limit to unlimited
echo "max" > "${BENCH_CGROUP}/memory.max"
echo "[cgroups_setup] Set memory.max to 'max' (unlimited) as initial state."

# ---------------------------------------------------------------------------
# 4. Disable swap to enforce hard memory limits
# ---------------------------------------------------------------------------
echo 0 > "${BENCH_CGROUP}/memory.swap.max" 2>/dev/null || true
SWAPPINESS=$(cat /proc/sys/vm/swappiness)
if [ "${SWAPPINESS}" -gt 0 ]; then
    echo "[cgroups_setup] Current vm.swappiness=${SWAPPINESS}."
    echo "[cgroups_setup] To disable swap system-wide for this session: sysctl vm.swappiness=0"
    echo "[cgroups_setup] (Not done automatically to avoid disrupting other workloads.)"
fi

# ---------------------------------------------------------------------------
# 5. Check that cgexec is available
# ---------------------------------------------------------------------------
if command -v cgexec &>/dev/null; then
    echo "[cgroups_setup] cgexec found: $(command -v cgexec)"
else
    echo "WARNING: cgexec not found."
    echo "         Install it with: apt-get install cgroup-tools"
    echo "         (Required by bench_ramlimit.py to start the API inside the cgroup.)"
fi

echo ""
echo "[cgroups_setup] Setup complete."
echo "  Cgroup path : ${BENCH_CGROUP}"
echo "  memory.max  : $(cat ${BENCH_CGROUP}/memory.max)"
echo ""
echo "You can now run bench_ramlimit.py as root."
