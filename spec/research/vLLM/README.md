# vLLM Explained — A Tutorial for Curious Kids (and Curious Grown-Ups)

This is a friendly, step-by-step explanation of **vLLM**, one of the most famous
"engines" for running AI brains, and how it connects to **SpecForge**, this project.

No scary math. No giant words without a translation. Every big idea gets a story
you can picture in your head.

> **Prefer pictures?** There is a fully illustrated, single-page HTML edition of
> this tutorial with a diagram for every trick:
> **[vllm-explained.html](vllm-explained.html)**.

---

## The story we will use everywhere

Imagine a restaurant called **The Word Kitchen**.

- The **chef** is an AI brain (grown-ups call it an **LLM** — a Large Language Model).
  She answers any question, but she writes her answer **one word at a time**.
- The **kitchen counter** is a **GPU** — a special computer chip that is very fast
  at the kind of math a brain like this needs.
- Every order needs **counter space** for the chef's scratch notes. Counter space
  is the GPU's **memory**, and it runs out fast.
- **vLLM** is a set of clever kitchen tricks that lets **one chef serve a whole
  restaurant of hungry customers at once** without dropping plates.

That's it. The whole tutorial is just these tricks, one chapter at a time.

---

## The chapters

| # | File | What you will learn |
|---|------|---------------------|
| 01 | [01-what-is-vllm.md](01-what-is-vllm.md) | What vLLM is and why it exists |
| 02 | [02-why-ai-computers-get-stuck.md](02-why-ai-computers-get-stuck.md) | Why running AI brains is slow and cramped |
| 03 | [03-paged-attention.md](03-paged-attention.md) | The parking-lot trick that fits 24× more cars |
| 04 | [04-continuous-batching.md](04-continuous-batching.md) | Why the chef never stands around waiting |
| 05 | [05-prefix-caching.md](05-prefix-caching.md) | Write the boring part once, use it forever |
| 06 | [06-structured-outputs.md](06-structured-outputs.md) | Forms the robot cannot fill in wrong |
| 07 | [07-universal-plug-and-embeddings.md](07-universal-plug-and-embeddings.md) | The plug that fits every kitchen |
| 08 | [08-speed-and-money.md](08-speed-and-money.md) | How we measure "fast" and "cheap" |
| 09 | [09-the-great-kitchen-race.md](09-the-great-kitchen-race.md) | vLLM against its rivals |
| 10 | [10-vllm-and-specforge.md](10-vllm-and-specforge.md) | What all this means for SpecForge |

---

## How to read the boxes

Each chapter has three kinds of boxes:

> **Grown-up word:** the real technical name for the thing in the story, so you
> can read other articles without getting lost.

> **Check the evidence:** where the number or claim comes from. All evidence lives
> in the big kid-brother of this tutorial:
> [../RES-31-vllm-inference-evaluation.html](../RES-31-vllm-inference-evaluation.html) —
> our engineering dossier with charts, benchmarks, and prices.

> **Check yourself:** a tiny quiz at the end of each chapter. Guess first, then peek.

---

## Two promises this tutorial makes

1. **Nothing is dumbed down into being wrong.** Every story maps exactly to a real
   mechanism. When a number is rounded, the precise number is in the evidence box.
2. **Honesty about the bad news too.** vLLM is not the fastest engine at everything,
   not the cheapest at everything, and definitely not something SpecForge should
   bundle into its own toolbox. Chapter 09 and 10 explain why.

---

*Part of SpecForge research. Parent document: [RES-31 — vLLM Inference Evaluation](../RES-31-vllm-inference-evaluation.html).*
