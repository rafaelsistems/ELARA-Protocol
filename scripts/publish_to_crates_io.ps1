# Publish ELARA crates to crates.io in dependency order
# Run from repo root: .\scripts\publish_to_crates_io.ps1
# Requires: cargo, crates.io account, cargo login

$ErrorActionPreference = "Stop"
Push-Location $PSScriptRoot\..

Write-Host "=== Publishing ELARA crates to crates.io ==="
Write-Host "Order: elara-core -> elara-wire -> elara-transport"
Write-Host ""

# 1. elara-core (no internal deps)
Write-Host ">>> Publishing elara-core..."
cargo publish -p elara-core
Write-Host ""

# 2. elara-wire (depends on elara-core)
Write-Host ">>> Publishing elara-wire..."
cargo publish -p elara-wire
Write-Host ""

# 3. elara-transport (depends on elara-core, elara-wire)
Write-Host ">>> Publishing elara-transport..."
cargo publish -p elara-transport
Write-Host ""

Write-Host "=== All crates published successfully ==="
Write-Host "elara-core, elara-wire, elara-transport are now on crates.io"

Pop-Location
