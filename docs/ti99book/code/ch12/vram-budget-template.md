# VRAM budget worksheet (TMS9918A, 16 KiB) — Ch. 12 §12.7

Fill one of these before placing a single byte in VRAM. The base registers work
in **coarse units**, so a table can only start where its register can express —
reconcile that here, at a desk, not on screen. Tables **must not overlap** unless
you mean them to (the classic bug: a sprite attribute table on top of the color
table). Copy this file per project.

## Base-register granularity (what each register can point at)

| Table              | Register | Formula                     | Granularity |
|--------------------|----------|-----------------------------|-------------|
| name table         | R2       | `(R2 & >0F) << 10`          | 1 KiB       |
| color table (G1)   | R3       | `R3 << 6`                   | 64 B        |
| pattern table (G1) | R4       | `(R4 & >07) << 11`          | 2 KiB       |
| sprite attributes  | R5       | `(R5 & >7F) << 7`           | 128 B       |
| sprite patterns    | R6       | `(R6 & >07) << 11`          | 2 KiB       |

(Bitmap/Graphics II reinterprets R3/R4 as mask+select — see Ch. 15.)

## Plan

```text
VRAM budget — <mode> (16 KiB)              base register       size
  name table        >____ – >____          R2 = >__            ___ B
  sprite attributes >____ – >____          R5 = >__            ___ B
  color table       >____ – >____          R3 = >__            ___ B
  pattern table     >____ – >____          R4 = >__            ___
  sprite patterns   >____ – >____          R6 = >__            ___
  --- free ---      >____ – >3FFF                              ___
```

## Checklist

- [ ] Every base lands on a legal boundary for its register's granularity.
- [ ] No two tables overlap (walk the address ranges in order).
- [ ] Sprite attribute table clear of the color table.
- [ ] Total ≤ 16 KiB; free region noted for buffers / double-buffer (Ch. 13).
- [ ] R1 mode bits + BL/IE decided; R7 fg|backdrop decided.

## Reference: standard Graphics I layout (the book's default)

```text
  name table        >0000 – >02FF          R2 = >00            768 B
  sprite attributes >0300 – >037F          R5 = >06            128 B
  color table       >0380 – >039F          R3 = >0E             32 B
  pattern table     >0800 – >0FFF          R4 = >01           2 KiB
  sprite patterns   >1000 – >17FF          R6 = >02           2 KiB
  --- free ---      >1800 – >3FFF                              10 KiB
```
