import { Slide, Heading, Text, FlexBox, Notes } from 'spectacle';
import type { JSX } from 'react';
import { colors } from '../theme/colors';
import { FullBleedCode } from '../components/layout/FullBleedCode';
import {
  specBehaviorExample,
  specInvariantExample,
  specEventExample,
  specPortExample,
  specTypeExample,
} from '../data/code-snippets';

export const coreConceptSlides: JSX.Element[] = [
  // ─── Slide 11: @specforge/software — 5 Entity Kinds Overview ───────────
  <Slide key="entity-graph-model" backgroundColor={colors.bg.primary}>
    <FlexBox flexDirection="column" alignItems="center" justifyContent="flex-start" height="100%" style={{ paddingTop: 24 }}>
      <Text fontSize="15px" color={colors.accent.teal} margin="0 0 8px 0" style={{ textTransform: 'uppercase', letterSpacing: '0.15em', fontWeight: 600 }}>@specforge/software &middot; 5 Entity Kinds</Text>
      <Heading fontSize="44px" color={colors.text.primary} margin="0 0 14px 0" style={{ textAlign: 'center' }}>
        The Software Engineering Extension
      </Heading>
      <Text fontSize="19px" color={colors.text.secondary} margin="0 0 36px 0" style={{ textAlign: 'center', maxWidth: 780, lineHeight: 1.6 }}>
        Each entity has a <span style={{ fontWeight: 700 }}>kind</span>, an <span style={{ fontWeight: 700 }}>id</span>, typed fields, and edges. The compiler validates every cross-reference.
      </Text>

      <div style={{ display: 'flex', gap: 18, maxWidth: 900, width: '100%', marginBottom: 28 }}>
        {[
          { kind: 'behavior', icon: '{ }', desc: 'What the system DOES. The central graph node — connects to invariants, ports, events, and types.', fields: 'invariants, ports, produces, contract, category', diagnostic: 'Testable, supports verify', color: colors.accent.teal },
          { kind: 'invariant', icon: '!', desc: 'What MUST always be true. A constraint with severity level.', fields: 'guarantee, risk', diagnostic: 'Unreferenced → W003', color: colors.accent.red },
          { kind: 'event', icon: '⚡', desc: 'What HAPPENED. A domain occurrence with a typed payload.', fields: 'channel, category, payload, contract', diagnostic: 'Orphan → W007', color: colors.accent.yellow },
          { kind: 'port', icon: '⇌', desc: 'What it DEPENDS ON. Hexagonal architecture interface boundary.', fields: 'direction, method declarations', diagnostic: 'Unused → W005', color: colors.accent.purple },
          { kind: 'type', icon: 'T', desc: 'What SHAPE data has. A value object with typed fields.', fields: 'fields { name type }', diagnostic: '@readonly, @unique, @optional', color: colors.accent.blue },
        ].map((item) => (
          <div key={item.kind} style={{ flex: 1, padding: '16px 14px', background: colors.bg.card, borderTop: `3px solid ${item.color}`, border: '1px solid rgba(255,255,255,0.06)', borderTopWidth: 3, borderTopStyle: 'solid', borderTopColor: item.color, borderRadius: 10, display: 'flex', flexDirection: 'column', gap: 8, minWidth: 0 }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
              <div style={{ width: 30, height: 30, borderRadius: 6, background: `${item.color}20`, display: 'flex', alignItems: 'center', justifyContent: 'center', fontSize: 14, fontWeight: 800, color: item.color, fontFamily: '"Inter", sans-serif', flexShrink: 0 }}>{item.icon}</div>
              <Text fontSize="18px" color={item.color} margin="0" fontWeight="bold" style={{ fontFamily: '"JetBrains Mono", monospace' }}>{item.kind}</Text>
            </div>
            <Text fontSize="15px" color={colors.text.secondary} margin="0" style={{ lineHeight: 1.5 }}>{item.desc}</Text>
            <div style={{ borderTop: '1px solid rgba(255,255,255,0.06)', paddingTop: 6 }}>
              <Text fontSize="14px" color={colors.text.muted} margin="0" style={{ fontFamily: '"JetBrains Mono", monospace' }}>{item.fields}</Text>
              <Text fontSize="14px" color={item.color} margin="3px 0 0 0" fontWeight="bold">{item.diagnostic}</Text>
            </div>
          </div>
        ))}
      </div>

      <div style={{ padding: '14px 24px', background: `${colors.accent.teal}06`, borderLeft: `4px solid ${colors.accent.teal}`, borderRadius: '0 10px 10px 0', maxWidth: 900, width: '100%' }}>
        <Text fontSize="17px" color={colors.accent.teal} margin="0" fontWeight="bold">
          behavior is the central hub — every other entity connects through it. 12 edge types, all compiler-validated.
        </Text>
      </div>
    </FlexBox>
    <Notes>5 entity kinds from @specforge/software. behavior is the central hub — it connects to invariants (constraints), ports (dependencies), events (emissions), and types (data shapes). 12 edge types, all compiler-validated.</Notes>
  </Slide>,

  // ─── Slide 12: Edge Types — How Entities Connect ──────────────────────
  <Slide key="edge-types" backgroundColor={colors.bg.primary}>
    <FlexBox flexDirection="column" alignItems="center" justifyContent="flex-start" height="100%" style={{ paddingTop: 28 }}>
      <Heading fontSize="44px" color={colors.text.primary} margin="0 0 14px 0" style={{ textAlign: 'center' }}>
        12 Edge Types — How Entities Connect
      </Heading>
      <Text fontSize="19px" color={colors.text.secondary} margin="0 0 36px 0" style={{ textAlign: 'center', maxWidth: 780, lineHeight: 1.6 }}>
        Every reference in a .spec file becomes a typed, compiler-validated edge in the graph.
      </Text>

      <div style={{ maxWidth: 860, width: '100%', borderRadius: 12, overflow: 'hidden', border: '1px solid rgba(255,255,255,0.08)' }}>
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1.3fr', padding: '12px 24px', background: 'rgba(255,255,255,0.05)' }}>
          {['Entity Relationship', 'Edge Type', 'What It Means'].map((h) => (
            <div key={h} style={{ fontSize: 14, fontWeight: 700, color: colors.text.muted, textTransform: 'uppercase', letterSpacing: '0.06em' }}>{h}</div>
          ))}
        </div>
        {[
          { from: 'behavior → invariant', edge: 'BehaviorEnforcesInvariant', meaning: 'This behavior must satisfy this constraint', color: colors.accent.red },
          { from: 'behavior → port', edge: 'BehaviorUsesPort', meaning: 'This behavior depends on this interface', color: colors.accent.purple },
          { from: 'behavior → event', edge: 'BehaviorProducesEvent', meaning: 'This behavior emits this event', color: colors.accent.yellow },
          { from: 'behavior → type', edge: 'BehaviorReferencesType', meaning: 'This behavior uses this data shape', color: colors.accent.blue },
          { from: 'event → type', edge: 'EventCarriesPayloadType', meaning: 'This event carries data in this typed shape', color: colors.accent.blue },
          { from: 'behavior → feature', edge: 'BehaviorImplementsFeature', meaning: 'This behavior implements this feature (cross-extension)', color: colors.accent.green },
        ].map((row, i) => (
          <div key={row.edge} style={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1.3fr', padding: '12px 24px', background: i % 2 === 0 ? 'rgba(255,255,255,0.02)' : 'transparent', borderTop: '1px solid rgba(255,255,255,0.04)' }}>
            <Text fontSize="16px" color={colors.text.primary} margin="0" style={{ fontFamily: '"JetBrains Mono", monospace' }}>{row.from}</Text>
            <Text fontSize="16px" color={row.color} margin="0" fontWeight="bold" style={{ fontFamily: '"JetBrains Mono", monospace' }}>{row.edge}</Text>
            <Text fontSize="16px" color={colors.text.secondary} margin="0">{row.meaning}</Text>
          </div>
        ))}
      </div>

      <div style={{ display: 'flex', gap: 16, maxWidth: 860, width: '100%', marginTop: 28 }}>
        <div style={{ flex: 1, padding: '14px 20px', background: `${colors.accent.teal}06`, borderLeft: `4px solid ${colors.accent.teal}`, borderRadius: '0 10px 10px 0' }}>
          <Text fontSize="16px" color={colors.accent.teal} margin="0" fontWeight="bold">Agent reads one behavior → gets the full neighborhood: constraints, dependencies, events, data shapes.</Text>
        </div>
        <div style={{ flex: 1, padding: '14px 20px', background: `${colors.accent.green}06`, borderLeft: `4px solid ${colors.accent.green}`, borderRadius: '0 10px 10px 0' }}>
          <Text fontSize="16px" color={colors.accent.green} margin="0" fontWeight="bold">Every edge is validated. Dangling references → E001. Missing connections → W003/W005/W007.</Text>
        </div>
      </div>
    </FlexBox>
    <Notes>Every reference in a .spec file becomes a typed edge. Behaviors connect to invariants, ports, events, types, and features. The agent queries one behavior and gets the full neighborhood. No guessing, no parsing.</Notes>
  </Slide>,

  // ─── Slide 12: Behavior — The Central Entity ──────────────────────────
  <FullBleedCode
    key="concept-behavior"
    filename="spec/auth/rate_limited_auth.spec"
    language="hcl"
    badge="behavior"
    badgeColor={colors.accent.teal}
    code={specBehaviorExample}
    takeaway={'A behavior is the central node. It declares WHAT the system does (contract), WHAT constraints it must satisfy (invariants), WHAT it depends on (ports), WHAT it emits (produces), and HOW to verify it (verify). An agent reading this graph knows every dependency and constraint.'}
    notes={<Notes>Walk through every field. invariants = constraints. ports = dependencies. produces = events. requires/ensures = pre/postconditions from @specforge/formal. contract = natural language intent in the graph. verify = test declarations.</Notes>}
  />,

  // ─── Slide 13: Invariant — Constraints That Must Hold ─────────────────
  <FullBleedCode
    key="concept-invariant"
    filename="spec/auth/invariants.spec"
    language="hcl"
    badge="invariant"
    badgeColor={colors.accent.red}
    code={specInvariantExample}
    takeaway={'An invariant declares a constraint that MUST always hold. Severity levels: critical, high, medium, low. If no behavior references an invariant → W003 (unreferenced). If the same behavior references contradictory invariants (15m token vs 24h session), the compiler catches the conflict.'}
    notes={<Notes>This is how SpecForge catches the JWT expiry bug from the .cursorrules slide. "auth_token_expiry" says 15m. W003 catches invariants no behavior enforces. Contradictory invariants on the same behavior are also detected.</Notes>}
  />,

  // ─── Slide 14: Event — What Happened ──────────────────────────────────
  <FullBleedCode
    key="concept-event"
    filename="spec/domain/events.spec"
    language="hcl"
    badge="event"
    badgeColor={colors.accent.yellow}
    code={specEventExample}
    takeaway={'An event declares a domain occurrence with a typed payload, channel, and delivery contract. Behaviors declare "produces [event_name]" — the compiler tracks the chain. If no behavior produces an event → W007 (orphan). sync blocks (from @specforge/formal) define ordering contracts.'}
    notes={<Notes>Events have typed payloads, channels, and delivery contracts. Behaviors declare "produces [event_name]" so the compiler tracks the full chain. sync blocks (from @specforge/formal) define event ordering. W007 catches orphan events no behavior produces.</Notes>}
  />,

  // ─── Slide 15: Port — Interface Boundaries ────────────────────────────
  <FullBleedCode
    key="concept-port"
    filename="spec/infra/ports.spec"
    language="hcl"
    badge="port"
    badgeColor={colors.accent.purple}
    code={specPortExample}
    takeaway={'A port declares an external interface with typed method signatures. Behaviors reference ports to declare dependencies. If a port exists but no behavior uses it → W005 (unreferenced port). Ports support method-level pre/postconditions from @specforge/formal. The agent knows exactly what to inject or mock.'}
    notes={<Notes>Ports map to hexagonal architecture. The agent sees: "rate_limited_auth depends on UserStore, TokenService, and RateLimiter — these are the interfaces I must provide." Method signatures give the exact type contracts.</Notes>}
  />,

  // ─── Slide 16: Type — Data Shapes ─────────────────────────────────────
  <FullBleedCode
    key="concept-type"
    filename="spec/domain/types.spec"
    language="hcl"
    badge="type"
    badgeColor={colors.accent.blue}
    code={specTypeExample}
    takeaway={'A type declares a domain value object with typed fields. Field types: string, integer, decimal, uuid, datetime, boolean, and references to other types (e.g. OrderItem[]). Annotations: @readonly, @unique, @optional. The agent knows the exact data shape — no hallucinating fields.'}
    notes={<Notes>Types are the data dictionary. An agent generating code knows OrderItem has product_id (uuid), quantity (integer), unit_price (decimal). No more guessing. The ? suffix means optional.</Notes>}
  />,

  // ─── Slide 17: How Entities Form a Graph ──────────────────────────────
  <Slide key="entity-connections" backgroundColor={colors.bg.primary}>
    <FlexBox flexDirection="column" alignItems="center" justifyContent="flex-start" height="100%" style={{ paddingTop: 16 }}>
      <Heading fontSize="40px" color={colors.text.primary} margin="0 0 12px 0" style={{ textAlign: 'center' }}>
        How Entities Form a Graph
      </Heading>
      <Text fontSize="17px" color={colors.text.secondary} margin="0 0 24px 0" style={{ textAlign: 'center', maxWidth: 780, lineHeight: 1.6 }}>
        A single behavior creates 7+ typed edges. The compiler validates every connection.
        The agent sees the full neighborhood, not isolated facts.
      </Text>

      <div style={{ maxWidth: 900, width: '100%', background: '#0a0a12', borderRadius: 14, border: '1px solid rgba(255,255,255,0.08)', padding: '20px 28px', fontFamily: '"JetBrains Mono", monospace', fontSize: 14, lineHeight: 2, whiteSpace: 'pre' }}>
        <div style={{ color: colors.text.muted, marginBottom: 8 }}>// Entity graph for rate_limited_auth (8 nodes, 7 edges)</div>
        <div>&nbsp;</div>
        <div>
          <span style={{ color: colors.accent.red }}>invariant</span> auth_token_expiry
          <span style={{ color: colors.text.muted }}> &larr;&mdash; BehaviorInvariant &mdash;&rarr; </span>
          <span style={{ color: colors.accent.teal }}>behavior</span> rate_limited_auth
          <span style={{ color: colors.text.muted }}> &mdash;&rarr; Produces &mdash;&rarr; </span>
          <span style={{ color: colors.accent.yellow }}>event</span> auth_succeeded
        </div>
        <div>
          <span style={{ color: colors.accent.red }}>invariant</span> rate_limit_per_ip
          <span style={{ color: colors.text.muted }}> &larr;&mdash; BehaviorInvariant &mdash;&rsaquo;</span>
        </div>
        <div>&nbsp;</div>
        <div>
          <span style={{ color: colors.accent.purple }}>port</span> UserStore
          <span style={{ color: colors.text.muted }}>{'      '}&larr;&mdash; BehaviorPort &mdash;&rsaquo;</span>
        </div>
        <div>
          <span style={{ color: colors.accent.purple }}>port</span> TokenService
          <span style={{ color: colors.text.muted }}>{'   '}&larr;&mdash; BehaviorPort &mdash;&rsaquo;  </span>
          <span style={{ color: colors.accent.teal }}>behavior</span> rate_limited_auth
        </div>
        <div>
          <span style={{ color: colors.accent.purple }}>port</span> RateLimiter
          <span style={{ color: colors.text.muted }}>{'    '}&larr;&mdash; BehaviorPort &mdash;&rsaquo;</span>
        </div>
        <div>&nbsp;</div>
        <div>
          <span style={{ color: colors.accent.yellow }}>event</span> auth_failed
          <span style={{ color: colors.text.muted }}>{'    '}&larr;&mdash; Produces &mdash;&mdash;&mdash;&mdash;&mdash;&mdash; </span>
          <span style={{ color: colors.accent.teal }}>behavior</span> rate_limited_auth
        </div>
      </div>

      <div style={{ display: 'flex', gap: 16, maxWidth: 900, width: '100%', marginTop: 20 }}>
        {[
          { label: 'Graph neighborhood', detail: 'Query rate_limited_auth → 8 nodes, 7 edges returned. All constraints, dependencies, and event contracts in one response.', color: colors.accent.teal },
          { label: 'Token efficiency', detail: '2,400 tokens of validated JSON instead of 45,000 tokens of scattered source files. 19x compression with zero information loss.', color: colors.accent.green },
          { label: 'Deterministic', detail: 'Same spec → same graph → same JSON. No retrieval randomness, no chunk boundary issues, no lost-in-the-middle degradation.', color: colors.brand },
        ].map((item) => (
          <div key={item.label} style={{ flex: 1, padding: '12px 14px', background: `${item.color}08`, borderLeft: `3px solid ${item.color}`, borderRadius: '0 8px 8px 0' }}>
            <Text fontSize="14px" color={item.color} margin="0" fontWeight="bold">{item.label}</Text>
            <Text fontSize="14px" color={colors.text.secondary} margin="4px 0 0 0">{item.detail}</Text>
          </div>
        ))}
      </div>
    </FlexBox>
    <Notes>One behavior creates a rich neighborhood. The agent queries rate_limited_auth and gets 8 nodes and 7 edges — validated JSON at 19x token compression vs raw source. That's why the code comes out correct on first attempt.</Notes>
  </Slide>,
];
