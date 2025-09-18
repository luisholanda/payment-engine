#!/bin/bash

# Test script to validate all sample inputs and outputs for the payment engine CLI

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Build the project
echo -e "${YELLOW}Building payment engine...${NC}"
cargo build --release

# Test function
test_sample() {
    local sample_dir=$1
    local sample_name=$(basename "$sample_dir")

    echo -e "${YELLOW}Testing sample: $sample_name${NC}"

    # Run the CLI and capture output
    local actual_output=$(cargo run --release -- "$sample_dir/input.csv" 2>/dev/null)
    local expected_output=$(cat "$sample_dir/output.csv")

    # Compare outputs
    if [ "$actual_output" = "$expected_output" ]; then
        echo -e "${GREEN}✓ $sample_name passed${NC}"
        return 0
    else
        echo -e "${RED}✗ $sample_name failed${NC}"
        echo "Expected:"
        echo "$expected_output"
        echo "Actual:"
        echo "$actual_output"
        return 1
    fi
}

# Find and test all samples
failed_tests=0
total_tests=0

echo -e "${YELLOW}Running all sample tests...${NC}"
echo

for sample_dir in samples/*/; do
    if [ -f "$sample_dir/input.csv" ] && [ -f "$sample_dir/output.csv" ]; then
        total_tests=$((total_tests + 1))
        if ! test_sample "$sample_dir"; then
            failed_tests=$((failed_tests + 1))
        fi
    fi
done

echo
echo "========================================="
echo "Test Results:"
echo "Total tests: $total_tests"
echo "Passed: $((total_tests - failed_tests))"
echo "Failed: $failed_tests"

if [ $failed_tests -eq 0 ]; then
    echo -e "${GREEN}All tests passed! 🎉${NC}"
    exit 0
else
    echo -e "${RED}Some tests failed! ❌${NC}"
    exit 1
fi
