#!/bin/bash

echo "Testing incremental build performance..."
echo "======================================="

# Wait for initial build
echo "Waiting for initial build to complete..."
timeout 300 cargo build --bin query-crafter 2>&1 | tail -5

if [ ! -f "target/debug/query-crafter" ]; then
    echo "Initial build failed or timed out!"
    exit 1
fi

echo -e "\nInitial build complete!"

# Test 1: No changes (should be instant)
echo -e "\n1. Testing no-change rebuild..."
time cargo build --bin query-crafter 2>&1 | grep -E "Compiling|Finished"

# Test 2: Touch config.toml (should NOT trigger rebuild with our fix)
echo -e "\n2. Testing config.toml timestamp change..."
touch config.toml
time cargo build --bin query-crafter 2>&1 | grep -E "Compiling|Finished"

# Test 3: Change theme color (should only rebuild theme crate)
echo -e "\n3. Testing theme color change..."
sed -i 's/Color::Rgb(97, 175, 239)/Color::Rgb(97, 175, 240)/' query-crafter-theme/src/lib.rs
time cargo build --bin query-crafter 2>&1 | grep -E "Compiling|Finished"
# Revert
sed -i 's/Color::Rgb(97, 175, 240)/Color::Rgb(97, 175, 239)/' query-crafter-theme/src/lib.rs

# Test 4: Touch main.rs (should rebuild main crate)
echo -e "\n4. Testing main.rs touch..."
touch src/main.rs
time cargo build --bin query-crafter 2>&1 | grep -E "Compiling|Finished"

echo -e "\nIncremental build tests complete!"