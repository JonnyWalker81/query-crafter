#!/bin/bash

echo "Testing incremental build after theme fix..."
echo "==========================================="

# Ensure we have a baseline build
echo "1. Baseline build (if needed):"
cargo build 2>&1 | tail -3

# Test 1: No-change rebuild
echo -e "\n2. No-change rebuild (should be instant):"
time cargo build 2>&1 | grep -E "Compiling|Finished" || echo "Already up to date"

# Test 2: Modify db.rs
echo -e "\n3. Adding comment to db.rs..."
echo "// Test incremental build $(date)" >> src/components/db.rs

echo "Building after db.rs change:"
time cargo build 2>&1 | tee /tmp/db_build.log | grep -E "Compiling|Finished"

COMPILED=$(grep -c "Compiling" /tmp/db_build.log || echo "0")
echo "Crates compiled: $COMPILED"

if [ "$COMPILED" -eq 1 ]; then
    echo "✓ SUCCESS: Only query-crafter was recompiled!"
else
    echo "✗ ISSUE: Multiple crates were recompiled:"
    grep "Compiling" /tmp/db_build.log
fi

# Revert
git checkout -- src/components/db.rs

# Test 3: Modify theme crate
echo -e "\n4. Modifying theme crate..."
echo "// Test $(date)" >> query-crafter-theme/src/lib.rs

echo "Building after theme change:"
time cargo build 2>&1 | tee /tmp/theme_build.log | grep -E "Compiling|Finished"

THEME_COMPILED=$(grep -c "Compiling" /tmp/theme_build.log || echo "0")
echo "Crates compiled: $THEME_COMPILED"

# Revert
git checkout -- query-crafter-theme/src/lib.rs

# Cleanup
rm -f /tmp/db_build.log /tmp/theme_build.log