import { Slide, Heading, Text, FlexBox, Notes } from 'spectacle';
import type { JSX } from 'react';
import { colors } from '../theme/colors';

export const closingSlides: JSX.Element[] = [
  // ─── Slide 36: What Ships Today ───────────────────────────────────────
  <Slide key="what-ships-today" backgroundColor={colors.bg.primary}>
    <FlexBox flexDirection="column" alignItems="center" justifyContent="center" height="100%">
      <Heading fontSize="44px" color={colors.text.primary} margin="0 0 32px 0" style={{ textAlign: 'center' }}>
        What Ships Today
      </Heading>

      <div style={{ display: 'flex', gap: 20, maxWidth: 880, width: '100%', marginBottom: 0 }}>
        <div style={{ flex: 1, padding: '18px 20px', background: colors.bg.card, border: '1px solid rgba(255,255,255,0.06)', borderRadius: 12 }}>
          <Text fontSize="18px" color={colors.accent.teal} margin="0 0 14px 0" fontWeight="bold">Implementation Status</Text>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
            {[
              'Tree-sitter parser with error recovery',
              '4-stage compiler pipeline (Rust)',
              'petgraph entity graph with typed edges',
              'ariadne diagnostic output (file:line:col)',
              'Extism Wasm extension runtime (AOT cached)',
              'LSP server (diagnostics, hover, go-to-def, completions)',
              'MCP server (24 tools, 7 resources, 5 prompts)',
              '4 extensions: software (5 kinds), product (9), governance (3), formal (5)',
              '22 entity kinds, 55 edge types, 51 diagnostic codes',
              'Sub-100ms incremental recompilation'
            ].map((item) => (
              <div key={item} style={{ display: 'flex', gap: 10, alignItems: 'flex-start' }}>
                <div style={{ width: 6, height: 6, borderRadius: '50%', background: colors.accent.green, marginTop: 7, flexShrink: 0 }} />
                <Text fontSize="14px" color={colors.text.secondary} margin="0" style={{ lineHeight: 1.5 }}>{item}</Text>
              </div>
            ))}
          </div>
        </div>

        <div style={{ flex: 1, padding: '18px 20px', background: colors.bg.card, border: '1px solid rgba(255,255,255,0.06)', borderRadius: 12 }}>
          <Text fontSize="18px" color={colors.accent.purple} margin="0 0 14px 0" fontWeight="bold">Architecture Principles</Text>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
            {[
              'Zero domain knowledge in compiler core',
              'Extensions define ALL entity kinds, edge types, validation rules',
              'The Graph Protocol (JSON) is the product',
              'Not a code generator — provides context, agents produce output',
              'Not a test runner — traces tests, consumes results',
              'Structure is a spectrum — one spec file already improves output',
              'Wasm sandbox for extension isolation',
              'Deterministic output — same spec → same graph → same JSON'
            ].map((item) => (
              <div key={item} style={{ display: 'flex', gap: 10, alignItems: 'flex-start' }}>
                <div style={{ width: 6, height: 6, borderRadius: '50%', background: colors.accent.purple, marginTop: 7, flexShrink: 0 }} />
                <Text fontSize="14px" color={colors.text.secondary} margin="0" style={{ lineHeight: 1.5 }}>{item}</Text>
              </div>
            ))}
          </div>
        </div>
      </div>
    </FlexBox>
    <Notes>Technical summary of what's implemented today. Left side shows concrete implementation details. Right side shows non-negotiable architectural principles.</Notes>
  </Slide>,

  // ─── Slide 37: Try It ──────────────────────────────────────────────────
  <Slide key="try-it" backgroundColor={colors.bg.primary}>
    <FlexBox flexDirection="column" alignItems="center" justifyContent="center" height="100%">
      <Heading fontSize="44px" color={colors.text.primary} margin="0 0 28px 0" style={{ textAlign: 'center' }}>
        Try It
      </Heading>

      <div style={{ maxWidth: 720, width: '100%', background: '#0a0a12', borderRadius: 14, border: '1px solid rgba(255,255,255,0.08)', padding: '20px 32px', fontFamily: '"JetBrains Mono", monospace', fontSize: 18, lineHeight: 2.2, marginBottom: 24 }}>
        <div><span style={{ color: colors.accent.green }}>$</span><span style={{ color: colors.text.primary }}> cargo install specforge-cli</span></div>
        <div><span style={{ color: colors.accent.green }}>$</span><span style={{ color: colors.text.primary }}> specforge init --extensions @specforge/software</span></div>
        <div><span style={{ color: colors.accent.green }}>$</span><span style={{ color: colors.text.primary }}> specforge check && specforge export --format=context</span></div>
      </div>

      <Text fontSize="17px" color={colors.text.secondary} margin="0 0 28px 0" style={{ textAlign: 'center', maxWidth: 680 }}>
        Dogfooding: SpecForge's own domain model is specified in 180+ .spec files.
      </Text>

      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: 18, maxWidth: 780, width: '100%' }}>
        {[
          { label: 'GitHub', value: 'github.com/specforge/specforge', color: colors.accent.teal },
          { label: 'Language', value: 'Written in Rust, 90K+ lines', color: colors.accent.green },
          { label: 'Tests', value: '2,600+ passing tests', color: colors.accent.purple },
        ].map((item) => (
          <div key={item.label} style={{ padding: '14px 18px', background: `${item.color}06`, borderLeft: `3px solid ${item.color}`, borderRadius: '0 10px 10px 0' }}>
            <Text fontSize="13px" color={item.color} margin="0 0 4px 0" fontWeight="bold" style={{ textTransform: 'uppercase', letterSpacing: '0.08em' }}>{item.label}</Text>
            <Text fontSize="14px" color={colors.text.secondary} margin="0" style={{ lineHeight: 1.4 }}>{item.value}</Text>
          </div>
        ))}
      </div>

      <Text fontSize="16px" color={colors.text.muted} margin="28px 0 0 0" style={{ textAlign: 'center', maxWidth: 680 }}>
        Open source, open schema, no lock-in.
      </Text>
    </FlexBox>
    <Notes>Quick-start slide showing three install commands and key project details. Purely technical with GitHub, language stats, and test count.</Notes>
  </Slide>,
];
