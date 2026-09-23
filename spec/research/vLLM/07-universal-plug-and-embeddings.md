# Chapter 07 — The universal plug (and treasure maps for meanings)

> **In one sentence:** vLLM speaks the **same request language as almost every
> AI tool on Earth** (the "OpenAI-compatible API"), so you can swap kitchens
> without rewriting anything — and it can also turn sentences into
> **treasure-map coordinates** that show which meanings are neighbors.

---

## Part 1 — the plug that fits every socket

Imagine if every country's wall sockets needed a **different question format**
to ask for electricity. Nightmare. Now imagine one plug shape used everywhere.

When programs talk to an AI kitchen, they send a small structured letter:
"here is my question, here are my settings, please send back the answer."
In 2023, one company's letter format became so popular that **everyone** adopted
it as the standard — not because of the company, but because a shared letter
format lets tools and kitchens mix freely.

vLLM reads and writes that exact letter format. So does SGLang, Ollama, LM
Studio, and others. What that buys you:

- Write your program **once** → it works with any of these kitchens.
- Swap kitchens later = change **one line** (the address), not the program.
- No kitchen can kidnap you by changing the letter format.

> **Grown-up word:** the letter format is the **OpenAI-compatible REST API** —
> endpoints like `/v1/chat/completions` for chat and `/v1/embeddings` for the
> treasure maps below. Evidence [E3], [E16]. SpecForge's port design (Chapter 10)
> leans on exactly this: the port stays standard, the kitchen is swappable.

---

## Part 2 — treasure maps for meanings

Beside writing answers, kitchens can do something quieter but huge: take a
sentence and spit out a **point on a map**.

Here is the game: pick a number like **1,024**. Every sentence becomes a point
with 1,024 coordinates. Sentences with **similar meaning** land **near each
other** on the map:

```
"how do I log in?"          •        ← these two are neighbors
"can't access my account"   •

"recipe for pancakes"            •   ← far away from login questions
```

Now "search" becomes "who stands near my point?" — that is how computers find
things **by meaning** instead of by exact keywords. This powers recommendation
lists, duplicate detectors, and (in SpecForge's case) letting an AI agent ask
"which recipes talk about *payments*?" and get the right ones even if the word
"payments" never appears.

> **Grown-up word:** the point is called an **embedding**, the map is a **vector
> space**, and "who is nearby" is measured by **cosine similarity**. vLLM can
> serve embedding models through the same universal plug (`/v1/embeddings`) and
> even score pairs directly. Evidence [E15].

---

## Both, one kitchen

A neat detail: **one vLLM kitchen can do both jobs** — the chatty chef
(generating answers) and the mapmaker (embeddings) — on the same rented counter.
For a small team that means one machine, one bill, two abilities.

---

> **Check yourself**
>
> 1. Why did everyone adopt the same letter format for AI kitchens?
> 2. Your program talks to vLLM today. Tomorrow you want SGLang. How much rewriting?
> 3. On the meaning-map, what does it mean when two sentences' points are close?
>
> <details><summary>Answers</summary>
>
> 1. Because a shared format lets any tool work with any kitchen — network
>    effects made the common format the standard.
> 2. Change the kitchen's address (one line). The letters are identical.
> 3. Their meanings are similar — that is semantic search.
>
> </details>

---

**Previous:** [Chapter 06 — Structured outputs](06-structured-outputs.md) ·
**Next:** [Chapter 08 — Speed and money](08-speed-and-money.md)
