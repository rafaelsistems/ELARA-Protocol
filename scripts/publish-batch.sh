#!/bin/bash

# ELARA Protocol - Batch Publishing Script
# This script publishes ELARA crates in the correct dependency order

set -e

echo "🚀 ELARA Protocol Batch Publishing Script"
echo "========================================"

# Check if cargo is installed
if ! command -v cargo &> /dev/null; then
    echo "❌ Cargo is not installed. Please install Rust and Cargo."
    exit 1
fi

# Check if token is provided
if [ -z "$CARGO_REGISTRY_TOKEN" ]; then
    echo "❌ CARGO_REGISTRY_TOKEN environment variable is not set."
    echo "Please set it with: export CARGO_REGISTRY_TOKEN=your_token_here"
    exit 1
fi

# Crates in dependency order (bottom-up)
CRATES=(
    "elara-core"
    "elara-wire"
    "elara-transport"
    "elara-crypto"
    "elara-time"
    "elara-state"
    "elara-runtime"
    "elara-msp"
    "elara-visual"
    "elara-voice"
    "elara-diffusion"
)

# Function to update dependencies in Cargo.toml
update_dependencies() {
    local crate_dir="$1"
    echo "📦 Updating dependencies for $crate_dir"
    
    cd "$crate_dir"
    
    # Update path dependencies to version dependencies
    sed -i 's/elara-core = { path = "..\/elara-core" }/elara-core = "0.2.0"/g' Cargo.toml
    sed -i 's/elara-wire = { path = "..\/elara-wire" }/elara-wire = "0.2.0"/g' Cargo.toml
    sed -i 's/elara-crypto = { path = "..\/elara-crypto" }/elara-crypto = "0.2.0"/g' Cargo.toml
    sed -i 's/elara-time = { path = "..\/elara-time" }/elara-time = "0.2.0"/g' Cargo.toml
    sed -i 's/elara-state = { path = "..\/elara-state" }/elara-state = "0.2.0"/g' Cargo.toml
    sed -i 's/elara-transport = { path = "..\/elara-transport" }/elara-transport = "0.2.0"/g' Cargo.toml
    sed -i 's/elara-runtime = { path = "..\/elara-runtime" }/elara-runtime = "0.2.0"/g' Cargo.toml
    sed -i 's/elara-msp = { path = "..\/elara-msp" }/elara-msp = "0.2.0"/g' Cargo.toml
    sed -i 's/elara-visual = { path = "..\/elara-visual" }/elara-visual = "0.2.0"/g' Cargo.toml
    sed -i 's/elara-voice = { path = "..\/elara-voice" }/elara-voice = "0.2.0"/g' Cargo.toml
    sed -i 's/elara-diffusion = { path = "..\/elara-diffusion" }/elara-diffusion = "0.2.0"/g' Cargo.toml
    
    cd ..
}

# Function to test crate
test_crate() {
    local crate_name="$1"
    echo "🧪 Testing $crate_name"
    
    cd "crates/$crate_name"
    
    echo "  Running tests..."
    cargo test --release
    
    echo "  Running clippy..."
    cargo clippy -- -D warnings
    
    echo "  Checking formatting..."
    cargo fmt -- --check
    
    cd ../..
}

# Function to publish crate
publish_crate() {
    local crate_name="$1"
    echo "🚀 Publishing $crate_name"
    
    cd "crates/$crate_name"
    
    echo "  Packaging..."
    cargo package --list
    
    echo "  Publishing to crates.io..."
    cargo publish --token "$CARGO_REGISTRY_TOKEN"
    
    echo "  Waiting for availability..."
    sleep 30
    
    cd ../..
    
    echo "✅ Published $crate_name"
}

# Main execution
main() {
    echo "Starting ELARA Protocol batch publishing..."
    echo "Token: ${CARGO_REGISTRY_TOKEN:0:10}..."
    echo ""
    
    # Check if we're in the right directory
    if [ ! -f "Cargo.toml" ] || [ ! -d "crates" ]; then
        echo "❌ This script must be run from the ELARA Protocol root directory"
        exit 1
    fi
    
    # Update dependencies for all crates first
    echo "📋 Step 1: Updating dependencies..."
    for crate in "${CRATES[@]}"; do
        if [ -d "crates/$crate" ]; then
            update_dependencies "crates/$crate"
        fi
    done
    
    echo ""
    echo "🧪 Step 2: Testing all crates..."
    for crate in "${CRATES[@]}"; do
        if [ -d "crates/$crate" ]; then
            test_crate "$crate"
        fi
    done
    
    echo ""
    echo "🚀 Step 3: Publishing crates..."
    for crate in "${CRATES[@]}"; do
        if [ -d "crates/$crate" ]; then
            publish_crate "$crate"
        fi
    done
    
    echo ""
    echo "🎉 All crates published successfully!"
    echo ""
    echo "Published crates:"
    for crate in "${CRATES[@]}"; do
        echo "  ✅ $crate"
    done
    
    echo ""
    echo "Links:"
    for crate in "${CRATES[@]}"; do
        echo "  📦 https://crates.io/crates/$crate"
    done
}

# Handle script interruption
trap 'echo "❌ Script interrupted"; exit 1' INT TERM

# Run main function
main "$@"