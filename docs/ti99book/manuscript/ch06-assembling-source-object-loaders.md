# Chapter 6 — Assembling: Source, Object, and Loaders

*One program, three bodies — the source you write, the object the assembler emits, the image the
loader builds — and four roads between them.*

<!-- Part II — The TMS9900 and Assembly Fundamentals · target ≈24 pp -->
<!-- STATUS: STUB (Phase 2 populated, 2026-07-05) — narrative is final-voice; technical cores are OPUS-TODO work orders. Protocol: _stubs.md. -->
<!-- SPEC: 00-master-outline.md, "### Chapter 6 —" (outline v1.1, lines 232–244). This chapter is R-10's home: the xas99 + Classic99 period workflow is primary here BY DESIGN (libre99asm deliberately lacks tagged object / EA5 / DEF/REF — canon card); libre99asm returns as the lab's fourth leg. -->

## The Noise That Wasn't

A Friday evening in February 1983, Elkhart, Indiana. The algebra teacher has owned his TI-99/4A
for a year and the expansion system for three months — the console was a family Christmas present;
the Peripheral Expansion Box, with its 32K card and disk drive, he justified to himself one
school-supply order at a time. The Editor/Assembler package came last, and it is why supper is
going cold on the corner of the desk.

The loop goes like this. He types sixty lines into the Editor — a program that is supposed to sort
a gradebook — and saves them to disk. He backs out, brings up the Assembler, answers its questions
— source file, object file, listing — and the drive chatters through pass one, in which the
assembler reads everything and learns his names, then pass two, in which it reads everything again
and writes opcodes into a new file. Back out. Option 3. The object file's name. ENTER. And the
program dies with the cursor stuck in the wrong corner of a blank screen, because assembling
cleanly and working are different virtues. He has run this loop nine times tonight. The coffee has
gone cold twice.

Saturday morning, the kids parked in front of cartoons, he sits down meaning only to tidy the disk
— and instead does the idle thing that turns out to matter. There is a file on that disk he has
never actually looked at: the middle one, the object file, the thing the Assembler makes and
Option 3 eats. Machine code. He knows what machine code ought to look like — noise — but the
nine-pin printer he justified in September as a Christmas-card machine is sitting right there, so
he copies the file to it, expecting a page of static to show his son.

What comes out is columns. Capital letters and digits in tidy ranks, like attendance. His
program's name — he gave it one without thinking, the way you fill in any blank on any form — sits
legible near the head of the first line. He fetches the manual, the fat one that never quite fits
back in the box, and finds the appendix that admits, in plain tables, what every character of
those columns means: this letter says *place a word here*; this one says *here is a name other
files may ask for*; that group at the end of the line is a checksum, the file guarding itself
against the disk. The pencil comes out. By the second cup of coffee he has decoded a dozen words
of his own machine code by hand, tag by tag — and found, incidentally, where Friday night's cursor
bug lives, because reading a program that slowly is a kind of debugging nobody warns you about.

The page goes up on the corkboard over the desk, and it stays there for years — not because the
gradebook sorter ever quite worked, but because of what the page proved. There is no magic between
the text he types and the memory the machine runs. There is a file in the middle, and the file can
be read. Texas Instruments had chosen a middleman that speaks ASCII — an old habit, we will see,
carried down from its bigger machines — and then printed the decoder ring in a consumer manual any
family could buy.

This chapter is that Saturday morning, done properly: the grammar of the source file, the anatomy
of the object file, the loaders that consume each form — and the 1982 ritual, honored once on the
machine itself, so that you know exactly what your modern tools are saving you from.

---

## What You Will Learn

- Split any assembly source line into its four fields, and say which rules are assembler law and
  which are punch-card habit.
- Choose between absolute and relocatable organization — AORG against RORG — and predict the
  address every symbol receives under each.
- Read a tagged object file with no tool but your eyes, tag by tag, and explain what each record
  instructs a loader to do.
- Explain DEF and REF as linking done at load time: what the Editor/Assembler's loader patches,
  when, and what the REF/DEF table holds.
- Build and run one program four ways — Option 3 object, Option 5 image, assembled on the console
  itself, and a libre99asm cartridge — and state what each path proves.
- Give the contract — inputs, outputs, fine print — for the nine E/A utility names every period
  listing assumes, and the environment trap that comes with them.
- Place Mini Memory in the ecosystem: what the battery-backed cartridge offered a console-only
  owner, and what its line-by-line workflow cost.
- Organize a multi-file project two ways — COPY inclusion and separately assembled modules — and
  read listings and symbol maps as instruments instead of build exhaust.

## The Bridge: You Already Ship Object Files

Every toolchain you have ever used runs the same relay. A compiler or assembler translates source
into *object files* — incomplete machine code, salted with a symbol table and a to-do list of
*relocations*: "I could not finish this address; whoever places me, patch it." A linker merges
object files and settles the to-do list; a loader copies the finished image into memory and jumps.
ELF and PE dress the relay up in section headers and dynamic tables, but the relay itself —
translate, resolve, place, run — is decades older than the microprocessor, and the TI-99/4A runs
an unusually legible version of it. The Editor/Assembler's assembler emits object files with
symbols and relocation records; its loader is a *linking loader*, doing the linker's patch-work at
load time, much the way your dynamic linker resolves symbols the moment a library is opened; and
the EA5 "program image" is the fully settled result — a flat memory snapshot, the firmware blob of
its day.

One difference is a gift. Your ELF files are binary; you read them through `readelf` and
`objdump`, trusting the tool's translation. TI's tagged object format is ASCII text — records made
of printable tag characters and hexadecimal digits, designed for an era when object code had to
survive teletypes and paper handling. In §6.4 you will read relocation records with your naked
eyes, checksum and all. No modern format lets you get this close with this little equipment; it is
the best seminar on linking you will ever buy for the price of a text file.

A word on tools, because this chapter's stance is deliberate (R-12 governs it). Our own libre99asm
speaks none of these formats — no tagged object, no EA5, no DEF/REF — because its one target, the
cartridge image, is absolute code at a hardware-fixed address and needs no loader. That is the
right modern design, and it is useless for teaching 1982. So this chapter runs primarily on the
period-faithful pair: **xas99**, the xdt99 suite's cross-assembler, which speaks the period
formats natively; and **Classic99**, whose licensed bundle includes the Editor/Assembler cartridge
itself. Listings here are E/A-native unless flagged; constructs beyond E/A carry `[libre99asm]` or
`[xas99]` to say which dialect owns them (R-13). The xas99 conventions of R-10 govern the period
invocations; R-14 governs the lab's fourth leg, where libre99asm ships the same program as a modern
cartridge.

## 6.1 The Anatomy of a Source Line

You have been writing assembly since Chapter 3 the way children learn grammar — by imitation,
correctly, without the rules. Here are the rules. A source line holds up to four fields, in fixed
order: a **label**, naming the line's address (or, with EQU, a value); a **mnemonic** —
instruction or directive; its **operands**; and a **comment**, which the assembler discards
unread. Two rules are law. A label must begin in column 1 — and anything beginning in column 1
*is* a label, which is why an unlabeled instruction must be indented. And an asterisk in column 1
surrenders the whole line to commentary. Everything past the operand field, set off by blank
space, is commentary too — which is where a no-operand mnemonic gets treacherous: with no operand
expected, where does the comment begin? Our libre99asm settles that corner by requiring `;` before a
trailing comment on a no-operand mnemonic `[libre99asm]`; what each of the other assemblers does with
the same line is exactly the kind of dialect fine print the table below pins down.

Then there is the geometry you have surely noticed: real listings line their fields up in columns,
as if the page had ruled margins only assembly programmers can see. That is the 80-column ghost.
These fields earned fixed column positions in the punch-card era, when a field *was* a range of
columns, and the convention rode into the 1980s on the editors themselves — the E/A editor steered
typists toward the canonical stops. The assembler cares about far less than the columns suggest —
column 1 and field order, essentially — but two generations of listings keep the columns out of
habit, and so does this book, because code you can scan vertically is code you can review.

Names, finally. The classic Editor/Assembler symbol is at most six characters, the first
alphabetic — MSGLEN fits, MESSAGELENGTH does not — and that budget shaped a whole culture of
terse, percussive naming you will now start to read fluently. Our dialects relax the budget in
different ways, and this book flags any label that would not survive the classic assembler
`[libre99asm]` (R-13), because names travel: a routine that might ever be fed to the period toolchain
should carry a period-lawful name.

<!-- OPUS-TODO ch06-srcfields [table+listing]:
DELIVER: (1) a five-line annotated source fragment, every field labeled (a labeled instruction; an
  indented unlabeled one; a `*` comment line; an EQU; a no-operand mnemonic with a `;` trailing
  comment [libre99asm]); (2) the field-rules table: column-1 law, the E/A editor's tab stops vs what
  the assembler enforces, symbol charset/length limits per dialect (E/A / xas99 / libre99asm), case
  rules, and each assembler's no-operand trailing-comment behavior.
CODE: code/ch06/fields.a99 — must assemble under BOTH libre99asm and xas99 (-R) unmodified.
VERIFY: verify at HEAD (R-12) that libre99asm accepts the shared subset and confirm its symbol-length
  behavior empirically; xas99 legality against live xdt99 (ledger: session-2 verified flags
  -R/-i/-b/-L/-o; there is NO --version flag); E/A column rules and the 6-char limit page-cited to
  the E/A manual (repo copy) — do not trust memory.
LEDGER: rows for per-dialect symbol limits and the column-1 label rule, source-cited.
PROSE: fragment first, table after the 80-column paragraph; ≈150 words of reading-the-table; keep the punch-card framing. Budget ≈ ¾ page.
-->

## 6.2 Directives: Where, What, and How Much

A source file speaks to two readers. Instructions are for the CPU; **directives** are for the
assembler, spend no cycles, and emit no code of their own — they place, describe, reserve, name,
and terminate. The classic set sorts into five families. *Where:* AORG sets the location counter
to an absolute address you choose; RORG begins relocatable code — "assemble as if from zero, and
mark every address provisional"; DORG opens a dummy section, where the location counter advances
but nothing is emitted — the period tool for describing memory you don't own, laying out a record
or a buffer as named offsets. *What:* DATA plants words, BYTE plants bytes, TEXT plants
characters. *How much:* BSS and BES reserve space without emitting bytes, and differ in which end
of the block the label names — a distinction small enough to forget and sharp enough to cut (the
table pins it; Exercise 6.2 makes you bleed on it safely). *Names:* EQU christens a value; DEF
publishes a name for other object files to call; REF imports a name some other file must supply.
*Hygiene:* EVEN rounds the location counter up to a word boundary, and END closes the file —
optionally naming the entry point.

Behind the directory of directives sits the chapter's first big idea: **absolute versus
relocatable thinking.** Absolute code commits to its addresses at assembly time; you are the
placement algorithm. The cartridge world is absolute by nature — ROM answers at `>6000` because
the hardware says so (Ch. 5) — which is why libre99asm is an absolute-only assembler on purpose: AORG
is the only organization it speaks, and DEF, REF, RORG, DORG, and BES are simply not in its
vocabulary, because a cartridge needs no loader and makes no promises. Relocatable code defers the
decision: the assembler measures every address from a provisional zero and attaches the promise
list, and the loader — who alone knows what memory is free on the day — adds the base. One
program, assembled once, can land at `>A000` today and somewhere else entirely tomorrow.

Relocation earned its keep on this machine because object files were meant to *share*. A loader
packs modules into expansion RAM in whatever order they arrive; none of them can know their final
addresses, and none of them needs to. DEF and REF are the social contract that makes the packing
useful — this module offers SHOUT, that one requires it — and the resolution happens not in a
linker at build time but in the loader at load time, a fusion this chapter keeps returning to
(§6.4 shows the promise records themselves; §6.6 shows the loader honoring them).

<!-- OPUS-TODO ch06-directives [table+listing]:
DELIVER: the §6.2 directive reference table — AORG/RORG/DORG/BSS/BES/DATA/BYTE/TEXT/EQU/DEF/REF/
  EVEN/END (add IDT if the §6.4 dissection needs it): one-line semantics, operand form, which
  dialect implements it (E/A / xas99 / libre99asm), and the label's value for BSS vs BES. Plus a
  10–15 line demo assembled twice — once under AORG, once under RORG [xas99] — with the two
  listings' address columns shown side by side, so relocation is something the reader SEES.
CODE: code/ch06/reloc.a99 (xas99, both modes via -L listings); code/ch06/absol.a99 (the libre99asm
  AORG twin, assembles clean at HEAD).
VERIFY: verify at HEAD (R-12) libre99asm's exact supported subset (canon: no RORG/DORG/BES/DEF/REF —
  confirm, and quote once the actual error text a rejected directive produces); xas99 semantics
  from live xdt99 listings; E/A semantics page-cited to the manual, esp. BES label-at-end.
LEDGER: a row per directive-semantics fact asserted (BES/BSS label rule above all), source-cited.
PROSE: table mid-section; the side-by-side listing closes the section and hands off to §6.4 with
  the line "the loader finishes the sentence." Budget ≈ 1 page.
-->

## 6.3 Expressions, the Location Counter, and the Classic Idioms

Operand fields take more than bare numbers: they take **expressions** — symbols, constants, and
arithmetic — evaluated once, at assembly time, and therefore free at run time. The pivot of the
whole expression system is `$`, the location counter, the assembler's "you are here" sign. From it
flow the idioms you will meet in every period listing from now to the back cover: a length
computed as the distance between here and a label — the assembler does the counting, so the code
never lies about its own sizes; `JMP $` — jump to yourself, the idle loop you have already used
(it encodes to `>10FF`); DATA lists of routine addresses — dispatch tables, a habit later chapters
lean on hard; and the deliberately reserved hole, a labeled word set aside to be patched by code
or by loader.

The fine print is dialect fine print. Which operators exist, whether precedence is honored or the
expression simply runs left to right, whether parentheses are allowed at all — the three
assemblers in this chapter's life do not answer identically, and a program that assembles to
*different bytes* under two assemblers is a trap with a forty-year fuse. There is also an algebra
of relocatability: the difference of two relocatable symbols is an honest absolute distance, while
some combinations — a relocatable times anything, say — mean nothing and must be rejected. We pin
all of it empirically rather than folklorically.

<!-- OPUS-TODO ch06-idioms [listing+table]:
DELIVER: (1) the expression-rules table per assembler — operator set, precedence or left-to-right,
  parentheses yes/no, what a relocatable symbol may legally combine with; (2) a dozen-line idiom
  gallery, one idiom + one comment per line: length-by-subtraction (MSGLEN EQU $-MSG), JMP $,
  EVEN-before-DATA repair, a 4-entry DATA dispatch table, a labeled patch word.
CODE: code/ch06/idioms.a99 — assembles under BOTH libre99asm and xas99 -R; listings generated
  (--listing / -L) and eyeballed for identical bytes.
VERIFY: verify at HEAD (R-12) libre99asm's operator/precedence behavior with 3-line probe files
  (record accept/reject + resulting values from the listings); same probes through live xdt99;
  E/A's rules page-cited to the manual. JMP $ = >10FF is already ledgered — reuse, don't remeasure.
LEDGER: expression-rule rows per dialect; any precedence divergence gets its own citable row.
PROSE: gallery closes the section; keep the "you are here" framing. Budget ≈ ¾ page.
-->

## 6.4 The Tagged Object File: A Relic That Reads Itself

What the assembler emits, in the Editor/Assembler world, is not memory. It is a *description* of
future memory — a little program whose only interpreter is the loader, made of records that say
"place this word," "this address is provisional, add your base," "here is a name I export," "patch
this location with a name I could not resolve," "I end here." TI encoded those records as
printable text: each begins with a single **tag** character that announces its meaning, followed
by a fixed-width payload of hexadecimal digits. The format came down from TI's larger 990
minicomputer toolchain <!-- OPUS-VERIFY: 990 lineage of the tagged object format — confirm against
a primary or solid secondary source (E/A manual, 990 documentation, community record) and hedge to
match the evidence -->, an environment of paper tape and printing terminals where "object code you
can read at the teletype" was not a luxury but an operating condition.

Hold on to why this matters beyond charm. The format has three audiences. The loader, obviously.
The student — you — because linking is usually taught as an act of faith in binary tools, and here
every mechanism of it sits on the page in capital letters and digits: you can watch a relocation
be *promised*. And the archaeologist: a text format is self-disclosing, and disks recovered after
decades give up their contents to anyone holding the tag table. The dissection below walks one
complete object file — a real one, assembled from a program small enough to hold in your head —
line by line, tag by tag, checksum and all.

> **Field Notes — Archaeology with a text editor.** Preservationists sorting a shoebox of
> unlabeled 1980s disks meet this format's kindness immediately: object files announce themselves.
> The dissection below shows the module's own name riding at the head of the file in plain ASCII,
> and DEF'd entry names — a library's public vocabulary — legible without any tool fancier than
> something that can show a file's characters. Formats that read themselves outlive their tools:
> the assembler that wrote such a file may be forty years gone, but the file explains itself to
> anyone who finds the tag table — printed in a manual that itself survives (see the sidebar in
> §6.6). When you design file formats of your own (Part VII), remember who your last reader might
> be.

There is also a **compressed object** variant — the same records, packed denser and less charming,
a concession to disk space and load speed. It changes the encoding, not the ideas; one paragraph
in the dissection settles it.

<!-- OPUS-TODO ch06-tagged-dissect [dissection]:
DELIVER: the chapter's museum piece — a complete tagged object file (~6 records) reproduced
  verbatim in a ```text fence, dissected line by line: every tag named and explained (header/IDT,
  absolute vs relocatable data, address, DEF and REF, checksum, end-of-record and end-of-file).
  Then the full tag reference table, then one paragraph on compressed object (what changes, why
  Option 3 still reads it). Also emit code/ch06/mystery.obj — a second tiny program, object file
  only, source NOT printed in the book — for Ex. 6.5/6.10.
CODE: code/ch06/tagdemo.a99 → xas99 -R -o build/tagdemo.obj; keep the program ≤8 instructions with
  one DEF and one relocatable data word so every interesting tag appears once.
VERIFY: tag meanings cross-checked BOTH ways — E/A manual's object-format section (page-cite) AND
  the actual bytes xas99 emitted; where tool and manual disagree, say so in a one-line aside.
  Recompute one line's checksum by hand and show the arithmetic.
LEDGER: rows for each tag character asserted, the checksum rule, and the compressed-object delta.
PROSE: let the dissection breathe — this is the section the vignette promised; keep the "relic
  that reads itself" frame. Budget ≈ 2 pages including fence and table.
-->

## 6.5 Program Images: EA5 and the Verbatim Slab

The tagged format's philosophical opposite also shipped in the same box. An **EA5 program image**
asks the loader to do no thinking at all: a header of a few words, then a verbatim slab of future
memory, copied in and jumped to. No symbols, no relocation, no promises — every decision already
made at build time, every address already final. On 1982 disk hardware that bluntness was the
point: images loaded fast, and they loaded the same way every time, which is why so much software
that had outgrown the hobbyist workflow reportedly shipped this way when it didn't ship as a
cartridge.

Constraint bred convention. An image bigger than the loader's appetite splits into a chain of
files — the header carries a "more follows" flag, and continuation files take a naming convention
so the loader can find its own next course — and execution, once the last file lands, begins by an
autostart rule rather than by anyone typing a program name. The period route to an image was
usually a harvest: load a program the slow way once, then run a SAVE utility that wrote settled
memory back out as image files <!-- OPUS-VERIFY: the SAVE-utility harvest route and its
SFIRST/SLAST/SLOAD linkage — page-cite the E/A manual before asserting any detail -->. The modern
route skips the ceremony: xas99 emits the image straight from source with a switch. Header layout,
splitting rule, naming convention, autostart — all pinned by the order below, from real bytes.

You have, incidentally, already met this idea wearing modern clothes: an EA5 image is
`objcopy -O binary` with a six-byte hat, and libre99asm's cartridge output is the same species —
fully settled bytes, everything resolved at build time, down to synthesizing the cartridge's
`>6000` header for you. The difference is only *who* fixes the address: for a cartridge, the
hardware; for an EA5 file, the header itself. What happens when a program outgrows any single slab
is Part VIII's problem, and banking is its answer (Ch. 34–35).

<!-- OPUS-TODO ch06-ea5-dissect [dissection]:
DELIVER: (1) the EA5 header anatomy — build the lab program as an image (xas99 -R -i over the
  lab's E/A top), hexdump the opening bytes into a ```text fence, and table the header words
  (continuation flag, length, load address — establish the true order/meaning from evidence);
  (2) the multi-file convention: how a too-big image splits and how continuation files are named —
  demonstrated with a real split if a teachable size can force one, else manual-cited; (3) the
  autostart rule, manual-cited and observed via Option 5; (4) a two-sentence resolution of the
  SAVE/SFIRST/SLAST/SLOAD OPUS-VERIFY in the narrative above.
CODE: code/ch06/hello4ea.a99 → build/HELLO4 image file(s); hexdump excerpt in the fence.
VERIFY: header words read from actual bytes (od/hexdump), cross-checked against the E/A manual AND
  proven by Classic99 loading the image via Option 5 to a working screen.
LEDGER: rows for header layout, naming convention, autostart rule.
PROSE: lands after the "verbatim slab" paragraph; ≈½ page + fence + table.
-->

## 6.6 The Editor/Assembler Cartridge: The 1982 Ritual, Honored Properly

The Editor/Assembler package was the machine's professional credential: a cartridge, diskettes,
and the manual the sidebar below spends time with. The cartridge alone is not the whole workshop —
it contributes the menu and the loaders, while the editor and the assembler themselves arrive as
programs on diskette <!-- OPUS-VERIFY: the cartridge/disk division of labor (menu + loaders in the
cartridge; editor and assembler loaded from disk) — confirm against the manual and the running
cartridge before asserting -->; the package presumes the full standard system — console, 32K,
disk — which is this book's baseline anyway. What it bought you was the loop our vignette's
teacher lived: edit, assemble, load, despair, repeat.

Formally, the loop has three stations. *Edit:* source lives as plain text files on disk, made and
remade in the editor. *Assemble:* the classic two passes — pass one reads the whole program to
build the symbol table, pass two reads it again to emit object code and, if asked, a listing —
with the assembler prompting for file names and a small set of option letters (the transcript in
the work order shows the exact dialogue). *Load:* Option 3 reads tagged object, adds bases to
relocatable words, honors promises — and keeps prompting for further files, so a multi-module
program chains in — then asks for a *program name* before anything runs.

That last prompt is the DEF machinery surfacing where you can touch it. As the loader digests DEF
records it builds the **REF/DEF table** — external names paired with their finally settled
addresses. Typing a name at the prompt is a table lookup and a branch; that is the entire sense in
which a DEF'd name "becomes callable." The same table is how a REF in a later file finds its
answer in an earlier one — and, far from now, it is how Extended BASIC's CALL LINK will find
assembly routines by name, in this book's single scheduled page of BASIC (Ch. 36). Where the table
lives in memory and what a record looks like, the order pins from evidence, because that address
will be load-bearing later.

Option 5 is the loop's other exit: it runs the program images of §6.5 — no names, no patching, no
questions beyond a file name. Dumber and faster, and the lab makes the difference something you
have felt in your own hands rather than read in a table.

<!-- OPUS-TODO ch06-ea-ritual [transcript+facts]:
DELIVER: the E/A cartridge's actual menu text (quoted exactly); the Option 3 dialogue (file
  prompt, additional-file loop, program-name prompt); the Option 5 dialogue; the assembler's
  prompts and option letters (confirm the real set); the editor's on-disk source record format;
  and the REF/DEF table mechanics — memory location, record shape (name + address), how the
  program-name prompt consults it.
VERIFY: verify at HEAD (R-12) whether the project's embedded media include the E/A cartridge (and
  any usable E/A disk among the 15 embedded disks); if yes, run the ritual on the project emulator
  and say so; if not, run it on Classic99's licensed bundle (its canon shelf role) and state the
  gap plainly. REF/DEF table location page-cited to the E/A manual AND peeked live (Classic99's
  debugger, or BENCH99 `m` if the cartridge is embedded) — both, since Ch. 36 anchors on it.
LEDGER: rows for menu strings, prompts, assembler options, REF/DEF table location and record shape.
PROSE: weave evidence into the existing narrative — replace hedges, keep wording. Budget ≈ 1¼ pages.
-->

> **Sidebar — The Most-Thumbed Four Hundred–Odd Pages in TI History.** The *Editor/Assembler*
> manual is the book this book grew up reading. Nominally the documentation for one accessory, it
> is actually the platform's public reference: the editor, the assembler's grammar, the loaders'
> behavior, the utility contracts of §6.7, memory maps — and, remarkably for a consumer product of
> its era, the object-format appendix that let our vignette's teacher decode his own program at a
> kitchen table. That was not normal corporate behavior: a mass-market company handing every
> customer the keys, with a small industry of books and columns growing in its margins. Copies
> wore out in a
> characteristic way — spine cracked at the assembler chapters, corners soft at the appendices —
> and the community has kept it alive ever since <!-- OPUS-VERIFY: manual anatomy against the copy
> in this repository — exact page count, major section list, and the repo path (cite it); also
> whether the package shipped a game in object form for loader practice, as commonly recalled —
> assert what the copy shows, hedge the rest -->. A copy lives in this book's repository, and
> every page-cite reading "E/A manual" in this chapter points at it. When some later chapter
> strands you, the odds are decent the answer has been waiting in this manual since 1981.

## 6.7 Nine Names and Their Contracts: The E/A Utility Vocabulary

Open any period listing longer than a page and you will find calls to routines nobody in the
listing ever defines: a standard vocabulary of nine names the Editor/Assembler environment
supplies. Five move data past the funnel to and from the VDP's private 16K: **VSBW** (VDP single
byte write), **VMBW** (multiple byte write), **VSBR** and **VMBR** (the matching reads), and
**VWTR** (write a VDP register). **KSCAN** runs the console's keyboard scanner. The last three are
gateways between floors of the tower: **GPLLNK** borrows a routine from the GPL interpreter's
world (Ch. 25–26), **XMLLNK** calls the console ROM's machine-language services — the floating
point package above all (Ch. 23) — and **DSRLNK** hands a request to a Device Service Routine in a
peripheral's own ROM (Ch. 30). Learn them now as *names and contracts* — what goes in which
register, what comes back, what gets clobbered. We build our own equivalents in their home
chapters — vdplib (Ch. 12), inplib (Ch. 21), and the gateways where their floors are explored.

Now the trap, and it is the section's real lesson: **these routines are furniture of the E/A
environment, not organs of the machine.** They exist in memory because one particular loader put
them there. A cartridge program that calls VSBW is branching to whatever happens to occupy that
address in *its* world — probably nothing, briefly. Environment-dependence is the first question
to ask of any period listing: *who loaded this, and what furniture did it assume?* It is also why
this book builds lib99 (Ch. 11): our code will carry its own furniture, and run anywhere.

<!-- OPUS-TODO ch06-utils-table [table]:
DELIVER: the nine-name contract table — VSBW, VMBW, VSBR, VMBR, VWTR, KSCAN, GPLLNK, XMLLNK,
  DSRLNK: linkage (confirm the calling convention — expected BLWP-style; state what the manual
  actually specifies), input registers (address/data/count conventions), outputs, clobbers, and
  where each lives (loader-supplied utility vs console-ROM-backed). Final column: "ours arrives
  in —" with the chapter number (12/21/23/25–26/30), so the narrative's promises stay honest.
VERIFY: every cell page-cited to the E/A manual's utility documentation (repo copy). No live
  measurement required this chapter — these are names-and-contracts (tier 2), implemented and
  bench-verified in their home chapters. Where the manual is silent on a clobber, write "manual
  silent," never a guess.
LEDGER: one row per routine — name, contract summary, manual page. Five later chapters cite these rows.
PROSE: table is the section's spine; the environment-trap paragraph follows it unchanged. Budget ≈ 1 page.
-->

## 6.8 Mini Memory: The $99 On-Ramp

There was another 1982, and it did not own a Peripheral Expansion Box. The Editor/Assembler
package presumed the full expansion stack; for everyone else TI sold **Mini Memory** — a $99
cartridge carrying 4K of battery-backed RAM, memory that kept its contents with the console
switched off — and with it the **Line-by-Line Assembler**, reportedly delivered on cassette
<!-- OPUS-VERIFY: delivery medium and packaging of the Line-by-Line Assembler — confirm before
asserting -->. The Line-by-Line Assembler is exactly what its name confesses: you type an
instruction, and it assembles it *now*, into the module's RAM, at the address shown on the screen.
There is no source file. There is no second pass. The editor is your spiral notebook, and the
symbol table, mostly, is you.

The costs and the gift arrive together. With one pass there is no leisurely forward reference —
you plan your jumps like a chess player or patch them by hand afterward; your program's whole
world is the module's 4K; and your only listing is the one you keep in pencil. In exchange: the
machine's real instruction set on a bare console, a program that survives the power switch, and a
total price within reach of a paper route — the on-ramp thousands actually used, and the reason a
generation of TI programmers arrived at their first disk system already fluent, frugal, and
faintly suspicious of luxuries like labels. The same corner of the ecosystem had a companion
debugger as its constant sidekick <!-- OPUS-VERIFY: the companion debugger and its packaging —
commonly recalled as EASY BUG, arriving with the console-only tier; confirm name and where it
shipped before asserting -->.

For this book, Mini Memory is history rather than toolchain — our baseline includes the expansion
it heroically worked around — but its fingerprints are all over the period's style: tight code,
hand-kept address maps, and a durable cultural instinct that 4K is plenty. When a later chapter's
sources seem strangely austere, remember where their authors learned. `[console-only]` was not a
badge in 1982; it was a budget.

<!-- OPUS-TODO ch06-minimem [facts+transcript]:
DELIVER: the section's technical box — Mini Memory's geography: module RAM size and range, what is
  battery-backed, what ROM/GROM the cartridge carries, whether it supplies its own copies of any
  §6.7 utilities (a famous kindness, if true) — plus a Line-by-Line session transcript (3–4
  instructions entered live, on-screen address visible). Prefer a real session; if impossible, a
  reconstruction from the manual, labeled per R-1.
VERIFY: verify at HEAD (R-12) whether Mini Memory is among the project's 137 embedded cartridges;
  else use Classic99's licensed bundle (canon shelf role) and say which host ran it. Geography
  cross-checked against a primary source (module manual if locatable, else Classic99's bundled
  documentation); hedge anything single-sourced. Resolve this section's two OPUS-VERIFY flags
  (cassette delivery; companion debugger) and fix or hedge the narrative per R-2.
LEDGER: rows for RAM range, battery-backed size, utilities-in-module (if confirmed), and the
  Line-by-Line Assembler's key limits.
PROSE: box sits after the first paragraph; the cultural frame stays as written. Budget ≈ ¾ page.
-->

## 6.9 Building Bigger: COPY, Modules, Listings, and Maps

There are two ways for a program to be more than one file, and they embody the chapter's two
philosophies. The first is textual inclusion: **COPY** splices another source file into the stream
mid-assembly, as if you had typed it there — one assembly, one output, however many files fed it.
This is libre99asm's entire multi-file story (its COPY takes a single-quoted filename), and for the
cartridge world it is *enough*: when the output is a single settled image, nothing needs a linker,
so nothing needs modules. You know this model; it is the include model, and its costs are the ones
you already respect — everything assembles every time, and names from all files share one big room.

The second way is separate assembly, and in this ecosystem it belongs to xas99 and the E/A world:
modules assembled independently into their own object files, each publishing DEFs and confessing
REFs, stitched together by Option 3 at load time. The linking loader is linker and loader fused:
resolution happens as the files chain in, which means *load order is part of your program's
correctness*, and an unsatisfied REF is discovered standing at the console. The order below makes
you meet that error on purpose — the error is the lesson — and with it the discipline the period
learned: a module's DEFs are its public interface, six characters of API surface per name.

Either way you build, two artifacts should fall out of every build you intend to debug, and R-14
makes them standing orders. The **listing** marries your source to the addresses and bytes it
became — the first document to read whenever the machine's behavior and your intentions disagree,
because it shows what the assembler *actually did*. The **symbol map** is the name-to-address
dictionary; BENCH99 speaks addresses, and the map is where addresses come from. Treat both as
instruments, not exhaust: Ch. 5 taught you to measure the machine, and these two files are how a
measurement finds its way back to a line of your source.

> **Pitfalls.** Three traps this toolchain sets for newcomers, all cheap to demonstrate on purpose
> and expensive to meet by surprise. (1) *TEXT with no EVEN after it:* TEXT emits exactly the
> bytes of its string, strings are odd-lengthed half the time, and the next DATA or instruction
> wants a word boundary — what each assembler does about it differs. (2) *AORG overlap:* two
> absolute regions that converge on the same addresses — who complains and who silently lets the
> last writer win differs too. (3) *Symbol length:* a name beyond the classic six characters is
> accepted, truncated, or rejected depending on dialect — and truncation can quietly merge two of
> your names into one. The table records observed behavior, not folklore.

<!-- OPUS-TODO ch06-pitfalls [probe]:
DELIVER: the Pitfalls box's three-row observed-behavior table — for each trap, what libre99asm, what
  xas99, and what E/A (manual-cited) actually do; one line each, tool messages quoted verbatim
  where a tool objects.
CODE: code/ch06/pit1.a99, pit2.a99, pit3.a99 — deliberately broken probes. They are SUPPOSED to
  fail: check how verify.sh sweeps code/ (if it assembles every .a99, coordinate an exclusion — a
  subfolder or naming convention — and document the choice in a comment at the top of each file).
VERIFY: verify at HEAD (R-12) libre99asm's behavior on all three probes; xas99 via live xdt99; E/A
  from the manual, not from guesswork.
LEDGER: one row per trap per dialect where behavior differs.
PROSE: keep the box ≤12 lines; the table lives inside the blockquote.
-->

<!-- OPUS-TODO ch06-modules [listing+build]:
DELIVER: (1) the COPY architecture: a two-file libre99asm project (main + COPY'd core), R-14 build
  lines, and 8–10 quoted lines each of --listing and --symbols output read as instruments; (2) the
  object-module path [xas99]: the same core DEF'd in one module, REF'd from another, assembled
  separately, both objects chained into Option 3, shown resolving — then the REF module loaded
  ALONE, to quote the loader's unresolved-name failure (the error is the lesson).
CODE: code/ch06/modmain.a99 + modcore.a99 (COPY path); code/ch06/eamain.a99 + eacore.a99 (DEF/REF
  path — xas99-only files; same verify.sh exclusion caveat as ch06-pitfalls). Note the COPY
  quoting rule per tool if they differ (libre99asm: single quotes — canon).
VERIFY: verify at HEAD (R-12) the libre99asm leg end-to-end (builds, boots on the project emulator);
  xas99 legs against live xdt99; the two-object Option 3 chain observed actually running on
  Classic99, and the unresolved-REF error observed and quoted.
LEDGER: rows for load-order sensitivity and the unresolved-REF failure mode.
PROSE: demonstrations replace this order where it stands; keep the include-vs-link frame.
Budget ≈ 1 page.
-->

## Lab 6 — One Program, Four Ways

One behavior — a greeting written onto the screen — delivered four different ways, so that every
artifact and loader in this chapter passes through your hands once. The program is deliberately
self-contained: it touches none of §6.7's utilities and writes to the screen through the VDP ports
directly (`>8C02`/`>8C00` — Ch. 5's map), precisely so the identical core can live in all four
environments; the environment trap, dodged by construction, is half of what the lab teaches. One
core source file does the work. Two thin tops adapt it: the E/A world wants a DEF'd entry name for
the loader's table, and the modern world wants libre99asm's START convention and no DEF at all.

| Leg | Assembler | Artifact | Loaded by | What it proves |
|-----|-----------|----------|-----------|----------------|
| (a) | xas99 | tagged object (§6.4) | E/A Option 3, Classic99's bundled cartridge | you can speak object, DEF and all |
| (b) | xas99, image switch | EA5 image (§6.5) | E/A Option 5 | you can ship what the stores shipped |
| (c) | the E/A assembler itself, on the console | object, made the 1982 way | Option 3 again — after the full edit/assemble ritual | you have lived the loop once, honestly |
| (d) | libre99asm (R-14) | `.ctg` cartridge | the project emulator's menu | the same source, modern delivery |

Run leg (c) once and only once, and pay attention to your wristwatch while you do: the point is
not nostalgia but calibration. Every design habit of 1982 — long think-first sessions, listings
read at the kitchen table, changes batched before the next assembly — is downstream of how much
that loop costs, and Exercise 6.11 asks you to put a number on it.

<!-- OPUS-TODO ch06-lab [lab]:
DELIVER: shared core (message to screen via direct VDP port writes; handle the screen mode and
  charset each environment presents, documenting what you find), the two thin tops, build.sh
  driving all four legs, per-leg transcripts trimmed to the interesting lines, and one
  screen-evidence check per leg (Classic99 eyeball for a–c; BENCH99 `screen` or screenshot for d).
CODE: code/ch06/hello4.a99 (core) + hello4ti.a99 (libre99asm top) + hello4ea.a99 (xas99/E/A top, DEF
  entry) + build.sh. R-14 canonical invocation for leg (d); R-10-style xas99 invocations for legs
  (a)/(b) — flags from the ledgered verified set (-R, -i, -b, -L, -o; NO --version).
VERIFY: verify at HEAD (R-12) which legs the project emulator can host — E/A cartridge/disks
  embedded? If not, legs (a)–(c) run on Classic99 (canon shelf role); say so plainly. xas99
  invocations against live xdt99; xdt99 needs a Python host — if this workstation lacks one, run
  those legs where Python exists and record which host. Files onto a Classic99 disk: FIAD
  directory or xdm99, whichever works — document it. EVERY leg must actually display the greeting;
  a leg that didn't run may not be narrated as if it did (R-15).
LEDGER: rows for any loader-environment differences the lab surfaces (screen state on entry,
  autostart behavior, load addresses used).
PROSE: transcripts land between the table and the closing paragraph. Budget ≈ 2 pages.
-->

## Exercises

- **6.1** ✦ For each of the five lines in §6.1's fragment, name its fields and say which assemblers
  accept it unchanged — and where one doesn't, which flag (`[libre99asm]`/`[xas99]`) the fix carries.
- **6.2** ✦ Reserve 128 bytes with the label naming the address *after* the block. Which directive
  does it, and — using the §6.2 table — what value does the label hold if the block begins at
  `>A000`?
- **6.3** ✦ From memory, match the nine §6.7 names to their one-line jobs; grade yourself against
  the table. Which three are gateways between floors of the tower, and to which floors?
- **6.4** ✦ Using §6.8: name two things Mini Memory added to a bare console and two workflow
  limits of line-by-line assembly, one sentence each.
- **6.5** ✦✦ Hand-decode `code/ch06/mystery.obj` — no tools, just the §6.4 tag table — far enough
  to state its module name, its entry point, and its first two instructions.
- **6.6** ✦✦ Using the §6.4 dissection: compute where every word of the relocatable demo lands
  under a base of `>A000`, then of `>2676`. Which records change meaning with the base?
- **6.7** ✦✦ Using the §6.5 header table: write out the header words for a hypothetical program
  image 9,000 bytes long loading at `>A000`. How many files does it take, and what does each
  file's header say?
- **6.8** ✦✦ Write a module that REFs a routine SHOUT, DEF'd by a second module (§6.9's pattern).
  Predict what Option 3 does if the REFing module loads *first*; then try it and quote the result.
- **6.9** ✦✦✦ Verify one record's checksum from the §6.4 dissection by hand. Show the arithmetic,
  and state the checksum rule in one sentence.
- **6.10** ✦✦✦ In any language on your desk machine, write a small reader for the §6.4 tag subset
  that prints a memory map — addresses, words, names — of a tagged object file. Run it on
  `mystery.obj` and reconcile with your Ex. 6.5 answer.
- **6.11** ✦✦✦ Run lab leg (c) once more with a deliberate one-character error; time the full
  edit → assemble → load → run loop on a wall clock, compare against leg (d)'s, and write three
  sentences on what that ratio did to how 1982 programmers organized their days — and programs.

## Further Reading

- **The *Editor/Assembler* manual** (Texas Instruments, 1981). The primary source under this
  entire chapter. A copy lives in this book's repository <!-- OPUS-VERIFY: cite the repo copy's
  actual path here -->; the §6.6 sidebar is its biography. Read the format appendix in the original.
- ***TI Intern*** (Heiner Martin). The console ROM, disassembled and annotated — the floor beneath
  every loader in this chapter, and where §6.7's gateway routines delegate to.
- **The xdt99 suite's documentation**, xas99 above all — the modern cross-toolchain that speaks
  the period formats. The invocations in this chapter use the ledgered, session-verified flag set.
- **Classic99's documentation.** The emulator whose licensed bundle hosts this chapter's period
  legs; its notes on the bundled TI software are the quickest map of what shipped with what.
- **MICROpendium** (1984–99). Period coverage of the assembly ecosystem as readers actually lived
  it — reviews, corrections, the E/A package's long afterlife. Read as journalism; hedge accordingly.
- **The Mini Memory module's own documentation**, together with the Line-by-Line Assembler's
  accompanying material — short, scarce, and the truest picture of §6.8's workflow.
- **The project README** (repository root), for current libre99asm flags and emulator usage — cited
  rather than restated, per R-12.

## Summary

<!-- SUMMARY-DRAFT: finalize against executed orders, then append verbatim to _summaries.md. -->

- **Source grammar:** four fields; labels begin in column 1; `*` opens a full-line comment; classic
  E/A symbols are ≤6 chars, first alphabetic; fixed columns are punch-card habit, not assembler
  law; `;` guards trailing comments on no-operand mnemonics `[libre99asm]`.
- **Directive families:** where (AORG/RORG/DORG), what (DATA/BYTE/TEXT), how much (BSS/BES —
  label-at-start vs label-at-end), names (EQU/DEF/REF), hygiene (EVEN/END). Absolute vs
  relocatable thinking; libre99asm is absolute-only by design — relocation exists to serve loaders.
- **Tagged object** = ASCII records (tag character + hex payload, checksummed), read line by line
  in §6.4's dissection; tag table ledgered; compressed variant noted. **EA5 image** = header +
  verbatim slab; splitting, continuation naming, autostart per §6.5; period SAVE-harvest route vs
  xas99's image switch.
- **E/A cartridge ritual** (§6.6): two-pass assembly; Option 3 = linking loader — REF/DEF table,
  program-name prompt (CALL LINK meets the same table in Ch. 36); Option 5 = image runner.
- **Nine utility contracts** (VSBW/VMBW/VSBR/VMBR/VWTR/KSCAN/GPLLNK/XMLLNK/DSRLNK) learned as
  names-and-contracts; they are E/A furniture, not machine organs — the environment trap; ours
  arrive in Ch. 12/21/23/25–26/30, and lib99 (Ch. 11) exists so our code carries its own.
- **Mini Memory** = the $99 console-only on-ramp: 4K battery-backed cartridge RAM, Line-by-Line
  Assembler, one-pass frugality that shaped a generation's style.
- **Multi-file:** COPY inclusion (libre99asm's whole story, single-quoted filename) vs separately
  assembled modules linked at load time `[xas99]`; listings and symbol maps are instruments
  (R-14), the road from a bench measurement back to a source line.
- **Lab:** one program, four ways — Option 3 object, Option 5 image, the on-console 1982 ritual
  (once, timed), libre99asm cartridge. R-10's period workflow lives here by design; leg (d) is R-14's.
