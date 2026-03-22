#!/bin/bash
# Run all fuzz targets in parallel

set -e

TARGETS="sinstr clone_drop eq ord collections"
PARALLEL="${PARALLEL:-1}"
TIMEOUT="${TIMEOUT:-60}"

echo "Running fuzz targets: $TARGETS"
echo "Parallel jobs: $PARALLEL"
echo "Timeout per target: ${TIMEOUT}s"
echo ""

# Kill all background jobs on exit
trap 'kill $(jobs -p) 2>/dev/null' EXIT

for target in $TARGETS; do
    echo "Starting: $target"
    timeout "$TIMEOUT" cargo fuzz run "$target" -- -max_total_time=$((TIMEOUT -5)) &
    
    # Limit parallelism
    while [ $(jobs -r | wc -l) -ge "$PARALLEL" ]; do
        sleep 1
    done
done

# Wait for all to complete
wait

echo ""
echo "All fuzz targets completed"