# Chapter 01 — What is vLLM?

> **In one sentence:** vLLM is a free program whose whole job is to let **one**
> AI brain answer **very many** people's questions **at the same time**, quickly
> and without wasting memory.

---

## First, three words you need

**1. An AI brain (LLM).**
Imagine a friend named Ada who has read almost everything ever written. Ask her
a question and she answers. But here is the strange part: **she writes her answer
one word at a time.** Word... word... word... like a very careful typist.

> **Grown-up word:** an AI brain is called an **LLM** (Large Language Model).
> Examples: Llama, Qwen, Mistral, DeepSeek. Each word-piece it writes is called
> a **token**. A token is sometimes a whole word, sometimes just a chunk like "ing".

**2. A GPU (the fast counter).**
Answering takes millions of tiny multiplications. A normal computer chip does a
few at a time. A **GPU** does thousands at once — like swapping a single rolling
pin for a hundred rolling pins working together.

**3. Serving (the restaurant).**
One Ada talking to one friend is easy. But what if **two hundred people** want Ada
at once? You need a **kitchen manager**: something that decides which order gets
the counter, when food goes out, and how to not waste space.

**That kitchen manager is what vLLM is.**

---

## The problem vLLM was born to solve

Before vLLM, kitchens were wasteful in two big ways:

1. **They reserved counter space clumsily.** Every order got a giant fixed shelf,
   even if the order needed a tiny corner. Shelf space ran out → new customers
   were turned away even though the chef was twiddling her thumbs.
2. **They made the chef wait.** "We start cooking when the bus is full" — so the
   chef sat idle while two people waited.

vLLM fixed the first problem with a parking-lot trick (Chapter 03) and the second
with a never-idle-chef trick (Chapter 04). Together they changed the whole industry.

> **Check the evidence:** the vLLM team measured up to **24× more answers per hour**
> than the naive method, and **3.5× more** than the previous best-known kitchen
> (HuggingFace TGI). See RES-31 Exhibit 02, evidence [E1], [E18].

---

## What vLLM is NOT

- **Not a brain.** vLLM does not come with an AI brain. You bring the brain
  (a downloaded model file); vLLM runs it well.
- **Not magic speed dust.** It makes one GPU serve many people efficiently.
  A single person alone at home may not notice a difference (Chapter 09).
- **Not a SpecForge feature.** SpecForge is a recipe-checker, not a kitchen.
  vLLM is a kitchen other people can rent or run. Chapter 10 explains the rules.

---

## Three facts to remember

1. **vLLM = a kitchen manager** for AI brains. It does not cook; it organizes cooking.
2. **Its two superpowers** are smart shelf space (PagedAttention) and a
   never-idle chef (continuous batching).
3. **It speaks a universal language** that all popular AI tools already
   understand (Chapter 07). That is why it fits anywhere.

---

> **Check yourself**
>
> 1. Does vLLM include an AI brain?
> 2. Why does Ada write one word at a time instead of a whole paragraph at once?
> 3. What are the two wasteful things vLLM fixed?
>
> <details><summary>Answers</summary>
>
> 1. No — you bring the brain (a model file); vLLM is the manager that runs it.
> 2. Each next word depends on all the words before it, so the answer grows one
>    token at a time. (Fast, but one at a time — like dominoes.)
> 3. Wasted shelf space for notes (Chapter 03) and a chef who waited for full
>    batches (Chapter 04).
>
> </details>

---

**Next:** [Chapter 02 — Why AI computers get stuck](02-why-ai-computers-get-stuck.md)
