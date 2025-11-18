#!/bin/bash
# scripts/clean-v8.sh
# Cleans V8 build artifacts for testing from scratch
# NOTE: This is for DEVELOPMENT ONLY - end users don't need this

echo "Cleaning V8 build artifacts (development only)..."

# Remove rusty_v8 build artifacts from target/
cargo clean -p v8

# Remove downloaded static libraries from target/
rm -rf target/debug/gn_root
rm -rf target/debug/gn_out
rm -rf target/debug/build/v8-*
rm -rf target/debug/.fingerprint/v8-*

echo "V8 build cache cleaned. Next build will download fresh static library."
echo "NOTE: End users don't need this - V8 is embedded in distributed executable."
