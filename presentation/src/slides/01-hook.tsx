import { Slide, Heading, Text, FlexBox, Notes } from 'spectacle';
import React from 'react';
import type { JSX } from 'react';
import { colors } from '../theme/colors';

export const hookSlides: JSX.Element[] = [
  // ─── Slide 1: Title ────────────────────────────────────────────────────
  <Slide key="title" backgroundColor={colors.bg.primary}>
    <FlexBox flexDirection="column" alignItems="center" justifyContent="center" height="100%">
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 48, padding: '8px 20px', background: `${colors.accent.teal}10`, border: `1px solid ${colors.accent.teal}30`, borderRadius: 999 }}>
        <div style={{ width: 7, height: 7, borderRadius: '50%', background: colors.accent.teal, boxShadow: `0 0 6px ${colors.accent.teal}` }} />
        <span style={{ fontSize: 14, fontWeight: 600, letterSpacing: '0.12em', textTransform: 'uppercase', color: colors.accent.teal, fontFamily: '"Inter", sans-serif' }}>
          Open Source &middot; Written in Rust
        </span>
      </div>

      <Heading fontSize="84px" color={colors.text.primary} margin="0 0 16px 0" style={{ textAlign: 'center', fontWeight: 800 }}>
        SpecForge
      </Heading>

      <Text fontSize="34px" color={colors.brand} margin="0 0 32px 0" style={{ textAlign: 'center', fontWeight: 600 }}>
        A Compiler for AI Context
      </Text>

      <Text fontSize="22px" color={colors.text.secondary} margin="0 0 40px 0" style={{ textAlign: 'center', maxWidth: 760, lineHeight: 1.7 }}>
        You write .spec files. The compiler parses, validates, resolves cross-references,
        and exports a <span style={{ color: colors.accent.teal, fontWeight: 700 }}>typed entity graph</span>.
        AI agents read the graph instead of guessing from source code.
      </Text>

      <div style={{ display: 'flex', gap: 20 }}>
        {[
          { label: 'The Problem', desc: 'Why agents get domains wrong', color: colors.accent.red },
          { label: 'The DSL', desc: 'keyword name { fields }', color: colors.accent.teal },
          { label: 'Extensions', desc: '4 shipped, 22 entity kinds', color: colors.accent.purple },
          { label: 'The Compiler', desc: 'Parse → Validate → Graph', color: colors.accent.green },
        ].map((item) => (
          <div key={item.label} style={{ display: 'flex', alignItems: 'center', gap: 8, padding: '6px 14px', background: `${item.color}08`, border: `1px solid ${item.color}20`, borderRadius: 8 }}>
            <div style={{ width: 6, height: 6, borderRadius: '50%', background: item.color }} />
            <span style={{ fontSize: 14, color: item.color, fontWeight: 600, fontFamily: '"Inter", sans-serif' }}>{item.label}</span>
            <span style={{ fontSize: 14, color: colors.text.muted, fontFamily: '"Inter", sans-serif' }}>{item.desc}</span>
          </div>
        ))}
      </div>
    </FlexBox>
    <Notes>Technical deep-dive, not a pitch. We'll cover the problem, every core concept with real DSL syntax, extension architecture, and the compiler pipeline.</Notes>
  </Slide>,

  // ─── Slide 2: What SpecForge Actually Is ────────────────────────────────
  <Slide key="what-is" backgroundColor={colors.bg.primary}>
    <FlexBox flexDirection="column" alignItems="center" justifyContent="center" height="100%">
      <Heading fontSize="44px" color={colors.text.primary} margin="0 0 40px 0" style={{ textAlign: 'center' }}>
        What Is SpecForge?
      </Heading>

      <div style={{ maxWidth: 900, width: '100%', display: 'grid', gridTemplateColumns: 'auto 1fr', gap: '20px 24px', alignItems: 'start' }}>
        {[
          {
            term: 'DSL',
            bullet: '.spec files',
            desc: 'Block syntax: keyword name { fields }. Extensions define valid keywords. References are compiler-resolved.',
            color: colors.accent.teal
          },
          {
            term: 'Compiler',
            bullet: 'specforge check',
            desc: 'Tree-sitter parser, reference resolver, 51 diagnostic rules, graph builder. Written in Rust.',
            color: colors.brand
          },
          {
            term: 'Graph Protocol',
            bullet: 'specforge export',
            desc: 'Standardized JSON output. Typed entities, typed edges, deterministic ordering. This is the product.',
            color: colors.accent.green
          },
          {
            term: 'Extensions',
            bullet: 'Wasm plugins',
            desc: 'Zero domain in core. Entity kinds, edge types, validation rules, CLI commands — all from sandboxed Wasm.',
            color: colors.accent.purple
          },
        ].map((item) => (
          <React.Fragment key={item.term}>
            <div style={{
              textAlign: 'right',
              paddingTop: 4,
              borderRight: `3px solid ${item.color}`,
              paddingRight: 16
            }}>
              <Text fontSize="20px" color={item.color} margin="0" fontWeight="bold" style={{ fontFamily: '"Inter", sans-serif' }}>
                {item.term}
              </Text>
            </div>
            <div style={{ paddingTop: 4 }}>
              <Text fontSize="18px" color={colors.text.primary} margin="0 0 6px 0" fontWeight="700" style={{ fontFamily: '"JetBrains Mono", monospace' }}>
                {item.bullet}
              </Text>
              <Text fontSize="16px" color={colors.text.secondary} margin="0" style={{ lineHeight: 1.6 }}>
                {item.desc}
              </Text>
            </div>
          </React.Fragment>
        ))}
      </div>

      <div style={{ padding: '16px 28px', background: 'rgba(255,255,255,0.03)', border: '1px solid rgba(255,255,255,0.08)', borderRadius: 10, maxWidth: 900, marginTop: 40 }}>
        <Text fontSize="18px" color={colors.text.primary} margin="0" style={{ textAlign: 'center' }}>
          <span style={{ fontWeight: 700 }}>Not</span> a code generator. <span style={{ fontWeight: 700 }}>Not</span> a test runner.
          SpecForge provides <span style={{ color: colors.accent.teal, fontWeight: 700 }}>validated context</span> — agents produce the output.
        </Text>
      </div>
    </FlexBox>
    <Notes>Four components: DSL (human-writable), Compiler (validation/resolution), Graph Protocol (agent-consumable JSON - THIS is the product), Extensions (domain knowledge). The graph is the product, not the compiler.</Notes>
  </Slide>,
];
