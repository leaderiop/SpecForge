# Chapter 09 — The great kitchen race

> **In one sentence:** vLLM is **not** the champion of everything — it is the
> most **balanced all-rounder**: near the front in almost every race, winner in
> the widest variety of kitchens, on the widest variety of counters.

---

## Meet the racers

| Racer | Kid description | Superpower | Weak spot |
|---|---|---|---|
| **vLLM** | The balanced pro kitchen | PagedAttention + never-idle chef + universal plug + mold-writing, runs on almost any counter | Setup needs a real counter (GPU) and some care |
| **Ollama** | The easy home kitchen | One-click install, works on your laptop, lovely for one person | Crowds make it wobble: ~21 s waits at just 10 people; by default cooks 1 order at a time |
| **llama.cpp** | The pocket mechanic | Runs on anything, even no-GPU laptops; invented the mold format (GBNF) | You assemble more yourself; less built for big crowds |
| **SGLang** | The speed demon | Beat vLLM by ~30% in a 2026 speed race (≈16,200 vs ≈12,500 words/sec) | Smaller menu of models and counters; fewer extras |
| **TensorRT-LLM** | The tuned race car | NVIDIA-only, hand-tuned for maximum lap times | NVIDIA-only, fiddly; vLLM actually beat it by ~11% on the newest counter for agent-like work |
| **TGI** | The veteran | Solid, was the pro standard | The founding race: vLLM was ~3.5× faster |
| **Managed API** | Someone else's restaurant | Zero dishes to wash, always open, elastic | Your recipes (source code) leave your house; no custom molds; per-burger fees forever |

---

## The scoreboard (the honest one)

SpecForge weighted seven race categories by what *this project* actually needs
(speed in crowds, molds, the universal plug, embeddings, easy setup, hardware
range, care effort, model range). Result:

```
Managed API   4.30   ← wins the default path (no dishes!)
vLLM          4.20   ← best self-hosted all-rounder
llama.cpp     3.85   ← the stealth pick (molds + runs anywhere)
SGLang        3.55   ┐ speed demons, narrower menus
Ollama        3.55   ┘ home heroes
TGI           3.10
TensorRT-LLM  2.75   ← fastest on paper, narrowest life
```

> **Check the evidence:** full scorecard with per-cell reasoning is RES-31
> Section 08 [E19]; race numbers: SGLang vs vLLM [E4], decode gaps [E5],
> vLLM vs TensorRT-LLM on B200 [E6], batch jobs [E7], Ollama under load [E8],
> [E9], single-user wins [E10].

---

## So who wins WHICH race?

- **Just me, on my laptop, tonight:** Ollama or llama.cpp. vLLM is overkill —
  its superpowers are crowd superpowers.
- **A team serving an app, or a batch job over a codebase:** vLLM — the widest,
  steadiest choice, with molds and the universal plug built in.
- **Maximum raw speed, one big NVIDIA counter, and you love tuning:** SGLang
  (or TensorRT-LLM). The plug is similar; swapping later is cheap if you keep
  your letters standard.
- **Zero servers to care for, and your data may leave the house:** managed API.
  The scoreboard's winner, honestly.
- **Secrets (private code):** self-host — and then vLLM is the default pick.

The deep lesson: **"best" depends on which race you are running.** That is why
the scorecard exists instead of a trophy.

---

> **Check yourself**
>
> 1. Which racer is fastest in the 2026 speed race?
> 2. Which racer would you pick for one curious kid on one laptop?
> 3. Why does managed API top the scoreboard despite the privacy downside?
>
> <details><summary>Answers</summary>
>
> 1. SGLang, by about 30% in that specific race (16,200 vs 12,500 words/sec).
> 2. Ollama (or llama.cpp) — simple install, great for a single user, no crowd,
>    no setup ceremony.
> 3. Because for most people the "zero dishes" advantages (no GPU, no setup,
>    elastic) outweigh privacy in *default* situations — but not when the data
>    is secret. That is exactly why the verdict is "conditional adopt".
>
> </details>

---

**Previous:** [Chapter 08 — Speed and money](08-speed-and-money.md) ·
**Next:** [Chapter 10 — vLLM and SpecForge](10-vllm-and-specforge.md)
