#!/bin/bash

echo "Testing db.rs incremental build issue..."
echo "========================================"

# First ensure we have a clean build
echo "Ensuring clean build state..."
cargo build --bin query-crafter 2>&1 | tail -5

# Test no-change rebuild
echo -e "\n1. Testing no-change rebuild:"
START=$(date +%s)
cargo build --bin query-crafter 2>&1 > /tmp/no_change.log
END=$(date +%s)
DIFF=$((END - START))
echo "Time: ${DIFF}s"
grep -c "Compiling" /tmp/no_change.log || echo "0"

# Test db.rs change
echo -e "\n2. Modifying db.rs (adding comment)..."
echo "// Test incremental build" >> src/components/db.rs

echo -e "\n3. Building after db.rs change:"
START=$(date +%s)
cargo build --bin query-crafter 2>&1 | tee /tmp/db_change.log | grep "Compiling" || true
END=$(date +%s)
DIFF=$((END - START))
echo "Time: ${DIFF}s"
CRATES_COMPILED=$(grep -c "Compiling" /tmp/db_change.log || echo "0")
echo "Crates compiled: $CRATES_COMPILED"

# Revert change
git checkout -- src/components/db.rs

# Analyze
echo -e "\n4. Analysis:"
if [ "$CRATES_COMPILED" -gt 1 ]; then
    echo "PROBLEM: db.rs change caused $CRATES_COMPILED crates to rebuild!"
    echo "Crates that were recompiled:"
    grep "Compiling" /tmp/db_change.log | head -10
else
    echo "GOOD: Only query-crafter was recompiled"
fi

# Check dependencies
echo -e "\n5. Checking what depends on db.rs:"
rg "mod db|use.*db::" src/ --type rust | grep -v "src/components/db.rs" | head -10

rm -f /tmp/no_change.log /tmp/db_change.log