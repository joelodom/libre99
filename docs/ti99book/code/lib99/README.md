# `lib99` — the book's accumulating assembly library

*Founded in Chapter 11 (Craftsmanship). This file is the formalization the Ch. 11 lab calls for:
the repo layout, the module conventions, the versioning story, and the test-harness discipline that
every `lib99` module obeys. The rationale lives in the chapter; this is the contract.*

## What `lib99` is

The reader's growing collection of **trusted, tested** 9900 routines — pulled out of the programs
that first needed them, documented once, and reused by name thereafter so no routine is ever written
(or debugged) twice. It is the concrete answer to the Ch. 11 vignette: a library you call instead of
a thicket you reread.

## Layout

```
code/lib99/
  README.md        this file — the library contract
  equates.inc      every magic console address, named once (the shared include; born Ch. 11)
  <module>.inc     the module's routines only — no START, COPY-includable by any consumer
  <module>.a99     the module's standalone self-test harness: COPYs its deps + .inc, adds START
```

**Composition (refined Ch. 13, when the first module built on another).** A module that a *second*
module needs cannot be a single file with its own `START` — two entry points collide. So a lib99
module is two files: a routines-only **`<module>.inc`** that any consumer `COPY`s, and a
**`<module>.a99`** self-test harness that `COPY`s the module's dependencies (in order), then the
`.inc`, then supplies a `START`. `verify.sh` assembles the `.a99` (proving the module) and skips the
`.inc` (pulled in by consumers). Example — `textlib` builds on `vdplib` builds on `equates`:

```
       COPY '../ch11/equates.inc'      ports + colors      (dependencies FIRST,
       COPY '../ch12/vdplib.inc'       the VDP core         in order — a flat
       COPY 'textlib.inc'              the text engine      COPY-namespace)
START  ...                             the consumer's code
```

Consumers pull dependencies in with `COPY`, **in dependency order** (there is no linker on our
baseline — a program is its entry file plus the includes it `COPY`s, assembled whole; §11.3).
**`COPY` is resolved relative to the directory of the source file being assembled**, so a sibling
include is a bare filename and a cross-directory one uses a relative path (`COPY '../ch12/vdplib.inc'`),
both verified. `memlib`/`mathlib` (Ch. 7–8) remain single-file for now — they compose nothing yet;
they gain a `.inc` split the first time a later module needs them.

## Module conventions (every module obeys)

1. **Header block** — purpose, copyright, how to build/run, and a **register-use map** for each public
   routine (which registers it reads / writes / destroys). Per §11.1.
2. **Calling convention R-16** — args/results in R0–R2 (caller-saved); a leaf keeps its return in R11
   and `RT`s; a non-leaf PUSHes R11 on the R10 software stack first; R10 and R13–R15 are never scratch;
   errors report in R0 (>0000 = success, nonzero = code).
3. **Single-file library + self-test** — the `mathlib`/`stack99` shape: the routines sit above a rule
   comment, and a `START` harness below it exercises them and paints the border-verdict light (GREEN
   >02 pass / RED >06 fail), so every module assembles stand-alone under `sh verify.sh` and proves
   itself on BENCH99. A companion `<module>.bench` script asserts the verdict off the pad (§11.6).
4. **Names, not magic numbers** — addresses come from `equates.inc`; a bare `>8C02` in a listing is a
   bug (§11.1).

## Versioning

`lib99` is versioned by the book itself: it grows one chapter at a time, and the library "as of
chapter N" is the set of modules that chapter has added. There is no separate version number — the
manuscript is the changelog.

## Founding modules

| Module | Born | Current home | Provides |
|---|---|---|---|
| `memlib`  | Ch. 7  | `code/ch07/memlib.a99`  | block move / fill / compare / scan (MEMCPY, MEMFIL, MEMCMP, MEMSCN) |
| `mathlib` | Ch. 8  | `code/ch08/mathlib.a99` | 32-bit add/sub/cmp, signed MPY/DIV wrappers, U16→decimal, LFSR |
| `equates.inc` / `SYSCHK` | Ch. 11 | `code/ch11/` | the shared include; the system-info card (first consumer) |

> **Note (owner decision pending):** the founding modules physically remain in their birth chapters'
> directories (`code/ch07/`, `code/ch08/`, `code/ch11/`) so their chapters' build/verify paths stay
> intact. Whether to *relocate* them into `code/lib99/` as a single source of truth — versus keeping
> the birth copies and adding new modules here — is a consolidation decision left to the author. The
> conventions above apply wherever a module lives.
