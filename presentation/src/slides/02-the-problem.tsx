import { Slide, Heading, Text, FlexBox, Notes } from 'spectacle';
import React from 'react';
import type { JSX } from 'react';
import { colors } from '../theme/colors';
import { FullBleedCode } from '../components/layout/FullBleedCode';
import { cursorRulesExample } from '../data/code-snippets';

export const theProblemSlides: JSX.Element[] = [
  // ─── Slide 3: The Problem (Concrete) ──────────────────────────────────
  <Slide key="the-problem" backgroundColor={colors.bg.primary}>
    <FlexBox flexDirection="column" alignItems="center" justifyContent="center" height="100%">
      <Heading fontSize="46px" color={colors.text.primary} margin="0 0 48px 0" style={{ textAlign: 'center', maxWidth: 900 }}>
        AI Agents Can Write Code.<br />
        <span style={{ color: colors.accent.red }}>They Can't Know Your Domain.</span>
      </Heading>

      <div style={{ maxWidth: 920, width: '100%', display: 'flex', flexDirection: 'column', gap: 20, marginBottom: 36 }}>
        {[
          {
            title: 'Missing constraints',
            desc: 'Agent writes auth with 24h token expiry because it found SESSION_TTL=24h in env, not knowing the invariant is 15 minutes',
            color: colors.accent.red
          },
          {
            title: 'Missing dependencies',
            desc: 'Agent generates a service without the rate limiter because no interface boundary was declared',
            color: colors.accent.orange
          },
          {
            title: 'Missing event contracts',
            desc: 'Agent skips emitting auth_failed because no producer-consumer chain was visible',
            color: colors.accent.yellow
          },
        ].map((item, idx) => (
          <div key={item.title} style={{
            display: 'flex',
            alignItems: 'flex-start',
            gap: 18,
            padding: '20px 24px',
            background: `${item.color}06`,
            borderLeft: `4px solid ${item.color}`,
            borderRadius: '0 12px 12px 0'
          }}>
            <div style={{
              minWidth: 32,
              height: 32,
              borderRadius: '50%',
              background: item.color,
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              fontWeight: 800,
              fontSize: 18,
              color: colors.bg.primary,
              flexShrink: 0
            }}>
              {idx + 1}
            </div>
            <div style={{ flex: 1 }}>
              <Text fontSize="19px" color={item.color} margin="0 0 8px 0" fontWeight="bold">
                {item.title}
              </Text>
              <Text fontSize="16px" color={colors.text.secondary} margin="0" style={{ lineHeight: 1.6 }}>
                {item.desc}
              </Text>
            </div>
          </div>
        ))}
      </div>

      <div style={{ maxWidth: 920, width: '100%', padding: '18px 28px', background: 'rgba(255,255,255,0.03)', border: '1px solid rgba(255,255,255,0.08)', borderRadius: 12 }}>
        <Text fontSize="18px" color={colors.text.secondary} margin="0" style={{ textAlign: 'center', lineHeight: 1.7 }}>
          These aren't model failures. The agent wrote syntactically correct code.<br />
          <span style={{ color: colors.accent.red, fontWeight: 700 }}>The domain semantics were invisible.</span>
        </Text>
      </div>
    </FlexBox>
    <Notes>Three concrete failure modes when agents read source code instead of a spec graph. The code compiles and builds cleanly — the bugs are semantic, not syntactic. This is a structural problem, not a model capability problem.</Notes>
  </Slide>,

  // ─── Slide 4: What Context Looks Like Today ────────────────────────────
  <FullBleedCode
    key="cursorrules-code"
    filename=".cursorrules  ·  847 lines  ·  manually maintained  ·  contradictions marked"
    language="yaml"
    badge="unvalidated prose"
    badgeColor={colors.accent.red}
    code={cursorRulesExample}
    takeaway="No validation. No graph. No cross-references. The JWT expiry contradiction between Auth and Session Management is invisible to agents."
    notes={<Notes>This is how teams provide context today. It's the BEST current approach. The token expiry contradiction is the exact kind of bug SpecForge catches — we'll show this later.</Notes>}
  />,

  // ─── Slide 5: The Missing Infrastructure Layer ─────────────────────────
  <Slide key="missing-layer" backgroundColor={colors.bg.primary}>
    <FlexBox flexDirection="column" alignItems="center" justifyContent="center" height="100%">
      <Heading fontSize="44px" color={colors.text.primary} margin="0 0 36px 0" style={{ textAlign: 'center' }}>
        The Missing Infrastructure Layer
      </Heading>

      <div style={{ maxWidth: 880, width: '100%', marginBottom: 32 }}>
        <div style={{
          display: 'grid',
          gridTemplateColumns: '200px 120px 140px 120px 140px',
          gap: '0',
          background: 'rgba(255,255,255,0.02)',
          border: '1px solid rgba(255,255,255,0.08)',
          borderRadius: 12,
          overflow: 'hidden'
        }}>
          {/* Header row */}
          {['Approach', 'Validated', 'Cross-Referenced', 'Typed Edges', 'Token-Efficient'].map((header, idx) => (
            <div key={header} style={{
              padding: '14px 16px',
              background: 'rgba(255,255,255,0.05)',
              borderRight: idx < 4 ? '1px solid rgba(255,255,255,0.06)' : 'none',
              borderBottom: '1px solid rgba(255,255,255,0.08)'
            }}>
              <Text fontSize="14px" color={colors.text.primary} margin="0" fontWeight="bold" style={{ textAlign: 'center' }}>
                {header}
              </Text>
            </div>
          ))}

          {/* Data rows */}
          {[
            { approach: '.cursorrules / CLAUDE.md', values: ['no', 'no', 'no', 'no'] },
            { approach: 'RAG pipelines', values: ['no', 'no', 'no', 'partial'] },
            { approach: 'OpenAPI / Protobuf', values: ['yes', 'no', 'partial', 'yes'] },
            { approach: 'SpecForge', values: ['yes', 'yes', 'yes', 'yes'], highlight: true },
          ].map((row) => (
            <React.Fragment key={row.approach}>
              <div style={{
                padding: '14px 16px',
                borderRight: '1px solid rgba(255,255,255,0.06)',
                borderBottom: '1px solid rgba(255,255,255,0.04)',
                background: row.highlight ? `${colors.accent.teal}08` : 'transparent'
              }}>
                <Text fontSize="15px" color={row.highlight ? colors.accent.teal : colors.text.secondary} margin="0" fontWeight={row.highlight ? 'bold' : 'normal'}>
                  {row.approach}
                </Text>
              </div>
              {row.values.map((value, idx) => (
                <div key={`${row.approach}-${idx}`} style={{
                  padding: '14px 16px',
                  borderRight: idx < 3 ? '1px solid rgba(255,255,255,0.06)' : 'none',
                  borderBottom: '1px solid rgba(255,255,255,0.04)',
                  background: row.highlight ? `${colors.accent.teal}08` : 'transparent',
                  textAlign: 'center'
                }}>
                  <Text
                    fontSize="14px"
                    color={value === 'yes' ? colors.accent.green : value === 'partial' ? colors.accent.yellow : colors.accent.red}
                    margin="0"
                    fontWeight="600"
                    style={{ fontFamily: '"JetBrains Mono", monospace' }}
                  >
                    {value}
                  </Text>
                </div>
              ))}
            </React.Fragment>
          ))}
        </div>
      </div>

      <div style={{ maxWidth: 880, width: '100%', padding: '20px 28px', background: `${colors.accent.teal}06`, borderLeft: `4px solid ${colors.accent.teal}`, borderRadius: '0 12px 12px 0' }}>
        <Text fontSize="18px" color={colors.text.secondary} margin="0" style={{ lineHeight: 1.7 }}>
          Every domain paradigm shift created a structured layer: <span style={{ color: colors.accent.teal, fontWeight: 700 }}>SQL</span> for databases,{' '}
          <span style={{ color: colors.accent.teal, fontWeight: 700 }}>Protobuf</span> for services,{' '}
          <span style={{ color: colors.accent.teal, fontWeight: 700 }}>HCL</span> for infrastructure,{' '}
          <span style={{ color: colors.accent.teal, fontWeight: 700 }}>OpenAPI</span> for APIs.{' '}
          <span style={{ color: colors.accent.teal, fontWeight: 700 }}>AI agents are next.</span>
        </Text>
      </div>
    </FlexBox>
    <Notes>Comparison table showing what's structurally missing in existing approaches. SpecForge is the first to satisfy all four properties. The paradigm shift framing positions this as inevitable infrastructure, not a novel experiment.</Notes>
  </Slide>,
];
