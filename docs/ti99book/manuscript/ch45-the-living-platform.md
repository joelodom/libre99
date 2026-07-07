# Chapter 45 — The Living Platform

<!-- Part X — Beyond the Console · target ≈10 pp -->
<!-- STATUS: DRAFTED (session 10, 2026-07-07) — pending review passes. The book's closing chapter and final word. Narrative/survey; community specifics hedged per R-2 (verified anchors: AtariAge TI-99 forums, 99er.net, Ninerpedia, the WHTech archive); event names hedged. The clean-room console GROM (original-content/system-roms/) is the project's own ethics showpiece — canon, no verification needed. The portfolio checklist is cross-checked against the actual Part IX deliverables and lib99 modules. No code artifact. Applies R-2/R-12; continues Ch. 1's ethics stance. This chapter closes the manuscript body (Ch. 1-45); front matter + appendices remain. -->
<!-- SPEC: 00-master-outline.md, "### Chapter 45 —" (lines 687–693). -->

## A New Game, an Old Machine

Somewhere as you read this, a cartridge is being flashed for a console older than
most of the people who will play it. A programmer — maybe in a bedroom, maybe at a
kitchen table, maybe you, a week from finishing this book — has just watched a
green border come up on a machine designed in 1979, and is about to hand the
result to strangers on the far side of the world who will run it on *real
hardware*, plug it in, hear it beep, and grin. There will be a manual, because
this community still writes manuals. There will be, perhaps, a box. And there will
be, at some point, a small annual gathering where people carry these machines into
a hotel ballroom and show each other what they made this year.

This is the fact the whole book has been quietly building toward: the TI-99/4A is
not a dead machine. It is a *living platform* — smaller than it was, yes, and
volunteer-run, and commercially finished for forty years — but genuinely, stubbornly
alive, with new software, new hardware, new documentation, and new people arriving
every year. You have spent forty-four chapters learning to program it. This last
chapter is about *joining* it: where the community is, how to release your work
into it, how to help preserve what came before, and — the book's final question —
why any of it is worth your one and only attention.

---

## What You Can Do Now

This chapter has no new technique. It has an invitation. After it you can:

- **Find the community** — its forums, archives, and gatherings — and know where
  finished software actually lands.
- **Publish your own work** honorably: on cartridge, on disk, as a digital
  download, with a manual and a license you chose on purpose.
- **Preserve** the record: dump, document, and contribute to the emulators and
  archives that keep the platform legible for the next arrival.
- **Give back** — a tool patch, a hardware note, an hour spent answering a
  beginner — and understand that the platform is alive *because* people do.
- **Answer, for yourself, why you would program a machine the world has left
  behind** — which is really a question about what you want from a computer.

## The Bridge: You Are the Platform Now

Every other bridge in this book connected a modern idea to a vintage one. This one
runs the other way. A living retro platform is not a museum that other people
maintain for you to visit; it is a thing that exists *only* to the extent that
people keep doing it — writing the software, dumping the ROMs, answering the
questions, showing up to the faire. The moment you shipped your first program you
stopped being an audience and became a member, which means the platform's future is
now, in some small measurable part, *your* responsibility and your gift. The modern
world calls this open source and community stewardship and treats it as a movement.
This community was doing it before the words were fashionable, out of love, for a
machine no company would ever thank them for. You are one of them now.

## 45.1 The Community Atlas

The map, as of this writing, is small and warm and easy to learn. The center of
gravity is online, in the community's forums — the TI-99/4A sections of the larger
retro-computing boards are where new work is announced, debugged in public, and
celebrated. Alongside them sit the reference wikis and the great file archives that
hold decades of software, documentation, and scanned manuals; treat these as the
platform's library, and the shelf tools of R-12 (Classic99, js99er, MAME, the
xdt99 suite) as its workbench. And a few times a year, in physical rooms, the
community *gathers* — regional fests and a long-running annual faire tradition
where hardware is demonstrated, new releases are shown, and the people behind the
usernames shake hands. (Names, URLs, and dates drift; **Appendix N** carries the
current, verified list, and you should trust it over any specific I might name here
that has since moved.)

What matters more than the exact addresses is the *shape*: a place to talk, a place
to archive, a place to gather, and a small economy of volunteer vendors who will
still, in the twenty-first century, manufacture you a cartridge. Learn that shape
and you can find your way in any living retro community, because they all have it.

## 45.2 Publishing Your Work in 2026

Your capstone is done and green. How does it reach a player? You have, remarkably,
the full range of choices the 1983 developer had, plus new ones:

- **On cartridge.** Community vendors produce real, boxed cartridges — flashable
  boards in shells — so your game can go into a slot and *click* like a first-party
  title. This is the medium Part IX built for (Ch. 35's bank-safe design pays off
  here).
- **On disk, or as a disk image.** A DV80 or program-image file (Chs. 31–32, 42)
  distributes instantly as a `.dsk` anyone can mount in an emulator or write to
  real media.
- **As a digital download.** The modern default: post the image, the manual, and
  the source, and it is on every continent by morning.

Two things separate a *release* from a pile of bytes, and both are craft. The
first is the **manual** — the 1982-style artifact Ch. 39 had you write, which is a
design review in disguise and, more than that, a courtesy to the player. The second
is the **license**, and here the book asks you to choose on purpose. This very
project ships under a source-available license chosen deliberately — the Modified
MIT License with a Commons Clause — so that its source can be read, learned from,
and built upon, while a particular commercial right is withheld. You may choose
differently. But choose: put a `LICENSE` in the archive, a header on the source,
and a clear word to the next programmer about what they may do with your gift.
Un-licensed code is not generous; it is merely ambiguous.

## 45.3 Preservation and Stewardship

The platform is alive only because someone, for forty years, refused to let it die
quietly — and that refusal is *technical work*, the same skills this book taught,
pointed at the past instead of the future. To **dump** a ROM or a rare disk
faithfully, to **document** an undocumented behavior so the next emulator can model
it, to file the bug that makes Classic99 or js99er a hair more accurate — these are
acts of preservation, and they are how the record stays legible.

And preservation has an ethics, which this book has practiced on every page, not
just preached. Consider this project's own showpiece. The Libre99 emulator ships
**no** Texas Instruments firmware and **no** commercial cartridge — its
`cartridges/` directory is empty by design, and it stays that way. In place of the
console's copyrighted GROM, the project wrote a **clean-room replacement from
scratch**: an original console firmware that boots, shows a menu, and runs
programs, sharing not one proprietary byte with the original. That is the ethical
high-water mark of the whole endeavor — the proof that you can understand,
recreate, and celebrate a machine *without* appropriating what was never yours to
take. It is why Part IX reconstructs its genres behaviorally and never from a
copied image; it is why this book teaches you to build the thing rather than to
pirate it. Preserve generously, attribute scrupulously, and take nothing that
belongs to someone else. The community's long life is built on exactly that
discipline.

## 45.4 Contributing Back

You do not have to write a landmark game to matter here. The platform runs on
small gifts, and the most valuable are often the least glamorous:

- **A tool patch** — a fix to an assembler, an emulator accuracy improvement, a
  build-script convenience — helps everyone who comes after, silently, forever.
- **A hardware note** — a measured timing, a pinout, a corrected datasheet
  ambiguity — is the raw material every future emulator and programmer depends on
  (this book's own ledger is nothing but such notes, gathered).
- **An hour with a beginner** — an answered forum question, a commented example, a
  patient explanation of the byte-high law for the hundredth time — is how the
  platform acquires its next member, which is the only way it survives another year.

Mentoring is not charity; it is propagation. Every programmer reading this exists
because someone, somewhere, took the time. Pay it the same way.

## 45.5 Why Program a Dead Machine?

So we arrive at the question this book has owed you since its first page. With
every modern language, framework, and machine available — with computers a
*billion* times faster sitting in your pocket — why spend your finite attention on
a 3 MHz console the market euthanized before you were born?

Three answers, and they are the whole book distilled.

The first is **constraint**. A modern machine is effectively infinite, and infinity
is a poor teacher, because it never says *no*. The 4A says no constantly — no more
than 256 bytes of fast RAM, no more than four sprites on a line, no more than fifty
thousand cycles a frame — and every no forces a decision, and every decision
teaches. You learned the memory hierarchy because a wait state made you feel it. You
learned what a data structure *costs* because a gap buffer had to fit in a
scratchpad. Scarcity, we said in Chapter 40, is not a cage; it is an editor. It
edited you into a better programmer, and it could not have done so if the machine
had let you off easy.

The second is **comprehension**. This is the rarest thing modern computing offers
and the thing this machine offers freely: you can understand *all of it*. Every
byte of the memory map, every line of the boot sequence, every cycle of the
instruction that just executed — the whole machine fits in one human head, and by
now it fits in yours. You have held a complete computer in your mind, from the
silicon of the TMS9900 to the software of a shipped game, with nothing hidden, no
layer you had to take on faith. Almost no working programmer today has ever had
that experience with the machine they use. You have. It changes how you see every
larger machine after it, because you now *know*, in your hands and not just in
theory, that the towering abstractions are only towers — that underneath every one
of them is a chip moving bytes, comprehensible if someone bothers to look.

The third is simply **pleasure** — the particular, quiet joy of holding a whole
thing in your head and making it do exactly what you meant. It is the pleasure of
craft at a scale a person can encompass, and it does not get less real for the
machine being old. If anything it gets *more* real, because nothing here is trying
to sell you something, or harvest your attention, or update itself overnight into a
stranger. It is just you, and a machine you understand completely, and an idea you
want to make true. That is as good as programming gets, and this old console offers
it more purely than almost anything built since.

That is why. Not nostalgia — nostalgia is for people who were there, and most of us
weren't. It is because a small, complete, unforgiving machine is the best teacher of
the real thing computing is, underneath all the abstraction, and because
understanding a whole computer is a joy you owe yourself at least once. The 4A will
give you that joy for the price of your attention. It is a bargain.

## The Final Artifact: Your Portfolio

Close the book and look at what you have. It is not a stack of finished chapters; it
is a working body of software, all of it yours:

- **Five complete games**, one per architecture: METEOR BELT the cartridge shooter,
  GRIDRUNNER 99 the console-only arcade, DUNGEONS OF FATE the data-driven RPG
  engine, AUTHOR99 the productivity tool, and DRIFT the modern port — each machine-
  verified, each shippable.
- **A library** — `lib99` — grown one chapter at a time: math and memory, the VDP
  and text and bitmap and sprites, sound and speech, input, GROM, the disk and file
  and sector layers, compression. The reusable craft of a working programmer, in
  one tree.
- **A toolchain and a chassis**: fluency in `libre99asm` and `libre99gpl`, the
  BENCH99 instrument for *proving* code rather than hoping, and SKELETON99, the
  chassis every capstone stood on.

That portfolio is the real deliverable of this book. The chapters were only the way
to build it.

## Lab 45 — Ship Something, and Sign It

One last lab, and it is the only one that leaves the page:

1. **Release a program.** Take any capstone or exercise you built, write it a real
   one-page manual, choose it a license and put the file in the archive, and post
   it — a disk image, a cartridge build, a download — somewhere a stranger can find
   it. It does not have to be big. It has to be *shipped*.
2. **Give one thing back.** File one bug, answer one beginner's question, or send
   one measured hardware note to the record. Join the platform by adding to it.

## Exercises

**✦ Reflection**

1. In your own words, and in three sentences, answer the question §45.5 asked: why
   program this machine? Your answer, not the book's.
2. Pick the license you would ship your own 4A work under, and defend the choice in
   a paragraph a non-lawyer could follow.

**✦✦ Practice**

3. Write the one-page manual for one of your Part IX capstones as if it shipped in
   1983 — controls, objective, scoring, and the single screenshot you would print.
4. Take one behavior this book measured (any ledger row) and write the note you
   would send an emulator author to help them model it accurately.

**✦✦✦ The open road**

5. There is no exercise 5. There is the platform, and the things it does not have
   yet, and now you can build them. Go.

## Further Reading

- **Appendix N**, the annotated bibliography and resource guide, holds the current,
  verified community map — forums, archives, gatherings, and vendors — and should be
  trusted over any specific in this chapter that has since moved.
- **Chapter 1** made the promise this chapter keeps; read its opening and this
  closing essay together to see the arc whole.
- The project's own `original-content/system-roms/` — the clean-room console
  firmware — is the ethics of §45.3 made real and readable.

## Summary

The TI-99/4A is a living platform, and finishing this book makes you a member of it
rather than a student of it. The community is small, warm, and easy to find — forums
to talk in, archives to learn from, gatherings to meet at, and volunteer vendors who
will still build you a cartridge; **Appendix N** carries the current map. You can
publish your work with the full range of media the platform allows — cartridge,
disk, download — and the craft that separates a release from a pile of bytes is a
real manual and a chosen license. You can preserve the record by dumping,
documenting, and improving the emulators, and you can do it *ethically*, the way
this project did when it shipped a clean-room console firmware and not one
proprietary byte. And you can contribute back in gifts large and small, the smallest
of which — an hour with a beginner — is how the platform survives another year.

Why any of it? Because a small, complete, unforgiving machine teaches constraint,
offers total comprehension, and gives the pure pleasure of holding a whole computer
in your head — three things almost no modern machine will ever give you, and three
things this old console gives freely. You came to these pages able to program,
perhaps, in the abstract. You leave them having held an entire computer in your
mind, from its silicon to a shipped game, and having built five of those games, a
library, and the fluency to build more. That was the promise of Chapter 1, and it
is kept. Now there is a machine, forty-odd years old, waiting to run whatever you
imagine next — and, if you ask it nicely and wire up the synthesizer, to say your
words out loud. Go and make it speak.
