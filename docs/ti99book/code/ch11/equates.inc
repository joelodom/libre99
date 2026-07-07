* EQUATES.INC — the console's magic addresses, named ONCE.            (Ch. 11)
* Copyright (c) 2026 Joel Odom. Licensed under the Modified MIT License
* with Commons Clause — see LICENSE.md at the repository root.
*
* The book's first shared include (S11.3). Every hardware address the
* machine forces on us — the VDP ports, the sound and speech and GROM
* ports, the memory-map landmarks, the peripheral CRU bases — is given a
* name here, in ONE place, so no program ever hard-codes a bare >8C02
* again. A source file pulls the whole table in with a single line:
*
*        COPY 'equates.inc'
*
* NOTE (libre99asm COPY convention, verified Ch. 11): libre99asm resolves a
* COPY path relative to the DIRECTORY OF THE SOURCE FILE being assembled,
* not the working directory. syschk.a99 lives in code/ch11/ beside this
* file, so it writes  COPY 'equates.inc'  (a bare filename), even though
* the assembler is invoked from docs/ti99book/ as
*        libre99asm code/ch11/syschk.a99 ...
*
* This file has NO entry label and NO code — only EQU definitions — so it
* is not a stand-alone program (and the .inc extension keeps verify.sh's
* code/ch*/*.a99 sweep from trying to assemble it on its own).
*
* Values are the book's canon (see manuscript/_ledger.md, Ch. 2 MMIO row).

* ============ CPU RAM and memory-map landmarks =================
CONROM EQU  >0000            console system ROM (8 K, 16-bit, 0-wait)
LOWEXP EQU  >2000            low  32K expansion RAM  >2000-3FFF (8-bit)
DSRWIN EQU  >4000            peripheral DSR ROM window >4000-5FFF
CARTRM EQU  >6000            cartridge ROM window     >6000-7FFF
HIEXP  EQU  >A000            high 32K expansion RAM   >A000-FFFF (8-bit)

* -------- the 256-byte scratchpad "fast island" >8300-83FF --------
PAD    EQU  >8300            scratchpad base (16-bit, 0-wait)
PADEND EQU  >83FF            scratchpad last byte
GPLST  EQU  >837C            GPL status byte (console/GPL tenant)
GPLWS  EQU  >83E0            GPL workspace (WP) — leave it alone

* ============ VDP — TMS9918A video processor ports =============
* The chip hangs off the HIGH byte of the multiplexed bus: drive the
* two-byte port sequences with BYTE instructions, one transfer each.
VDPRD  EQU  >8800            VDP read  data   (returns a prefetched byte)
VDPST  EQU  >8802            VDP read  status (side-effect: clears flags)
VDPWD  EQU  >8C00            VDP write data
VDPWA  EQU  >8C02            VDP write address / register (two-byte seq)

* ============ SN76489 sound chip (write-only) =================
SNDWR  EQU  >8400            sound port (write-only)

* ============ Speech synthesiser (not emulated) ===============
SPCHRD EQU  >9000            speech read
SPCHWR EQU  >9400            speech write

* ============ GROM / GRAM ports ==============================
GRMRD  EQU  >9800            GROM read  data  (auto-increments, slow)
GRMRA  EQU  >9802            GROM read  address
GRMWD  EQU  >9C00            GROM write data  (GRAM only)
GRMWA  EQU  >9C02            GROM write address

* ============ Peripheral CRU (DSR) bases ======================
* A card's DSR ROM is paged into DSRWIN when its CRU base is loaded into
* R12 and bit 0 is set (SBO 0). Bases run >1000..>1F00 in >0100 steps.
* (Recall the bit address is HALF the R12 value: base >1100 -> bit >0880.)
DSRLO  EQU  >1000            first peripheral CRU base
DSRHI  EQU  >1F00            last  peripheral CRU base
DSRSTP EQU  >0100            spacing between adjacent CRU bases

* ============ Border-verdict scaffold (book-wide, Ch. 7) ======
* Paint VDP register 7 (backdrop): write the colour byte, then VR7SET.
GREEN  EQU  >0200            medium green (high half) — PASS
RED    EQU  >0600            dark red     (high half) — FAIL
VR7SET EQU  >8700            "write VDP register 7" command byte (high half)
