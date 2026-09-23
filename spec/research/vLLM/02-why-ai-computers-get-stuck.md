# Chapter 02 — Why AI computers get stuck

> **In one sentence:** an AI brain must keep **notes about everything it has read
> so far**, the notes take up **counter space**, and the old way of saving space
> for notes wasted most of it.

---

## The chef's scratchpad problem

Remember Ada, our chef who writes one word at a time?

Here is her secret: to write the **next** word, she must remember **everything
that came before** — your whole question, and every word of her answer so far.

She would go crazy re-reading the whole conversation for every single new word.
So she keeps a **scratchpad**: after reading each word once, she writes down
"What that word means here" in super-short note form. Next word? Glance at the
notes. No re-reading.

This is a brilliant trick. It is also the reason kitchens get stuck.

> **Grown-up word:** the scratchpad is called the **KV cache** ("Key-Value cache").
> "Cache" just means "notes I keep nearby so I don't have to re-do work."

---

## The notes eat the counter

Here is the painful part. The scratchpad grows with **every single word**:

- A short question → a thin notepad.
- A long story, or a hundred-page document, or "remember all 2,000 recipes" →
  a notepad the size of a mattress.

And the counter (GPU memory) is **big but not infinite**. When twenty customers
each need a mattress-sized notepad, the counter is full. Customer twenty-one
waits outside.

---

## The old way of saving space: the fixed-size locker

The old kitchen managers said: "Let's give every order a locker for its notes.
How big should lockers be? Well, some orders are huge... so let's make **every
locker mattress-sized**, just in case."

Sounds safe. It is a disaster:

```
Order 1: notes fill   5% of its giant locker   → 95% WASTED
Order 2: notes fill  10% of its giant locker   → 90% WASTED
Order 3: notes fill   3% of its giant locker   → 97% WASTED
...
Counter full. Chef mostly idle. Everyone waiting.
```

Studies found that in the old systems, often **60–80% of the note space was
wasted air**. The counter looked full but was mostly empty lockers.

> **Grown-up word:** this waste has two names: **internal fragmentation**
> (locker too big for its notes) and **reservation waste** (space reserved for
> a *possible* long answer that never gets long). The old systems pre-allocated
> a fixed maximum length per request.

---

## The silly traffic jam this creates

Because the counter filled up with wasted air:

- new customers were queued **even though the chef was not busy**,
- people with **short questions** waited behind phantom "full" lockers,
- the only fix was "buy more counters" — and counters (GPUs) cost as much as a car.

This is the stuck-ness that Chapter 03's parking-lot trick blows away.

---

> **Check yourself**
>
> 1. Why doesn't the chef just re-read your question for every new word?
> 2. What eats the GPU's counter space?
> 3. Why were the old lockers mattress-sized?
>
> <details><summary>Answers</summary>
>
> 1. She could, but it is thousands of times slower — the scratchpad means each
>    word costs one small glance instead of a full re-read.
> 2. The KV cache — the notes that grow with every word of every conversation.
> 3. Because managers reserved space for the *longest possible* conversation,
>    for every order, just in case.
>
> </details>

---

**Previous:** [Chapter 01 — What is vLLM?](01-what-is-vllm.md) ·
**Next:** [Chapter 03 — PagedAttention, the parking-lot trick](03-paged-attention.md)
