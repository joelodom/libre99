#!/usr/bin/env sh
# One-time toolchain bring-up for the TI-99/4A book project.
#
# The book is developed inside the libre99 repository and its toolchain
# IS the project's own: `libre99asm` (the TMS9900 assembler), `bench99` (the lab
# bench over the emulator core), and the `libre99` desktop app. All of
# it builds from source with cargo — nothing is downloaded.
#
# Requirements: a Rust toolchain (rustup.rs). POSIX sh (Git Bash on Windows).
set -e
cd "$(dirname "$0")"
ROOT=../..    # the libre99 repository root

echo "== building libre99asm (the assembler) and the desktop emulator =="
cargo build --release -p libre99-asm -p libre99-app --manifest-path "$ROOT/Cargo.toml"

echo "== building bench99 (the book's lab bench) =="
cargo build --release --manifest-path code/bench/Cargo.toml

# Smoke test: the first-light program must assemble.
mkdir -p build
"$ROOT/target/release/libre99asm" code/ch03/hello.a99 --name 'HELLO, 1981' \
    -o build/_smoke.ctg >/dev/null
rm -f build/_smoke.ctg
echo "OK — toolchain ready (hello.a99 assembles). Run 'sh verify.sh' to check all chapter code."
