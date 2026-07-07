#!/usr/bin/env sh
# Assemble every .a99 under code/ with the project's own assembler (libre99asm)
# and build bench99. Nonzero exit on any failure. This is the check every
# writing session runs before a chapter is considered done.
#
# Requirements: cargo + POSIX sh (Git Bash on Windows). No make, no Python.
set -e
cd "$(dirname "$0")"
ROOT=../..

cargo build --release -p libre99-asm --manifest-path "$ROOT/Cargo.toml" >/dev/null
XAS="$ROOT/target/release/libre99asm"
mkdir -p build

fail=0
count=0
for f in code/ch*/*.a99; do
    [ -e "$f" ] || continue
    count=$((count + 1))
    if "$XAS" "$f" -o build/_v.ctg >build/_v.log 2>&1; then
        echo "  ok   $f"
    else
        echo "  FAIL $f"
        cat build/_v.log
        fail=1
    fi
done

echo "== building bench99 =="
if cargo build --release --manifest-path code/bench/Cargo.toml >build/_v.log 2>&1; then
    echo "  ok   code/bench"
else
    echo "  FAIL code/bench"
    cat build/_v.log
    fail=1
fi

rm -f build/_v.ctg build/_v.log
if [ "$fail" -eq 0 ]; then
    echo "All $count assembly source(s) assembled and bench99 builds."
else
    echo "VERIFICATION FAILED"
    exit 1
fi
