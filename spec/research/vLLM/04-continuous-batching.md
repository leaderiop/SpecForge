# Chapter 04 — Continuous batching: the chef never waits

> **In one sentence:** old kitchens made the chef wait until a whole busload of
> orders finished together; vLLM slides the **next order in the moment one plate
> leaves the counter** — so the chef is always cooking.

---

## The bus that ruins everything

Imagine a school bus with 30 seats. Old kitchen rules:

1. Wait until **30 orders** are stacked up.
2. Cook all 30 **as one group**.
3. Wait until the **whole group** is done.
4. Only then let the next group in.

The disaster is step 3. Orders are different sizes! One kid asked "what is 2+2?"
(one word of answer). Another asked for a full essay (a thousand words). The bus
waits for the essayist while everyone else already got their food. The chef —
the most expensive thing in the building — sits idle on their behalf.

> **Grown-up word:** this is **static batching**. The whole group starts and
> finishes together, so the fastest orders waste the chef's time.

---

## The sliding-tray fix

vLLM plays by different rules. Watch the counter at any instant:

```
tick 1:  [Ada][Ada][Ada][Ada]        4 orders cooking
tick 2:  [Ada][DONE][Ada][Ada]  →   one plate leaves
tick 3:  [Ada][NEW!][Ada][Ada]  →   next order slips into the empty seat
tick 4:  [DONE][Ada][Ada][NEW!] →   and again, forever
```

Every time **any single plate** is handed out, a **new order takes that exact
seat**, immediately. There is no group. There is no bus. There is just a
continuously full counter.

> **Grown-up word:** **continuous batching**, also called **iteration-level
> scheduling**: the kitchen re-decides who is cooking at **every single word-step**,
> not once per group. It is the single biggest reason vLLM's counter stays busy.

---

## Why this matters more the bigger the party

With **one** customer, no batching matters at all — you get the whole chef.

The trick shows its power with a **crowd**. In a fair, careful university study
(2026), two popular home kitchens were loaded with more and more people at once:

- **Ollama** (a lovely simple home kitchen) averaged about **21 seconds** of
  waiting with just **10 people**, and got worse as the crowd grew. (By default
  it even cooks only **one order at a time**.)
- **vLLM** kept waiting **under 10 seconds** with **100 people at once** — and
  served **every single order successfully**. Not one dropped plate.

Ten times the crowd, less than half the wait. That is what a never-idle chef
means.

> **Check the evidence:** the study is "Benchmarking Ollama and vLLM for
> Concurrent LLM Serving" (MDPI, 2026) [E8]; Ollama's default of one parallel
> order is in its own FAQ [E9]. Full story in RES-31 Exhibit 03.

---

## The fair fine print

If it is just **you**, at home, asking one question at a time, continuous
batching does almost nothing for you — and simple home kitchens are wonderful
for that case (one even starts answering in about 45 thousandths of a second!).
The tricks in this chapter exist for **crowds**: teams, apps, batch jobs.
Chapter 09 compares the kitchens fairly.

> **Check the evidence:** Ollama's fast single-user answer time is from Ginger
> Labs' 2026 comparison [E10].

---

> **Check yourself**
>
> 1. What is wrong with "wait for a full bus" batching?
> 2. When does vLLM let a new order start cooking?
> 3. You alone, one question at a time, at home. Which kitchen tricks help you?
>
> <details><summary>Answers</summary>
>
> 1. Everyone in the group waits for the slowest order; fast orders waste the chef.
> 2. The instant any single order finishes — every word-step, the seat is refilled.
> 3. Mostly none of this one. For a single user you want a fast simple kitchen
>    (Chapter 09 explains which).
>
> </details>

---

**Previous:** [Chapter 03 — PagedAttention](03-paged-attention.md) ·
**Next:** [Chapter 05 — Prefix caching](05-prefix-caching.md)
