#!/bin/bash

echo "Testing incremental build after removing theme module..."
echo "======================================================="

# Ensure clean state
echo "1. Building baseline..."
cargo build 2>&1 | tail -3

# Test 1: No-change rebuild (should be instant)
echo -e "\n2. No-change rebuild:"
time cargo build 2>&1 | grep -E "Compiling|Finished" || echo "Already up to date"

# Test 2: Modify db.rs
echo -e "\n3. Modifying db.rs..."
echo "// Test incremental build" >> src/components/db.rs

echo "Building after db.rs change:"
time cargo build 2>&1 | tee /tmp/db_build.log | grep -E "Compiling|Finished"

COMPILED=$(grep -c "Compiling" /tmp/db_build.log || echo "0")
echo "Crates compiled: $COMPILED"

if [ "$COMPILED" -eq 1 ]; then
    echo "✓ SUCCESS: Only query-crafter was recompiled!"
else
    echo "✗ ISSUE: $COMPILED crates were recompiled"
    grep "Compiling" /tmp/db_build.log | head -5
fi

# Revert
git checkout -- src/components/db.rs

# Test 3: Modify theme crate
echo -e "\n4. Modifying theme crate..."
echo "// Test theme" >> query-crafter-theme/src/lib.rs

echo "Building after theme change:"
time cargo build 2>&1 | tee /tmp/theme_build.log | grep -E "Compiling|Finished"

THEME_COMPILED=$(grep -c "Compiling" /tmp/theme_build.log || echo "0")
echo "Crates compiled: $THEME_COMPILED"

if [ "$THEME_COMPILED" -eq 2 ]; then
    echo "✓ EXPECTED: Theme crate and query-crafter recompiled"
else
    echo "Unexpected result: $THEME_COMPILED crates compiled"
fi

# Revert
git checkout -- query-crafter-theme/src/lib.rs

rm -f /tmp/db_build.log /tmp/theme_build.log