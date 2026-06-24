import { Slide, Heading, Text, FlexBox, Notes } from 'spectacle';
import type { JSX } from 'react';
import { colors } from '../theme/colors';

export const theInsightSlides: JSX.Element[] = [
  // ─── Slide 6: The Core Idea ───────────────────────────────────────────
  <Slide key="core-idea" backgroundColor={colors.bg.primary}>
    <FlexBox flexDirection="column" alignItems="center" justifyContent="center" height="100%">
      <Heading fontSize="46px" color={colors.text.primary} margin="0 0 12px 0" style={{ textAlign: 'center' }}>
        The Core Idea: Specification as a Typed Graph
      </Heading>
      <Text fontSize="20px" color={colors.text.secondary} margin="0 0 40px 0" style={{ textAlign: 'center', maxWidth: 800 }}>
        SpecForge transforms human-readable .spec files into a validated, typed entity graph that AI agents consume directly.
      </Text>

      <div style={{ maxWidth: 920, width: '100%', display: 'flex', flexDirection: 'column', gap: 20 }}>
        {/* Data Flow Pipeline */}
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: 12 }}>
          {[
            { label: '.spec files', desc: 'Human-readable DSL', color: colors.accent.teal },
            { label: '→', desc: '', color: colors.text.muted },
            { label: 'Tree-sitter parser', desc: 'Generic block parsing', color: colors.brand },
            { label: '→', desc: '', color: colors.text.muted },
            { label: 'Reference resolver', desc: 'Links all cross-refs', color: colors.brand },
            { label: '→', desc: '', color: colors.text.muted },
            { label: 'Validation (51 rules)', desc: 'Catches errors', color: colors.brand },
            { label: '→', desc: '', color: colors.text.muted },
            { label: 'Graph Protocol (JSON)', desc: 'Standardized output', color: colors.accent.green },
            { label: '→', desc: '', color: colors.text.muted },
            { label: 'Any AI agent', desc: 'Zero parsing required', color: colors.accent.purple },
          ].map((step, i) => (
            step.label === '→' ? (
              <Text key={i} fontSize="28px" color={step.color} margin="0" style={{ flexShrink: 0 }}>{step.label}</Text>
            ) : (
              <div key={i} style={{ flex: 1, display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 6, padding: '14px 10px', background: `${step.color}08`, border: `1px solid ${step.color}30`, borderRadius: 10 }}>
                <Text fontSize="14px" color={step.color} margin="0" fontWeight="bold" style={{ textAlign: 'center', fontFamily: '"JetBrains Mono", monospace' }}>{step.label}</Text>
                <Text fontSize="14px" color={colors.text.muted} margin="0" style={{ textAlign: 'center' }}>{step.desc}</Text>
              </div>
            )
          ))}
        </div>

        {/* Key Points */}
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 20, marginTop: 12 }}>
          {[
            {
              title: 'The Graph is the Product',
              points: [
                'Every node has a kind, every edge has a label',
                'Every constraint is explicit, validated by the compiler',
                'Not prose, not source code, not RAG chunks',
                'If someone builds a better compiler producing this JSON, SpecForge succeeds',
              ],
              color: colors.accent.green,
            },
            {
              title: 'What the Compiler Does',
              points: [
                'Parses every block (Tree-sitter with error recovery)',
                'Resolves every cross-reference (typos become E001 errors)',
                'Validates 51 rules (cycles, orphans, contradictions)',
                'Exports a typed entity graph (typed nodes, typed edges)',
              ],
              color: colors.brand,
            },
          ].map((section) => (
            <div key={section.title} style={{ padding: '18px 20px', background: `${section.color}06`, border: `1px solid ${section.color}20`, borderLeft: `4px solid ${section.color}`, borderRadius: '0 10px 10px 0' }}>
              <Text fontSize="18px" color={section.color} margin="0 0 12px 0" fontWeight="bold">{section.title}</Text>
              <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
                {section.points.map((point) => (
                  <div key={point} style={{ display: 'flex', alignItems: 'flex-start', gap: 8 }}>
                    <div style={{ width: 5, height: 5, borderRadius: '50%', background: section.color, flexShrink: 0, marginTop: 6 }} />
                    <Text fontSize="14px" color={colors.text.secondary} margin="0" style={{ lineHeight: 1.5 }}>{point}</Text>
                  </div>
                ))}
              </div>
            </div>
          ))}
        </div>
      </div>
    </FlexBox>
    <Notes>This is the conceptual heart of SpecForge. The graph is the actual product — not the compiler, not the DSL. The JSON output is standardized. If ten compilers produce this format, SpecForge wins. The compiler's job: parse, resolve, validate, export.</Notes>
  </Slide>,

  // ─── Slide 7: What the Graph Gives Agents ─────────────────────────────
  <Slide key="graph-value" backgroundColor={colors.bg.primary}>
    <FlexBox flexDirection="column" alignItems="center" justifyContent="flex-start" height="100%" style={{ paddingTop: 24 }}>
      <Text fontSize="15px" color={colors.text.muted} margin="0 0 8px 0" style={{ textTransform: 'uppercase', letterSpacing: '0.15em', fontWeight: 600 }}>Agent Consumption</Text>
      <Heading fontSize="44px" color={colors.text.primary} margin="0 0 10px 0" style={{ textAlign: 'center' }}>
        What the Graph Gives Agents
      </Heading>
      <Text fontSize="18px" color={colors.text.secondary} margin="0 0 32px 0" style={{ textAlign: 'center', maxWidth: 820 }}>
        Agents get typed entities, typed edges, validated constraints, and test intent — not ambiguous prose.
      </Text>

      <div style={{ display: 'flex', flexDirection: 'column', gap: 16, maxWidth: 900, width: '100%' }}>
        {[
          {
            category: 'Typed Entities',
            before: 'Reading source code: "there\'s an auth function"',
            after: '{ kind: "behavior", id: "rate_limited_auth", invariants: ["JWT_must_expire"], ports: ["UserStore"], status: "implemented" }',
            benefit: 'No parsing, no ambiguity. Every entity has a declared kind.',
            color: colors.accent.teal,
          },
          {
            category: 'Typed Edges',
            before: 'Guessing from code: "auth probably uses a database"',
            after: '{ source: "rate_limited_auth", target: "UserStore", type: "BehaviorUsesPort" }',
            benefit: 'Explicit relationships. Compiler-validated, never dangling.',
            color: colors.brand,
          },
          {
            category: 'Validated Constraints',
            before: 'Inferring from comments: "tokens should expire"',
            after: '{ kind: "invariant", id: "JWT_must_expire", guarantee: "JWT tokens MUST expire within 15 minutes", risk: "critical" }',
            benefit: 'Machine-readable guarantees with declared risk levels. Relationships tracked via typed edges, not inline fields.',
            color: colors.accent.green,
          },
          {
            category: 'Test Intent',
            before: 'Hoping tests exist: "there should be tests"',
            after: '{ verify: [{ kind: "unit", desc: "rate limit exceeded returns 429" }] }',
            benefit: 'Declared test scenarios, traceable to implementation.',
            color: colors.accent.purple,
          },
        ].map((item) => (
          <div key={item.category} style={{ display: 'flex', flexDirection: 'column', gap: 8, padding: '16px 20px', background: `${item.color}06`, border: `1px solid ${item.color}20`, borderLeft: `4px solid ${item.color}`, borderRadius: '0 10px 10px 0' }}>
            <Text fontSize="16px" color={item.color} margin="0" fontWeight="bold">{item.category}</Text>
            <div style={{ display: 'flex', gap: 16, alignItems: 'flex-start' }}>
              <div style={{ flex: 1 }}>
                <Text fontSize="14px" color={colors.text.muted} margin="0 0 4px 0" style={{ textTransform: 'uppercase', letterSpacing: '0.08em' }}>Without SpecForge</Text>
                <Text fontSize="14px" color={colors.text.secondary} margin="0" style={{ fontStyle: 'italic' }}>{item.before}</Text>
              </div>
              <Text fontSize="20px" color={item.color} margin="0" style={{ flexShrink: 0 }}>→</Text>
              <div style={{ flex: 1 }}>
                <Text fontSize="14px" color={colors.text.muted} margin="0 0 4px 0" style={{ textTransform: 'uppercase', letterSpacing: '0.08em' }}>With SpecForge Graph</Text>
                <Text fontSize="14px" color={item.color} margin="0" style={{ fontFamily: '"JetBrains Mono", monospace' }}>{item.after}</Text>
              </div>
            </div>
            <Text fontSize="14px" color={colors.text.primary} margin="4px 0 0 0" fontWeight="bold">{item.benefit}</Text>
          </div>
        ))}
      </div>

      {/* Token Compression Metric */}
      <div style={{ marginTop: 24, padding: '16px 28px', background: `${colors.accent.purple}08`, border: `2px solid ${colors.accent.purple}`, borderRadius: 12, maxWidth: 900, width: '100%' }}>
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 20 }}>
          <div style={{ textAlign: 'center' }}>
            <Text fontSize="36px" color={colors.accent.purple} margin="0" fontWeight="bold" style={{ fontFamily: '"Inter", sans-serif' }}>45,000</Text>
            <Text fontSize="14px" color={colors.text.muted} margin="4px 0 0 0">tokens (raw source)</Text>
          </div>
          <Text fontSize="32px" color={colors.accent.purple} margin="0">→</Text>
          <div style={{ textAlign: 'center' }}>
            <Text fontSize="36px" color={colors.accent.purple} margin="0" fontWeight="bold" style={{ fontFamily: '"Inter", sans-serif' }}>2,400</Text>
            <Text fontSize="14px" color={colors.text.muted} margin="4px 0 0 0">tokens (validated graph)</Text>
          </div>
          <div style={{ padding: '0 20px', borderLeft: `2px solid ${colors.accent.purple}40` }}>
            <Text fontSize="40px" color={colors.accent.purple} margin="0" fontWeight="bold" style={{ fontFamily: '"Inter", sans-serif' }}>19x</Text>
            <Text fontSize="14px" color={colors.text.muted} margin="4px 0 0 0">compression</Text>
          </div>
        </div>
      </div>
    </FlexBox>
    <Notes>This slide shows the concrete value proposition for agents. Not just compression — semantic compression. Every token in the graph carries validated meaning. Agents get typed entities, typed edges, validated constraints, and test intent — all compiler-validated, zero ambiguity.</Notes>
  </Slide>,
];
