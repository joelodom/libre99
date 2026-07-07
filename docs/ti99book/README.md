# Programming the TI-99/4A — manuscript & companion code

**Version 0.0.1** — the book tracks the [Libre99](../../README.md) project
version; the single source of truth is the workspace `version` in the
repo-root `Cargo.toml`, and the book releases in lock-step with the toolchain
it documents. (The outline's `v1.x` amendments are editorial revisions, a
separate thing from this release number.)

A book-length guide to TMS9900 assembly and GPL programming on the TI-99/4A.
Work in progress, written one chapter per session with an AI writing partner —
**inside, and founded on, the [Libre99](../../README.md) project**: the
book's daily machine is this repository's emulator, its assembler is `libre99asm`,
its GPL toolchain is `libre99gpl`, and its debugging instrument is **BENCH99**
(`code/bench/`), a scriptable monitor over the same emulator core. Classic99,
js99er.net, MAME, and xdt99 are discussed throughout with their own roles.

## Picking up the work

Open this folder in **Claude Code** and say: *"Read CLAUDE.md and write the next chapter."*
Everything Claude needs to continue is in `CLAUDE.md` and `manuscript/`.

Manually (needs only `sh` + a Rust toolchain):
```
sh setup.sh       # once: build libre99asm, the emulator, and BENCH99 from this repo
sh verify.sh      # assemble all companion code + build the bench; must pass
```

## Status
**40 of 45 chapters drafted.** Complete: Part I, Part II (except Chapter 6, a
stub — narrative final, code work orders still open), and Parts III–VIII in
full. **Part IX** (the case studies) is underway — **Chapters 39–41** (METEOR
BELT, the scrolling shooter; GRIDRUNNER 99, the console-only maze chase;
DUNGEONS OF FATE, the data-driven RPG engine) are drafted; **Chapter 42**
(AUTHOR99, the productivity program) is next, then 43. Remaining after that:
**Part X** (44–45), the fourteen reference **appendices** (A–N), and the front
matter (preface, how-to-use). See
`manuscript/_summaries.md` for what each finished chapter covers, and
`manuscript/00-master-outline.md` for the full 45-chapter plan (amendments at
the end).

## Layout
- `manuscript/` — the book plus its three working files (`_style.md`, `_ledger.md`, `_summaries.md`)
- `code/` — `bench/` (BENCH99) and per-chapter assembly sources (machine-verified with `libre99asm`)
- `setup.sh`, `verify.sh`, `Makefile` — build & verification (sh + cargo only)

## Reading order for a new human collaborator
1. `manuscript/00-master-outline.md` (the plan — read the v1.1 amendment too)
2. `manuscript/_style.md` (the rules)
3. Any drafted chapter, e.g. `manuscript/ch03-the-workshop.md` (the foundation chapter)
