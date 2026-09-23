# Chapter 05 — Prefix caching: write the boring part once

> **In one sentence:** if every order starts with the **same long introduction**,
> vLLM keeps the chef's notes for that introduction **on the shelf** and reuses
> them for every customer — the reading is paid for once, not hundreds of times.

---

## The house-rules card

Every restaurant makes you sit through the same speech: "Welcome, here are the
hours, here is the allergy chart, here is how ordering works..." Imagine that
speech is **six pages long**, and every single customer must hear all six pages
before asking their real question.

For our chef, the "speech" is real work: she must **read** the six pages and
**write scratch notes** (Chapter 02) before she can even start your actual
question. One hundred customers = one hundred identical speeches. Ugh.

**vLLM's trick:** she reads the six pages **once**, keeps those notes **on the
shelf**, and for every new customer just points at them: "You may skip to the
good part." The first customer pays full price; everyone after pays almost
nothing for the speech.

> **Grown-up word:** the shared speech is the **prompt prefix** (for example a
> long system message), and the trick is **prefix caching** (also called
> automatic prefix caching). It works because of Chapter 03: identical notes can
> **share the same numbered slots** instead of being copied.

---

## How much is "the speech" in real work?

In SpecForge's own case (Chapter 10 explains this project), a typical work order
is about **30,000 words of reading** for the chef — and roughly **one fifth of
that** (about 6,000 words) is the *same* house-rules card every single time:
the system instructions plus a summary of the project's recipe book.

Share of the order that can skip the speech:

```
[■■ about 20% shared ■■][──────── your unique part ────────]
```

Skip one fifth of the reading and note-taking for every order in a big batch,
and the whole job finishes meaningfully sooner. And in 2025 vLLM rebuilt its
engine ("v1") so that keeping these shared notes on the shelf costs **almost
zero effort** — before, the shelf-keeping itself could get slow and awkward.

> **Check the evidence:** vLLM v1's "near-zero-overhead prefix caching" is from
> the official v1 announcement, Jan 2025 [E2]. The 30K/6K word numbers are
> SpecForge's own task model — assumption A5 in RES-31.

---

## When the trick does NOT help

Be fair: if every customer asks about something completely different, with no
shared speech, there is nothing to reuse. Prefix caching loves **repetitive
workloads** — like batch jobs that process a thousand files with the same
instructions. Lucky for SpecForge: its batch jobs are exactly that.

---

> **Check yourself**
>
> 1. What gets reused in prefix caching — the chef's answer, or her notes?
> 2. Why does Chapter 03 make this trick possible?
> 3. Name one workload where prefix caching is useless.
>
> <details><summary>Answers</summary>
>
> 1. Her **notes** (the KV cache for the shared opening text) — the answer is
>    always unique.
> 2. PagedAttention stores notes in small shared slots, so identical openings
>    can point at the same slots instead of being copied.
> 3. A workload where every request is totally different from the first word —
>    no shared prefix, nothing to reuse.
>
> </details>

---

**Previous:** [Chapter 04 — Continuous batching](04-continuous-batching.md) ·
**Next:** [Chapter 06 — Structured outputs](06-structured-outputs.md)
