import { Slide, Heading, Text, FlexBox, Notes } from 'spectacle';
import type { JSX } from 'react';
import { colors } from '../theme/colors';

export const integrationSlides: JSX.Element[] = [
  // ─── Slide 34: Integration Paths ─────────────────────────────────────
  <Slide key="integration-paths" backgroundColor={colors.bg.primary}>
    <FlexBox flexDirection="column" alignItems="center" justifyContent="flex-start" height="100%" style={{ paddingTop: 20 }}>
      <Heading fontSize="42px" color={colors.text.primary} margin="0 0 28px 0" style={{ textAlign: 'center' }}>
        Integration: MCP, CLI, LSP
      </Heading>

      <div style={{ display: 'flex', gap: 18, maxWidth: 880, width: '100%', marginBottom: 24 }}>
        {[
          {
            title: 'MCP Server',
            desc: 'JSON-RPC protocol. 24 tools (query graph, validate, export, trace), 7 resources (entity schemas, extension metadata), 5 prompts (guided spec authoring). Extensions auto-contribute additional tools via cmd__{id} Wasm exports. Works with Claude Code, Cursor, Windsurf, any MCP client.',
            detail: 'specforge mcp serve',
            color: colors.accent.teal
          },
          {
            title: 'CLI Export',
            desc: 'Pipe-based workflow. --scope=entity_id for subgraphs. --token-budget=N for context window control. --format=graph|context|brief for resolution levels. Integrates with any agent via stdin/stdout.',
            detail: 'specforge export --format=context',
            color: colors.accent.green
          },
          {
            title: 'LSP Server',
            desc: 'Real-time diagnostics, hover info (entity kind + fields + edge count), go-to-definition, completions (entity names, field names, reference targets), find-all-references. Sub-100ms incremental recompilation. Two binaries: specforge-cli and specforge-lsp.',
            detail: 'specforge lsp',
            color: colors.accent.purple
          },
        ].map((item) => (
          <div key={item.title} style={{ flex: 1, padding: '18px 16px', background: colors.bg.card, border: '1px solid rgba(255,255,255,0.06)', borderTop: `4px solid ${item.color}`, borderRadius: '0 0 12px 12px', display: 'flex', flexDirection: 'column', gap: 10 }}>
            <Text fontSize="20px" color={item.color} margin="0" fontWeight="bold">{item.title}</Text>
            <Text fontSize="14px" color={colors.text.secondary} margin="0" style={{ lineHeight: 1.5, flex: 1 }}>{item.desc}</Text>
            <div style={{ padding: '7px 12px', background: '#0a0a12', borderRadius: 8, fontFamily: '"JetBrains Mono", monospace', fontSize: 14, color: colors.accent.teal }}>
              $ {item.detail}
            </div>
          </div>
        ))}
      </div>

      <div style={{ display: 'flex', gap: 20, maxWidth: 880, width: '100%', marginBottom: 18 }}>
        <div style={{ flex: 1, padding: '14px 20px', background: `${colors.accent.purple}06`, borderLeft: `3px solid ${colors.accent.purple}`, borderRadius: '0 10px 10px 0' }}>
          <Text fontSize="16px" color={colors.accent.purple} margin="0" fontWeight="bold">MCP = Transport</Text>
          <Text fontSize="14px" color={colors.text.secondary} margin="6px 0 0 0">How agents connect to tools (JSON-RPC)</Text>
        </div>
        <div style={{ flex: 1, padding: '14px 20px', background: `${colors.accent.teal}06`, borderLeft: `3px solid ${colors.accent.teal}`, borderRadius: '0 10px 10px 0' }}>
          <Text fontSize="16px" color={colors.accent.teal} margin="0" fontWeight="bold">SpecForge = Content</Text>
          <Text fontSize="14px" color={colors.text.secondary} margin="6px 0 0 0">What agents receive (validated typed entity graph)</Text>
        </div>
      </div>

      <Text fontSize="17px" color={colors.text.muted} margin="0" style={{ textAlign: 'center', maxWidth: 700 }}>
        HTTP didn't make HTML unnecessary. MCP doesn't make structured content unnecessary.
      </Text>
    </FlexBox>
    <Notes>Three integration paths covering MCP server for agent tools, CLI for pipe-based workflows, and LSP for editor integration. The distinction matters: MCP is transport, SpecForge is content.</Notes>
  </Slide>,

  // ─── Slide 35: Agent Workflow ─────────────────────────────────────────
  <Slide key="agent-workflow" backgroundColor={colors.bg.primary}>
    <FlexBox flexDirection="column" alignItems="center" justifyContent="center" height="100%">
      <Heading fontSize="42px" color={colors.text.primary} margin="0 0 32px 0" style={{ textAlign: 'center' }}>
        How an Agent Uses SpecForge
      </Heading>

      <div style={{ maxWidth: 880, width: '100%', display: 'flex', flexDirection: 'column', gap: 12 }}>
        {[
          {
            step: '1',
            actor: 'Developer',
            action: '"Implement the rate_limited_auth behavior"',
            detail: 'Natural language prompt',
            color: colors.accent.blue
          },
          {
            step: '2',
            actor: 'Agent',
            action: 'Calls specforge export --scope=rate_limited_auth via MCP tool',
            detail: 'MCP tool call',
            color: colors.accent.teal
          },
          {
            step: '3',
            actor: 'SpecForge',
            action: 'Returns 2,400 tokens — 8 entities, 7 edges, all typed',
            detail: 'behavior + 2 invariants + 3 ports + 2 events',
            color: colors.accent.green
          },
          {
            step: '4',
            actor: 'Agent',
            action: 'Generates code following every graph edge',
            detail: 'Rate limiter first (port), 15m expiry (invariant), auth_failed emitted (event)',
            color: colors.accent.purple
          },
          {
            step: '5',
            actor: 'Developer',
            action: 'Reviews code that is domain-correct on first attempt',
            detail: 'All constraints, ports, and events present',
            color: colors.brand
          },
        ].map((item) => (
          <div key={item.step} style={{ display: 'flex', gap: 16, padding: '14px 18px', background: `${item.color}06`, borderLeft: `4px solid ${item.color}`, borderRadius: '0 10px 10px 0' }}>
            <div style={{ width: 32, height: 32, borderRadius: '50%', background: `${item.color}20`, display: 'flex', alignItems: 'center', justifyContent: 'center', fontSize: 16, fontWeight: 800, color: item.color, flexShrink: 0 }}>{item.step}</div>
            <div style={{ flex: 1 }}>
              <div style={{ display: 'flex', gap: 10, alignItems: 'baseline', marginBottom: 2 }}>
                <Text fontSize="14px" color={item.color} margin="0" fontWeight="bold">{item.actor}</Text>
                <Text fontSize="15px" color={colors.text.primary} margin="0" fontWeight="bold">{item.action}</Text>
              </div>
              <Text fontSize="14px" color={colors.text.muted} margin="0">{item.detail}</Text>
            </div>
          </div>
        ))}
      </div>
    </FlexBox>
    <Notes>Five-step workflow showing agent integration. Developer prompts, agent queries via MCP, SpecForge returns typed graph, agent generates domain-correct code, developer reviews.</Notes>
  </Slide>,
];
