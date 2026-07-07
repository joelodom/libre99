# Notable bugs and their root causes (archive)

Root-cause write-ups of the hardest bugs found while building the emulator
core, preserved verbatim from the milestone-era `docs/STATUS.md`. They are
kept because each one encodes a subtle hardware truth that future work (or a
future port) could trip over again. Firmware-rewrite bug histories live in
[`original-content/system-roms/DEBUGGING.md`](../../original-content/system-roms/DEBUGGING.md)
(case studies 1–10); user-visible quirk analyses live in
[`docs/KNOWN-ISSUES.md`](../KNOWN-ISSUES.md).

---

## The boot blocker — GROM address-port read corrupted the next write

**Root cause (`crates/libre99-core/src/grom.rs`, `Grom::read_address`).** The GPL
interpreter's *branch* opcodes read the GROM address port (`>9802`) to recover
the current GROM slot, then immediately write the branch target to the address
port (`>9C02`). On the real TMC0430 an address-port **read** resets the
address-**write** byte selector, so the following two-byte write is still
interpreted high-then-low. Our `read_address` instead *toggled* the shared
`low_byte_phase`, so after an **odd** number of address reads the next address
write was mis-sequenced — the GPL branch landed on the wrong GROM address.

The result looked exactly like "operand-length drift": the branch
`>0658 → (slot|target)` set the counter one region off, so the interpreter ran
the wrong bytes as GPL bytecode, eventually dispatching an illegal opcode at
`>0C0C` and wedging.

**The fix.** `read_address` now matches the real chip (and Classic99): it is a
destructive read (high byte first, then the counter holds `low:low` so the
next read returns the low byte) **and it resets `low_byte_phase`** so a
subsequent address write starts with the high byte regardless of how many
address reads preceded it. Regression test:
`reading_address_does_not_corrupt_the_next_address_write` in `tests/grom.rs`.

The CPU was *not* at fault — the CPU conformance tests, the flag tables, and
the GROM data path were all correct (cross-checked against Classic99's
`cpu9900.cpp` status lookup tables and `Tiemul.cpp` GROM model). The earlier
"branches to `>100F`" observation was itself an artifact of this bug (the
post-bug counter logged `>100F` while the data actually came from a stale
prefetch).

---

## The disk-title blocker — a word write to the VDP double-incremented

**Symptom (found running `libre99-app`).** With the TI Disk Controller installed
the console's master title screen came up **blank cyan**, even though
everything past it (Tunnels of Doom, TI BASIC) drew fine and sound worked. The
bare console (no disk) was unaffected.

**Root cause (`crates/libre99-core/src/machine.rs`, the VDP port decode).** The
bus is byte-organized and a CPU **word** access is performed as two byte
transfers (`>8C00` then `>8C01`). Both addresses satisfied the old
`addr & 2 == 0` test, so a single word write to the VDP data port called
`Vdp::write_data` **twice** — writing two VRAM bytes and auto-incrementing the
address twice. The 9918A actually hangs off the high byte of the data bus, so
a word access reaches it **once**.

The disk DSR's power-up routine reserves a buffer at the top of VRAM (it
lowers `>8370` to `>37D7`) and clears it with a tight `CLR @>8C00` loop — a
*word* write, count `R1 = >0828 = 2088` (exactly `>37D8`‥`>3FFF`). At 2× per
iteration the loop cleared 4176 bytes, ran off the end of the 14-bit VRAM
address, **wrapped to `>0000`, and zeroed the name table** — wiping the title
the console had just drawn. This is why only the disk-controller boot was
affected (only its DSR clears VRAM with a word loop) and why it looked
timing/interrupt-related but wasn't.

**The fix.** The VDP read (`>8800–8BFF`) and write (`>8C00–8FFF`) ports now
respond **only at the even address**; the odd half of a word access is open
bus on read and ignored on write, with no latch and no auto-increment. This
matches the real chip and Classic99 (`rvdpbyte`/`wvdpbyte` bail out when
`x & 1`). Byte (`MOVB`) access — how the GPL interpreter normally drives the
VDP — is unchanged. Regression tests:
`disk_controller_still_draws_the_master_title_screen` (`tests/disk.rs`) and
`word_write_to_vdp_data_port_lands_one_byte` (`tests/machine.rs`).

GROM, sound, and speech sit on the high byte too and share the same latent
behavior, but no exercised path drives them with word access (the boot,
cartridge, and disk DSR all use byte access), so they were left as-is rather
than risk the working paths; revisit if evidence appears.

---

## Where later stories live

Two later, equally instructive investigations are written up in
[`docs/KNOWN-ISSUES.md`](../KNOWN-ISSUES.md) because their symptoms are
user-visible: the Parsec in-game text garble (root cause: the clean-room
GROM's stubbed `>004A` lower-case character-set loader — with the
beam-accurate scanline renderer built along the way), and Extended BASIC
typing uppercase (root cause: an uppercase unshifted keytab where the real
machine stores lowercase). The 2026-07-05 whole-project quality evaluation and
its executed remediation plan are preserved at
[QUALITY-EVALUATION-2026-07-05.md](QUALITY-EVALUATION-2026-07-05.md).
