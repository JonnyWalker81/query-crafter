#!/bin/bash

echo "Testing incremental build with db.rs changes..."
echo "============================================="

# Initial build
echo -e "\n1. Initial build:"
cargo build 2>&1 | grep "Compiling" | wc -l

# Test 1: No changes (should be 0)
echo -e "\n2. No-change rebuild (should be 0):"
cargo build 2>&1 | grep "Compiling" | wc -l

# Test 2: Modify db.rs
echo -e "\n3. Modifying db.rs..."
# Add a comment to db.rs
sed -i '1s/^/\/\/ Test comment\n/' src/components/db.rs
echo "Build after db.rs change (should only compile query-crafter):"
cargo build 2>&1 | tee /tmp/build_output.txt | grep "Compiling"
COMPILED_COUNT=$(grep -c "Compiling" /tmp/build_output.txt)
echo "Total crates compiled: $COMPILED_COUNT"

# Revert the change
sed -i '1d' src/components/db.rs

# Test 3: Check what was actually compiled
echo -e "\n4. Analyzing what was compiled:"
if [ $COMPILED_COUNT -gt 1 ]; then
    echo "WARNING: More than just query-crafter was recompiled!"
    echo "Compiled crates:"
    grep "Compiling" /tmp/build_output.txt
fi

# Clean up
rm -f /tmp/build_output.txt
