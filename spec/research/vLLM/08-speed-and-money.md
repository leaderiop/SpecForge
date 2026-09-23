# Chapter 08 — Speed and money: how kitchens are judged

> **In one sentence:** kitchens are judged by four numbers (how many plates per
> hour, how long to the first bite, the gap between bites, and dropped plates)
> and by one money question: **rent your own food truck, or pay per burger at
> someone else's restaurant?**

---

## The four speed numbers

**1. Plates per hour (throughput).**
How many answers the whole kitchen produces per hour when a crowd is asking.
This is the number PagedAttention and continuous batching inflate — the founding
24× claim lives here.

**2. Wait for the first bite (TTFT — time to first token).**
You asked at noon. The first word of the answer arrives at 12:00 and 2 seconds.
That wait matters enormously to humans: it *feels* like the whole speed.

**3. Gap between bites (TPOT — time per output token).**
After the first word, how long between each next word? (About 0.07 seconds on a
big modern counter — faster than you can read.)

**4. Dropped plates (success rate).**
Under a crush of 100 people at once, does every order come out? In the university
study, vLLM: **all 100 served, none dropped**. A wobbly kitchen that drops plates
under load fails this number no matter how fast it is when empty.

> **Grown-up word:** throughput, TTFT, TPOT (also called inter-token latency),
> and success rate — the four KPIs of the RES-31 dossier, Section 04, where each
> has a target and a measurement recipe [E4], [E5], [E8].

---

## The money question: rent the truck or buy the burger?

Two ways to get AI answers:

**Way A — pay per burger (managed API).** A big company runs the kitchen; you
pay for each mouthful. Prices (per million words, roughly, 2026):

| Burger stand | Reading your text | Writing the answer |
|---|---|---|
| Qwen Flash (budget stand) | $0.03 | $0.13 |
| DeepSeek V3.2 | $0.28 | $0.42 |
| GPT-4.1 | $2.00 | $8.00 |
| Claude Sonnet | $3.00 | $15.00 |

**Way B — rent the food truck (self-host with vLLM).** You rent a big counter
yourself (an H100 GPU: about **$1.73–4 per hour** depending on the shop) and
cook unlimited burgers. Fixed cost, no per-burger fee.

---

## When does the food truck win?

SpecForge measured it with a realistic work order (read ~30,000 words, write
~3,000 — call it **one "infer task"**: about 1¢ to 14¢ per task depending on
the stand):

| Your monthly workload | Rent the truck (burst, ~$304/month) beats... |
|---|---|
| **~3,100 tasks/month** | ...Claude Sonnet burgers |
| **~5,000 tasks/month** | ...GPT-4.1 burgers |
| **~14,800 tasks/month** | ...GPT-4.1 burgers even with the truck running nonstop |

But here is the honest catch: the **budget stands** (DeepSeek at ~1¢/task, Qwen
at a tenth of a cent) are **almost impossible to beat on price alone** — the
24/7 truck would need ~129,000 tasks a month just to tie DeepSeek.

So why rent the truck at all? One giant reason: **secrets.** With your own
truck, your source code **never leaves your building** — zero bytes. With a
burger stand, every word you send travels to someone else's computer. For
private code, that reason alone can decide it.

> **Check the evidence:** truck prices [E11], burger prices [E13], the task
> model and every break-even number are computed in RES-31 Exhibit 05 and the
> assumptions register [A1–A4]. The full-graph embedding cost (~$0.01) is [A6].

---

## One cent, to map a whole library

And the mapmaker's bill? Mapping SpecForge's entire recipe book — about 2,000
recipes — costs about **half a million words**, which at the burger stand is
**one cent**. Remember that number next time someone proposes buying a GPU
"for the embeddings": this is why RES-31 says embeddings alone never justify a
truck.

---

> **Check yourself**
>
> 1. Which number matters most to how *fast a website feels*?
> 2. Roughly when does renting beat GPT-4.1 burgers?
> 3. What is the one reason that beats any price for self-hosting?
>
> <details><summary>Answers</summary>
>
> 1. TTFT — the wait for the first word dominates how fast it *feels*.
> 2. Around 3,100–5,000 work orders a month (burst schedule).
> 3. Privacy: self-hosted means zero source code leaves the building.
>
> </details>

---

**Previous:** [Chapter 07 — The universal plug](07-universal-plug-and-embeddings.md) ·
**Next:** [Chapter 09 — The great kitchen race](09-the-great-kitchen-race.md)
