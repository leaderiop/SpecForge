# Chapter 03 — PagedAttention: the parking-lot trick

> **In one sentence:** instead of giving every order one giant fixed locker for
> its notes, vLLM cuts note space into **tiny numbered slots** and hands them out
> as needed — like a parking lot with small marked spaces instead of spaces
> sized for trucks.

---

## Two parking lots

**The old lot:** every space is painted for a truck, because *someone* might
arrive with a truck. A kid's bicycle shows up? It still takes a whole truck
space. The lot "fills up" while being mostly air.

**The vLLM lot:** spaces are small and numbered. A bicycle takes one bicycle
space. A truck takes as many small spaces as it needs — and they do **not**
have to be next to each other! The truck parks in slots 7, 8, 22 and 23 while
bicycles squeeze in everywhere else.

```
OLD:  [T][T][T][T][T][T][T][T]   ← bike in a truck space = waste

vLLM: [b][T7][b][b][T8][b][T22][T23][b][b]
       ↑ tiny waste, lot fits MANY more vehicles
```

> **Grown-up word:** the trick is called **PagedAttention**. The small numbered
> slots are **KV blocks** (each holds the notes for a fixed few tokens). A
> conversation's notes are a **list of block numbers** — exactly like a computer's
> operating system manages memory with "pages". That is the joke in the name:
> *paging* + *attention*.

---

## What this buys you

1. **Almost no wasted space.** Leftover air shrinks from "most of the lot" to
   "less than one small slot per order".
2. **More customers at once.** Same counter, many more simultaneous conversations.
3. **Sharing for free.** Two orders starting with the *same* words (like the same
   house-rules card) can **point at the same slots** for the shared part.
   Nobody copies anything. (This becomes a superpower in Chapter 05.)

---

## How much more?

The vLLM team measured it when they introduced the trick:

- **~24× more answers per hour** than the naive locker system
  (HuggingFace Transformers doing serving the simple way).
- **~3.5× more** than the previous best professional kitchen manager of the time
  (HuggingFace TGI).

And years later, university researchers re-tested it under heavy crowds and saw
the same order of advantage. This was not a lucky demo; it is how the trick works.

> **Check the evidence:** numbers from the vLLM project's own announcement [E1],
> re-confirmed by an independent arXiv performance study in Nov 2025 [E18].
> Full story in RES-31 Exhibit 02.

---

## One honest footnote

PagedAttention fixes **space**. It does not, by itself, fix a chef who stands
idle waiting for a "full bus" of orders. That is a separate trick, and it is
Chapter 04.

---

> **Check yourself**
>
> 1. In the parking lot, what do the small numbered slots stand for?
> 2. Can two different orders share slots? When?
> 3. PagedAttention fixes space. What does it NOT fix?
>
> <details><summary>Answers</summary>
>
> 1. KV blocks — small chunks of the chef's notes (the KV cache).
> 2. Yes — if their prompts begin with identical text, the shared part points at
>     the same blocks. Copy-free sharing.
> 3. Idle-chef waiting between orders. That needs continuous batching — Chapter 04.
>
> </details>

---

**Previous:** [Chapter 02 — Why AI computers get stuck](02-why-ai-computers-get-stuck.md) ·
**Next:** [Chapter 04 — Continuous batching](04-continuous-batching.md)
