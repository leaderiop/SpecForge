import { Slide, Heading, Text, FlexBox, Notes } from 'spectacle';
import type { JSX } from 'react';
import { colors } from '../theme/colors';
import { FullBleedCode } from '../components/layout/FullBleedCode';
import { specSyntaxBasics, specUseImports } from '../data/code-snippets';

export const threeLayerSlides: JSX.Element[] = [
  // ─── Slide 8: Architecture — Three Layers, One Graph ──────────────────
  <Slide key="three-layers" backgroundColor={colors.bg.primary}>
    <FlexBox flexDirection="column" alignItems="center" justifyContent="center" height="100%">
      <Heading fontSize="46px" color={colors.text.primary} margin="0 0 10px 0" style={{ textAlign: 'center' }}>
        Architecture: Three Layers, One Graph
      </Heading>
      <Text fontSize="19px" color={colors.text.secondary} margin="0 0 36px 0" style={{ textAlign: 'center', maxWidth: 840, lineHeight: 1.5 }}>
        SpecForge is three distinct layers working together. Understanding each layer separately is key to understanding the system.
      </Text>

      <div style={{ maxWidth: 920, width: '100%', display: 'flex', gap: 18, marginBottom: 26 }}>
        {[
          {
            layer: 'Layer 1',
            title: 'The DSL',
            subtitle: '.spec files',
            details: [
              'keyword name { fields } — one syntax pattern',
              'Extensions define which keywords exist',
              'Reference lists: compiler-resolved cross-refs',
              'use imports for cross-file references',
              'verify declarations: test intent, not code',
              'Triple-quoted strings for contracts',
            ],
            color: colors.accent.teal,
          },
          {
            layer: 'Layer 2',
            title: 'The Compiler',
            subtitle: 'Parse → Resolve → Validate → Export',
            details: [
              'Written in Rust, 4-stage pipeline',
              'Tree-sitter: incremental, error recovery',
              'petgraph: typed directed entity graph',
              'lasso: string interning for O(1) ID comparison',
              '51 diagnostic rules, ALL from extensions',
              'Sub-100ms incremental recompilation',
            ],
            color: colors.brand,
          },
          {
            layer: 'Layer 3',
            title: 'The Graph Protocol',
            subtitle: 'Standardized JSON — the product',
            details: [
              'Typed entities: each has a kind + fields',
              'Typed edges: source, target, label',
              'Deterministic ordering: same spec → same JSON',
              '--format=graph | context | brief',
              '--token-budget=N controls output size',
              '--scope=id extracts subgraphs',
            ],
            color: colors.accent.green,
          },
        ].map((item) => (
          <div key={item.layer} style={{ flex: 1, display: 'flex', flexDirection: 'column', gap: 10 }}>
            <div style={{ padding: '18px 16px', background: `${item.color}07`, border: `1px solid ${item.color}25`, borderTop: `4px solid ${item.color}`, borderRadius: '0 0 12px 12px', flex: 1, display: 'flex', flexDirection: 'column' }}>
              <Text fontSize="14px" color={item.color} margin="0 0 2px 0" fontWeight="bold" style={{ textTransform: 'uppercase', letterSpacing: '0.1em' }}>{item.layer}</Text>
              <Text fontSize="22px" color={colors.text.primary} margin="0 0 2px 0" fontWeight="bold">{item.title}</Text>
              <Text fontSize="14px" color={colors.text.muted} margin="0 0 16px 0" style={{ fontFamily: '"JetBrains Mono", monospace' }}>{item.subtitle}</Text>
              <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
                {item.details.map((d) => (
                  <div key={d} style={{ display: 'flex', alignItems: 'flex-start', gap: 8 }}>
                    <div style={{ width: 5, height: 5, borderRadius: '50%', background: item.color, flexShrink: 0, marginTop: 7 }} />
                    <Text fontSize="14px" color={colors.text.secondary} margin="0" style={{ lineHeight: 1.5 }}>{d}</Text>
                  </div>
                ))}
              </div>
            </div>
          </div>
        ))}
      </div>

      {/* Flow Diagram */}
      <div style={{ maxWidth: 920, width: '100%', display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 0, flexWrap: 'wrap' }}>
        {[
          { label: '.spec files', color: colors.accent.teal },
          { label: '→', color: colors.text.muted },
          { label: 'Compiler', color: colors.brand },
          { label: '→', color: colors.text.muted },
          { label: 'Graph Protocol (JSON)', color: colors.accent.green },
          { label: '→', color: colors.text.muted },
          { label: 'Any AI Agent', color: colors.accent.purple },
        ].map((item, i) => (
          <div key={i} style={{ padding: item.label === '→' ? '0 10px' : '9px 18px', background: item.label === '→' ? 'transparent' : `${item.color}10`, border: item.label === '→' ? 'none' : `1px solid ${item.color}35`, borderRadius: item.label === '→' ? 0 : 9 }}>
            <Text fontSize={item.label === '→' ? '26px' : '16px'} color={item.color} margin="0" fontWeight="bold" style={{ fontFamily: item.label === '→' ? undefined : '"JetBrains Mono", monospace' }}>{item.label}</Text>
          </div>
        ))}
      </div>
    </FlexBox>
    <Notes>Three layers. The DSL is the user interface — how humans write specs. The compiler is the quality bottleneck — it parses, resolves, validates, exports. The Graph Protocol is the standard — the actual product. If someone builds a better compiler producing this JSON, SpecForge succeeds.</Notes>
  </Slide>,

  // ─── Slide 9: The DSL Syntax ──────────────────────────────────────────
  <FullBleedCode
    key="dsl-syntax"
    filename="The .spec DSL — one syntax, any domain"
    language="hcl"
    badge="syntax"
    badgeColor={colors.accent.teal}
    code={specSyntaxBasics}
    takeaway={'Every .spec file follows the same structure: keyword name { fields }. Extensions define which keywords exist. Reference lists are compiler-resolved — typos are errors, not silent bugs. verify declarations are test intent, not test code. Triple-quoted strings preserve whitespace for contracts.'}
    notes={<Notes>This slide teaches how to READ the DSL. Every block has the same structure. Reference lists are typed and compiler-validated. Verify declarations are test intent, not test code. Triple-quoted strings preserve whitespace for multi-line contracts. Extensions define which keywords are legal — the parser accepts ANY keyword.</Notes>}
  />,

  // ─── Slide 10: Imports and Cross-File References ──────────────────────
  <FullBleedCode
    key="dsl-imports"
    filename="Cross-file references — compiler-validated"
    language="hcl"
    badge="imports"
    badgeColor={colors.brand}
    code={specUseImports}
    takeaway={'use imports bring symbols into scope. Every reference is compiler-resolved — if an entity doesn\'t exist, you get error E001 with a "did you mean?" suggestion. No silent broken links. The compiler catches typos, missing imports, and undefined references — all with file:line:col.'}
    notes={<Notes>Imports are how specs compose across files. Every name in a reference list must resolve to a real entity. The compiler catches typos, missing imports, and undefined references — all with file:line:col error messages. use imports are explicit dependency declarations.</Notes>}
  />,
];
