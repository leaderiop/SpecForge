# 004 — Donald Knuth

**Cluster:** C1 — Foundations of intent capture & specification
**Roster role:** Literate programming (WEB/TeX); The Art of Computer Programming; Stanford GraphBase
**SpecForge anchors:** 'read like docs, compile like code' principle (vision/north-star.md "compiled infrastructure", vision/principles.md #4), DOT graph rendering (crates/specforge-emitter/src/dot.rs, `specforge export --format=dot`)

## Why this engineer
Knuth built the original proof of SpecForge's reading experience: literate programming (WEB, then TeX: The Program) shows an artifact can be an essay a human enjoys while being exactly machine-processable — `.spec` files that "read like documentation but are compiled like code" are that idea pointed at intent. The Stanford GraphBase treats graphs as first-class published artifacts with stable structure and public-domain sources — the model for the typed entity graph as the product, with DOT output (`emit_dot`) as its human-facing, deterministic portrait.

## References for SpecForge
**Key works**
- **Literate Programming** — *The Computer Journal* 27(2), 1984. The founding article: programs as literature addressed to humans, compiled for machines — the direct ancestor of 'read like docs, compile like code'.
- [Literate Programming (book)](https://www-cs-faculty.stanford.edu/~knuth/lp.html) — CSLI Lecture Notes 27, 1992. Expanded anthology including the 1984 article and excerpts from TeX/METAFONT; the full methodology.
- [The Stanford GraphBase: A Platform for Combinatorial Computing](https://www-cs-faculty.stanford.edu/~knuth/sgb.html) — ACM Press, 1994. 30+ graph programs as "programmatic essays" readable by humans and machines alike — graphs as shareable, versioned artifacts, exactly SpecForge's product shape.
- [ascherer/sgb](https://github.com/ascherer/sgb) — GitHub. The maintained public-domain source tree for the GraphBase (linked from Knuth's own page); a canonical graph dataset+code release to imitate.
- **TeX: The Program** — Addison-Wesley, 1986. 'Read like docs' at scale: prose-ordered sections that still compile bit-exactly.
- **The Art of Computer Programming** — Addison-Wesley, Vol. 1 (1968) through Vol. 4B (2022). Exactness discipline; the graph algorithms under specforge-graph are his syllabus.

## Study first
1. Literate Programming (1984) — the essay/compile duality
2. The Stanford GraphBase — readable, standard graph artifacts
3. TeX: The Program — the experience SpecForge spec corpora should give an agent or a new hire
