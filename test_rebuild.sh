#!/bin/bash

echo "Testing rebuild performance after theme changes..."
echo "================================================"

# Wait for initial build to complete
echo "Waiting for initial build to complete..."
while ! [ -f "target/debug/query-crafter" ]; do
    sleep 2
    echo -n "."
done
echo " Done!"

# Test 1: Touch main.rs (should trigger near-full rebuild)
echo -e "\nTest 1: Touching main.rs"
touch src/main.rs
time cargo build 2>&1 | tail -5
echo "---"

# Test 2: Change a color in theme_const.rs (should be minimal rebuild with our optimization)
echo -e "\nTest 2: Changing a color in theme_const.rs"
sed -i 's/Color::Rgb(97, 175, 239)/Color::Rgb(97, 175, 240)/g' src/theme_const.rs
time cargo build 2>&1 | tail -5
# Revert the change
sed -i 's/Color::Rgb(97, 175, 240)/Color::Rgb(97, 175, 239)/g' src/theme_const.rs
echo "---"

# Test 3: Change theme.rs delegation (should also be minimal)
echo -e "\nTest 3: Adding a comment to theme.rs"
echo "// Test comment" >> src/theme.rs
time cargo build 2>&1 | tail -5
# Remove the comment
sed -i '$ d' src/theme.rs
echo "---"

echo -e "\nRebuild tests completed!"