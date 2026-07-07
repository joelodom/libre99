# Memory-budget worksheet (the §5.7 form)

Copy this table into a new program's notes and fill the blank cells *before* writing code.
The ranges and sizes are the console memory map (Ch. 5 §5.1); the width/toll column is the
measured funnel cost (Ch. 5 §5.3). "Used"/"Free" are in bytes. The VRAM row is reserved
until you learn to spend it through the VDP ports (Ch. 12).

| Region | Range | Size | Width / toll | Claimed for | Used | Free | Notes |
|---|---|---|---|---|---|---|---|
| Console ROM | `>0000`–`>1FFF` | 8 K | 16-bit / 0 | — firmware | — | — | not yours |
| Low expansion | `>2000`–`>3FFF` | 8 K | 8-bit / +4 |  |  |  |  |
| DSR window | `>4000`–`>5FFF` | 8 K | 8-bit / +4 | — cards | — | — | Ch. 30 |
| Cartridge ROM | `>6000`–`>7FFF` | 8 K | 8-bit / +4 |  |  |  | code + constants |
| Scratchpad | `>8300`–`>83FF` | 256 B | 16-bit / 0 |  |  |  | mind the tenants (§5.2) |
| High expansion | `>A000`–`>FFFF` | 24 K | 8-bit / +4 |  |  |  | where programs live |
| VRAM | via VDP ports | 16 K | ports only | *reserved* | — | — | budgeted from Ch. 12 |

Guidance:
- Code that runs hot wants the scratchpad (stage it there at startup — §5.5) or, failing that,
  cartridge ROM; constants burn into ROM; variables need RAM (expansion or pad).
- Workspaces belong in the scratchpad if at all possible — every register touch is a memory
  access, so a workspace across the funnel taxes nearly every instruction (§5.3).
- Claim only scratchpad the console is not using (§5.2), or mask interrupts and know what you
  turned off (Ch. 22).
