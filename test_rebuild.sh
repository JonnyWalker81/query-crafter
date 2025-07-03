#\!/bin/bash
cargo build 2>&1 | grep -c "Compiling"
sleep 1
cargo build 2>&1 | grep -c "Compiling"
