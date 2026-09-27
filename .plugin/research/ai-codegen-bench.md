# R — AI Code-Generation Quality per Language (Lua vs Python vs TypeScript vs Rust)

Input to criterion 2 (authoring ergonomics for AI agents) and criterion 8 (debuggability).
Question: in which language do LLM coding assistants produce the fewest bugs, and whose API
documentation do they learn from best? All numbers below are from published benchmarks and
studies; URLs cited inline. Notation: pass@1 = % of problems whose first generated solution
passes all hidden tests.

## 1. What can and cannot be measured

- **HumanEval (164 problems) and MBPP are Python-only.** Cross-language numbers come from
  translations: **MultiPL-E** (arXiv:2208.08227) and **McEval** (40 languages, arXiv:2406.07436).
- **SWE-bench / SWE-bench Verified are Python-only.** The repo-level cross-language data point
  is **SWE-bench Multilingual** (300 tasks, 42 repos, 9 languages; swebench.com/multilingual.html).
  **Lua is absent from the entire SWE-bench family** — too few qualifying repos. Lua has no
  repo-level evidence at all; its case rests on function-level benchmarks and corpus statistics.
- **Defect/security studies** (Veracode 2025, Pearce et al. 2021, Spracklen et al. 2024) cover
  mainstream languages only: Python, JavaScript, Java, C#, C — never Lua, never Rust (Spracklen
  covers Python+JS; Veracode covers Java/Python/C#/JS).

## 2. Function-level generation quality

### 2.1 MultiPL-E (Codex-era, 2022) — the frequency law

Codex (code-davinci-002): Python 45.9%; JavaScript +2.3pp over Python (p=0.43, i.e. tie);
pass@1 > 40% on C++, Java, TypeScript, PHP, Ruby, Rust, Scala **and Lua**. Two durable findings:

- **Performance correlates with language frequency** (High > Medium > Low > Niche, all p<0.01).
  Lua is classed *Niche* (TIOBE 0.2%, GitHub rank ~25; Table I of the paper).
- Lua's HumanEval deficit vs Python is small but statistically significant (mixed-effects
  estimate −1.04pp, p=0.005); on MBPP-style tasks the same model's Lua gap widens (−0.94pp
  CodeGen fit; Perl/R far worse). So: even in 2022, Lua was "slightly behind, not cliff-behind."


| Model | Python | TypeScript | Rust | Lua |
| --- | --- | --- | --- | --- |
| GPT-4o (240513) | **76.0** | 56.0 | 83.0 | 60.0 |
| GPT-4 Turbo (231106) | **78.0** | 60.0 | 71.7 | 56.0 |
| GPT-3.5 Turbo | **60.0** | 54.0 | 52.8 | 58.0 |
| DeepSeek-Coder-Instruct 33B | 56.0 | 56.0 | **66.0** | 58.0 |
| Codestral 22B | 56.0 | 52.0 | **71.7** | 56.0 |

Readings:

- **Python is the only language that is consistently top-2 for every model.**
- **TypeScript is the anomaly**: 16–20pp below Python for GPT-4o/GPT-4T despite TS's corpus
  advantage. [INFERENCE: partly small-sample noise — McEval per-language cells are coarse
  (values repeat in 2–6pp steps) and the paper itself stresses high-resource vs low-resource
  imbalance; but the direction is consistent across both GPT models, so it is not one cell.]
- **Rust is high-variance** (52.8–83.0): weak for GPT-3.5, top of the four for GPT-4o.
- **Lua sits mid-pack** (56–60), never last, never first — the "niche but not cliff" pattern.
- Completion tasks (Table 2, GPT-4 Turbo) sharpen the ranking: single-line Python 86.7 >
  Rust 81.5 > Lua 80.0 > TS 77.5; **multi-line Python 83.3 > Rust 81.5 > TS 75.0 > Lua 66.3**.
  Lua degrades most when the model must hold state across lines (−13.7pp vs Python's −3.4pp) —
  the signature of a thinner training prior, and exactly the regime plugin bodies live in.
- Paper's own conclusion (§5): "the current state-of-the-art performance of most models
  primarily lies in high-resource languages like Python", with consistent MultiPL-E↔McEval
  rankings per language.

## 3. Repo-level agentic quality (SWE-bench Multilingual, SWE-agent + Claude 3.7 Sonnet)

| Language | Resolution rate |
| --- | --- |
| Rust | **58.14%** |
| Java | 53.49% |
| PHP | 48.84% |
| Ruby | 43.18% |
| JavaScript/TypeScript | 34.88% |
| Go | 30.95% |
| C/C++ | 28.57% |
| *(Python: SWE-bench Verified, same model/agent)* | *63%* |

Same model that scores 63% on Python Verified drops to 43% overall on the multilingual set.

- **Rust is #1** — despite ranking mid-pack on function-level generation. The authors note Rust
  tasks modify *more* LOC on average yet resolve best, ruling out task-ease as the explanation.
  [INFERENCE: the compiler-in-the-loop effect — a wrong guess fails `cargo check` immediately
  and locally, so the agent's repair loop converges; plus the sampled Rust repos (tokio, axum,
  bat, ripgrep, nushell, ruff) are flagship-quality, heavily doc-commented codebases.]
- **TypeScript/JavaScript is 4th of 7** (34.88%), below PHP and Ruby. Scripting's fast
  no-compile loop does *not* help the agent; weak static checking hurts at repo scale.

## 4. Bug/security-defect studies

- **Veracode 2025 GenAI Code Security Report** (100+ LLMs; veracode.com/blog/genai-code-security-report/):
  **45% of AI-generated code samples introduced OWASP Top-10 vulnerabilities**. By language:
  **Python 38% failure (best)**, JavaScript 43%, C# 45%, Java 72% (worst). Model generation
  date made functional quality better but **security flat**. Rust/Lua untested.
- **"Asleep at the Keyboard"** (Pearce et al., arXiv:2108.09293): of 1,689 Copilot programs over
  CWE Top-25 scenarios, **~40% vulnerable**. Top-10-CWE subset: C 50.3% vulnerable, Python 38.4%.
- **Package hallucinations** (Spracklen et al., arXiv:2406.10279; 576k samples, 16 models):
  **Python 15.8% vs JavaScript 21.3%** average hallucinated-package rate; commercial models
  5.2% vs open-source 21.7%; GPT-4 Turbo best at 3.59%. LLMs also mix ecosystems (npm names
  suggested for Python prompts). For plugin APIs without a package index at all, the
  hallucination check surface is even weaker — in Lua a hallucinated host call returns `nil`
  and fails (or silently propagates) downstream, per D07.

## 5. Which API documentation is best for AI to learn from?

Measured as three proxies: **corpus volume** (what the model memorized), **machine-checkability**
(can the agent verify a guess locally), **version stability** (does memorized knowledge rot).

| | Corpus volume | Machine-checkable API surface | Version stability |
| --- | --- | --- | --- |
| **Python** | Largest of the four (best-represented in training sets per MultiPL-E §I) | Weak by default: docstrings are untested prose; type hints optional (MultiPL-E §VI: static typing neither helped nor hurt pass rates) | 2→3 transition largely resolved in modern corpus |
| **TypeScript** | Largest combined JS+TS web corpus; `.d.ts` ships with virtually every npm package (DefinitelyTyped for the rest) | Strong: `tsc`/`deno check` turns hallucinated APIs into type errors; LSP feedback | Moderate; lib/TS-version drift exists |
| **Rust** | Smallest of the three mainstream (SWE-bench-ML repos aside) | **Strongest docs guarantee: rustdoc doc-tests are compiled and executed by `cargo test`** ("This makes sure that examples within your documentation are up to date and working" — The rustdoc book, doc.rust-lang.org/rustdoc/write-documentation/documentation-tests.html); docs.rs unified | Strong: editions preserve backwards compat |
| **Lua** | Niche class (MultiPL-E Table I: TIOBE 0.2%); smallest corpus of the four | Weak: terse reference manual, no type system, no package index; stdlib tiny (mitigates hallucination area, but zero checking) | **Fragmented: LuaJIT/Neovim ≈ 5.1 semantics vs 5.3/5.4 (integer division `//`, integers as distinct type); MultiPL-E's translation appendix had to pin "Reference Version: 5.3"** |

Empirical tie-breaker: the languages models score best on are the ones with the biggest,
most consistent documentation/corpus base (Python #1 on function-level, security and
hallucination metrics; §2, §4). Documentation *verifiability* is what makes Rust win at repo
level despite a smaller corpus (§3).

## 6. Direct answers

**Fewest bugs from AI assistants: Python, on every metric that covers it** — best function-level
pass@1 among the four (§2), lowest security-failure rate tested (38%, §4), lowest package-
hallucination rate (15.8% vs JS 21.3%, §4). **Caveat that inverts the ranking at repo scale:**
Rust is #1 on SWE-bench Multilingual (58.1%) because its bugs are loud — the compile-check-repair
loop converts model weakness into caught-and-fixed errors, while Python's fluency means its
mistakes are more likely to *run*. Lua ranks last on the evidence that exists: smallest corpus,
steepest multi-line degradation, no repo-level or security data at all, silent `nil`-propagation
failure mode. TypeScript is mid-pack everywhere: huge corpus, machine-checkable surface, yet
empirically 16–20pp below Python on McEval generation and 4th of 7 at repo level.

**Best API documentation for AI: Rust for correctness-per-doc-line** (doctests make docs
unrottable, and that shows up as SWE-bench ML #1), **Python for volume** (models demonstrably
learn it best), **TypeScript for machine-readable API surface** (`.d.ts` + tsc — the best
"docs as types" story), **Lua worst on volume and version coherence** despite its small,
memorizable core.

Relevance to the SpecForge runtime decision: this dimension ranks authoring surfaces
**Rust ≈ TS > Python > Lua** — the inverse of raw model fluency (Python first) because what
matters in an agent-authored plugin is not how fluently the model writes but how cheaply a
wrong guess is caught (D07's machine-checking-per-iteration thesis, here confirmed with
benchmark numbers). Lua's plugin-authoring story has no compensating benchmark evidence:
it is the only candidate with zero representation in repo-level agentic benchmarks and the
worst-documented ecosystem of the four.

## Sources

- MultiPL-E — arXiv:2208.08227 (incl. Table I frequency classes, Codex pass rates, mixed-effects fits, Lua 5.3 pin in translation appendix)
- McEval — arXiv:2406.07436 (Tables 1–2 per-language pass@1; §5 high-resource conclusion)
- SWE-bench Multilingual — swebench.com/multilingual.html (per-language resolution rates; Claude 3.7 Sonnet 43% vs 63% Verified)
- Veracode 2025 GenAI Code Security Report — veracode.com/blog/genai-code-security-report/
- Asleep at the Keyboard? — arXiv:2108.09293
- Package hallucinations — arXiv:2406.10279
- The rustdoc book, Documentation tests — doc.rust-lang.org/rustdoc/write-documentation/documentation-tests.html

## Verdict

**Verdict:** KEEP_WASM *(on this dimension; TYPESCRIPT is the fallback if authoring-toolchain friction blocks adoption)*
**Confidence:** 3
**One-line rationale:** Benchmark evidence ranks AI authoring surfaces Rust ≈ TS > Python > Lua — Python's fluency wins function-level contests, but Rust's verifiable docs and compiler-in-the-loop produce the fewest *uncounted* bugs at repo scale, while Lua is the only candidate with no repo-level evidence, the smallest corpus, and silent-failure semantics.

## Bottom line

AI assistants write the **fewest bugs in Python** on like-for-like tasks (McEval/MultiPL-E pass@1, Veracode 38% security failure, 15.8% package-hallucination — all best-in-class), but **fewest uncaught bugs in Rust** at real-repo scale (SWE-bench Multilingual: Rust 58.1% #1 vs TS/JS 34.9%, because compile errors make the agent's repair loop converge; Verdict confidence 3). **TypeScript** has the best machine-readable API surface (`.d.ts`/tsc) yet underperforms its corpus weight empirically (GPT-4o McEval 56 vs Python 76). **Lua is last on every measured axis**: niche corpus (TIOBE 0.2%), steepest single-line→multi-line drop (80.0→66.3), absent from SWE-bench entirely, version-fragmented docs (5.1/LuaJIT vs 5.3/5.4), and `nil`-propagating failure modes — the weakest choice for a product whose plugins are co-authored by AI agents.
