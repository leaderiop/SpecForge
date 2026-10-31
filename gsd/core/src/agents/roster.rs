//! The 15 `@gsd/core` agents: id, description, host-neutral tool classes and colour, as gsd-core v1.19.0 declares
//! them (`agents/gsd-<name>.md` frontmatter), and the embedded bodies.

/// One agent.
pub struct Agent {
    pub id: &'static str,
    pub description: &'static str,
    pub tools: &'static [&'static str],
    pub color: &'static str,
    /// The upstream path of its full body, under `agents/`.
    pub full: &'static str,
    /// The compact body, for the agents upstream ships one for.
    pub compact: Option<&'static str>,
}

pub const AGENTS: &[Agent] = &[
    Agent {
        id: "gsd-advisor-researcher",
        description: "Researches a single gray area decision and returns a structured comparison table with rationale. Spawned by discuss-phase advisor mode.",
        tools: &[
            "read",
            "shell",
            "search",
            "skill",
            "web_search",
            "web_fetch",
            "mcp:context7/*",
            "mcp:plugin_context7_context7/*",
        ],
        color: "cyan",
        full: include_str!("../../assets/agents/gsd-advisor-researcher.md"),
        compact: Some(include_str!(
            "../../assets/agents/gsd-advisor-researcher.compact.md"
        )),
    },
    Agent {
        id: "gsd-assumptions-analyzer",
        description: "Deeply analyzes codebase for a phase and returns structured assumptions with evidence. Spawned by discuss-phase assumptions mode.",
        tools: &["read", "shell", "search", "skill"],
        color: "cyan",
        full: include_str!("../../assets/agents/gsd-assumptions-analyzer.md"),
        compact: Some(include_str!(
            "../../assets/agents/gsd-assumptions-analyzer.compact.md"
        )),
    },
    Agent {
        id: "gsd-doc-classifier",
        description: "Classifies a single planning document as ADR, PRD, SPEC, DOC, or UNKNOWN. Extracts title, scope summary, and cross-references. Spawned in parallel by /gsd:ingest-docs. Writes a JSON classification file and returns a one-line confirmation.",
        tools: &["read", "write", "search"],
        color: "yellow",
        full: include_str!("../../assets/agents/gsd-doc-classifier.md"),
        compact: Some(include_str!(
            "../../assets/agents/gsd-doc-classifier.compact.md"
        )),
    },
    Agent {
        id: "gsd-doc-synthesizer",
        description: "Synthesizes classified planning docs into a single consolidated context. Applies precedence rules, detects cross-ref cycles, enforces LOCKED-vs-LOCKED hard-blocks, and writes INGEST-CONFLICTS.md with three buckets (auto-resolved, competing-variants, unresolved-blockers). Spawned by /gsd:ingest-docs.",
        tools: &["read", "write", "search", "shell"],
        color: "orange",
        full: include_str!("../../assets/agents/gsd-doc-synthesizer.md"),
        compact: Some(include_str!(
            "../../assets/agents/gsd-doc-synthesizer.compact.md"
        )),
    },
    Agent {
        id: "gsd-executor",
        description: "Executes GSD plans with atomic commits, deviation handling, checkpoint protocols, and state management. Spawned by execute-phase orchestrator or execute-plan command.",
        tools: &[
            "read",
            "write",
            "edit",
            "shell",
            "search",
            "skill",
            "mcp:context7/*",
            "mcp:plugin_context7_context7/*",
        ],
        color: "yellow",
        full: include_str!("../../assets/agents/gsd-executor.md"),
        compact: None,
    },
    Agent {
        id: "gsd-integration-checker",
        description: "Verifies cross-phase integration and E2E flows. Checks that phases connect properly and user workflows complete end-to-end.",
        tools: &["read", "shell", "search", "skill"],
        color: "blue",
        full: include_str!("../../assets/agents/gsd-integration-checker.md"),
        compact: Some(include_str!(
            "../../assets/agents/gsd-integration-checker.compact.md"
        )),
    },
    Agent {
        id: "gsd-nyquist-auditor",
        description: "Fills Nyquist validation gaps by generating tests and verifying coverage for phase requirements",
        tools: &["read", "write", "edit", "shell", "search", "skill"],
        color: "purple",
        full: include_str!("../../assets/agents/gsd-nyquist-auditor.md"),
        compact: Some(include_str!(
            "../../assets/agents/gsd-nyquist-auditor.compact.md"
        )),
    },
    Agent {
        id: "gsd-pattern-mapper",
        description: "Analyzes codebase for existing patterns and produces PATTERNS.md mapping new files to closest analogs. Read-only codebase analysis spawned by /gsd:plan-phase orchestrator before planning.",
        tools: &["read", "shell", "search", "write"],
        color: "purple",
        full: include_str!("../../assets/agents/gsd-pattern-mapper.md"),
        compact: Some(include_str!(
            "../../assets/agents/gsd-pattern-mapper.compact.md"
        )),
    },
    Agent {
        id: "gsd-phase-researcher",
        description: "Researches how to implement a phase before planning. Produces RESEARCH.md consumed by gsd-planner. Spawned by /gsd:plan-phase orchestrator.",
        tools: &[
            "read",
            "write",
            "edit",
            "shell",
            "search",
            "skill",
            "web_search",
            "web_fetch",
            "mcp:context7/*",
            "mcp:plugin_context7_context7/*",
            "mcp:firecrawl/*",
            "mcp:exa/*",
            "mcp:tavily/*",
            "mcp:ref/*",
            "mcp:jina/*",
            "mcp:perplexity/*",
        ],
        color: "cyan",
        full: include_str!("../../assets/agents/gsd-phase-researcher.md"),
        compact: None,
    },
    Agent {
        id: "gsd-plan-checker",
        description: "Verifies plans will achieve phase goal before execution. Goal-backward analysis of plan quality. Spawned by /gsd:plan-phase orchestrator.",
        tools: &["read", "shell", "search", "skill"],
        color: "green",
        full: include_str!("../../assets/agents/gsd-plan-checker.md"),
        compact: None,
    },
    Agent {
        id: "gsd-planner",
        description: "Creates executable phase plans with task breakdown, dependency analysis, and goal-backward verification. Spawned by /gsd:plan-phase orchestrator.",
        tools: &[
            "read",
            "write",
            "edit",
            "shell",
            "search",
            "skill",
            "web_fetch",
            "mcp:context7/*",
            "mcp:plugin_context7_context7/*",
        ],
        color: "green",
        full: include_str!("../../assets/agents/gsd-planner.md"),
        compact: None,
    },
    Agent {
        id: "gsd-project-researcher",
        description: "Researches domain ecosystem before roadmap creation. Produces files in spec/gsd/research/ consumed during roadmap creation. Spawned by /gsd:new-project or /gsd:new-milestone orchestrators.",
        tools: &[
            "read",
            "write",
            "shell",
            "search",
            "skill",
            "web_search",
            "web_fetch",
            "mcp:context7/*",
            "mcp:plugin_context7_context7/*",
            "mcp:firecrawl/*",
            "mcp:exa/*",
            "mcp:tavily/*",
            "mcp:ref/*",
            "mcp:jina/*",
            "mcp:perplexity/*",
        ],
        color: "cyan",
        full: include_str!("../../assets/agents/gsd-project-researcher.md"),
        compact: Some(include_str!(
            "../../assets/agents/gsd-project-researcher.compact.md"
        )),
    },
    Agent {
        id: "gsd-research-synthesizer",
        description: "Synthesizes research outputs from parallel researcher agents into SUMMARY.md. Spawned by /gsd:new-project after 4 researcher agents complete.",
        tools: &["read", "write", "shell", "skill"],
        color: "purple",
        full: include_str!("../../assets/agents/gsd-research-synthesizer.md"),
        compact: Some(include_str!(
            "../../assets/agents/gsd-research-synthesizer.compact.md"
        )),
    },
    Agent {
        id: "gsd-roadmapper",
        description: "Creates project roadmaps with phase breakdown, requirement mapping, success criteria derivation, and coverage validation. Spawned by /gsd:new-project orchestrator.",
        tools: &["read", "write", "shell", "search", "skill"],
        color: "purple",
        full: include_str!("../../assets/agents/gsd-roadmapper.md"),
        compact: Some(include_str!(
            "../../assets/agents/gsd-roadmapper.compact.md"
        )),
    },
    Agent {
        id: "gsd-verifier",
        description: "Verifies phase goal achievement through goal-backward analysis. Checks codebase delivers what phase promised, not just that tasks completed. Creates VERIFICATION.md report.",
        tools: &["read", "write", "shell", "search", "skill"],
        color: "green",
        full: include_str!("../../assets/agents/gsd-verifier.md"),
        compact: None,
    },
];
