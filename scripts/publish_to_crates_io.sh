#!/bin/bash
# Publish ELARA crates to crates.io in dependency order
# Run from repo root: ./scripts/publish_to_crates_io.sh
# Requires: cargo, crates.io account, cargo login

set -e
cd "$(dirname "$0")/.."

echo "=== Publishing ELARA crates to crates.io ==="
echo "Order: elara-core → elara-wire → elara-transport"
echo ""

# 1. elara-core (no internal deps)
echo ">>> Publishing elara-core..."
cargo publish -p elara-core
echo ""

# 2. elara-wire (depends on elara-core)
echo ">>> Publishing elara-wire..."
cargo publish -p elara-wire
echo ""

# 3. elara-transport (depends on elara-core, elara-wire)
echo ">>> Publishing elara-transport..."
cargo publish -p elara-transport
echo ""

echo "=== All crates published successfully ==="
echo "elara-core, elara-wire, elara-transport are now on crates.io"
