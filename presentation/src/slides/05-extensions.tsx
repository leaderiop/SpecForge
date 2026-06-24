import { Slide, Heading, Text, FlexBox, Notes } from 'spectacle';
import type { JSX } from 'react';
import { colors } from '../theme/colors';
import { FullBleedCode } from '../components/layout/FullBleedCode';
import {
  specFeatureJourneyExample,
  specPlanningExample,
  specGovernanceExample,
  specFormalExample,
  specContextEntitiesExample,
  specEntityEnhancementExample,
} from '../data/code-snippets';

export const extensionSlides: JSX.Element[] = [
  // ─── Slide 18: Extension Architecture — Zero Domain in Core ───────────
  <Slide key="ext-architecture" backgroundColor={colors.bg.primary}>
    <FlexBox flexDirection="column" alignItems="center" justifyContent="flex-start" height="100%" style={{ paddingTop: 18 }}>
      <Text fontSize="15px" color={colors.accent.purple} margin="0 0 8px 0" style={{ textTransform: 'uppercase', letterSpacing: '0.15em', fontWeight: 600 }}>Extension Architecture</Text>
      <Heading fontSize="42px" color={colors.text.primary} margin="0 0 8px 0" style={{ textAlign: 'center' }}>
        Zero Domain Knowledge in Core
      </Heading>
      <Text fontSize="18px" color={colors.text.secondary} margin="0 0 18px 0" style={{ maxWidth: 800, textAlign: 'center', lineHeight: 1.5 }}>
        The compiler core is a pure typed-graph engine. It parses any <span style={{ fontFamily: '"JetBrains Mono", monospace', color: colors.accent.teal }}>keyword name {'{ }'}</span> block generically.
        All domain vocabulary comes from <span style={{ color: colors.accent.purple, fontWeight: 700 }}>sandboxed Wasm extensions</span>.
      </Text>

      <div style={{ display: 'grid', gridTemplateColumns: '460px 380px', gap: 16, maxWidth: 880, width: '100%', marginBottom: 16 }}>
        {/* LEFT: Extension declares table */}
        <div style={{ borderRadius: 10, overflow: 'hidden', border: '1px solid rgba(255,255,255,0.08)' }}>
          <div style={{ display: 'grid', gridTemplateColumns: '160px 1fr', padding: '6px 16px', background: 'rgba(255,255,255,0.05)' }}>
            <div style={{ fontSize: 14, fontWeight: 700, color: colors.text.muted, textTransform: 'uppercase', letterSpacing: '0.06em' }}>Extension Declares</div>
            <div style={{ fontSize: 14, fontWeight: 700, color: colors.text.muted, textTransform: 'uppercase', letterSpacing: '0.06em' }}>What Compiler Does</div>
          </div>
          {[
            { declares: 'entity_kinds', does: 'Registers new DSL keywords', color: colors.accent.teal },
            { declares: 'edge_types', does: 'Typed relationships', color: colors.accent.green },
            { declares: 'validation_rules', does: 'Declarative patterns', color: colors.accent.red },
            { declares: 'entity_enhancements', does: 'Add fields to OTHER kinds', color: colors.accent.blue },
            { declares: 'verify_kinds', does: 'Custom verification types', color: colors.accent.purple },
          ].map((row, i) => (
            <div key={row.declares} style={{ display: 'grid', gridTemplateColumns: '160px 1fr', padding: '5px 16px', background: i % 2 === 0 ? 'rgba(255,255,255,0.02)' : 'transparent', borderTop: '1px solid rgba(255,255,255,0.04)' }}>
              <Text fontSize="14px" color={row.color} margin="0" fontWeight="bold" style={{ fontFamily: '"JetBrains Mono", monospace' }}>{row.declares}</Text>
              <Text fontSize="14px" color={colors.text.secondary} margin="0">{row.does}</Text>
            </div>
          ))}
        </div>

        {/* RIGHT: ManifestV2 JSON snippet */}
        <div style={{ background: '#0a0a12', borderRadius: 10, border: '1px solid rgba(255,255,255,0.08)', padding: '10px 14px', fontFamily: '"JetBrains Mono", monospace', fontSize: 14, lineHeight: 1.6, color: colors.text.muted, overflow: 'hidden' }}>
          <div style={{ color: colors.accent.purple, marginBottom: 6 }}>// ManifestV2 JSON</div>
          <div><span style={{ color: colors.text.muted }}>{'{'}</span></div>
          <div style={{ paddingLeft: 12 }}>
            <span style={{ color: colors.accent.teal }}>"entity_kinds"</span>: [
          </div>
          <div style={{ paddingLeft: 24 }}>
            {'{'} <span style={{ color: colors.accent.green }}>"name"</span>: <span style={{ color: colors.accent.yellow }}>"behavior"</span>,
          </div>
          <div style={{ paddingLeft: 32 }}>
            <span style={{ color: colors.accent.green }}>"fields"</span>: [<span style={{ color: colors.text.muted }}>...</span>],
          </div>
          <div style={{ paddingLeft: 32 }}>
            <span style={{ color: colors.accent.green }}>"testable"</span>: <span style={{ color: colors.accent.cyan }}>true</span> {'}'}
          </div>
          <div style={{ paddingLeft: 12 }}>],</div>
          <div style={{ paddingLeft: 12 }}>
            <span style={{ color: colors.accent.teal }}>"edge_types"</span>: [<span style={{ color: colors.text.muted }}>...</span>],
          </div>
          <div style={{ paddingLeft: 12 }}>
            <span style={{ color: colors.accent.teal }}>"validation_rules"</span>: [<span style={{ color: colors.text.muted }}>...</span>]
          </div>
          <div><span style={{ color: colors.text.muted }}>{'}'}</span></div>
        </div>
      </div>

      <div style={{ display: 'flex', gap: 16, maxWidth: 880, width: '100%' }}>
        <div style={{ flex: 1, padding: '12px 20px', borderLeft: `4px solid ${colors.brand}`, background: `${colors.brand}08`, borderRadius: '0 10px 10px 0' }}>
          <Text fontSize="16px" color={colors.brand} margin="0" fontWeight="bold">
            Like Terraform: zero infrastructure knowledge in core — providers supply it. SpecForge: zero domain knowledge — extensions supply it.
          </Text>
        </div>
        <div style={{ flex: 1, padding: '12px 20px', borderLeft: `4px solid ${colors.accent.purple}`, background: `${colors.accent.purple}08`, borderRadius: '0 10px 10px 0' }}>
          <Text fontSize="16px" color={colors.accent.purple} margin="0" fontWeight="bold">
            Wasm (Extism runtime): sandboxed, AOT-cached, language-agnostic. Any team can write @specforge/shipping or @specforge/healthcare.
          </Text>
        </div>
      </div>
    </FlexBox>
    <Notes>ManifestV2 has 18 contribution categories total. entity_enhancements is key — it's how @specforge/formal adds requires/ensures to behaviors without modifying @specforge/software.</Notes>
  </Slide>,

  // ─── Slide 19: Entity Enhancement — Cross-Extension Composition ───────
  <FullBleedCode
    key="entity-enhancements"
    filename="@specforge/formal manifest.json + behavior usage"
    language="javascript"
    badge="entity_enhancements"
    badgeColor={colors.accent.blue}
    code={specEntityEnhancementExample}
    takeaway={'Extensions compose without modifying each other. @specforge/formal adds structured conditions to @specforge/software entities via entity_enhancements. The compiler merges them at compile time.'}
    notes={<Notes>Entity enhancements are the key to cross-extension composition. @specforge/formal adds requires/ensures/maintains to behaviors, sync blocks to events, method-level conditions to ports — all without touching @specforge/software.</Notes>}
  />,

  // ─── Slide 20: @specforge/product — Planning Entities ──────────────────
  <Slide key="ext-product-planning" backgroundColor={colors.bg.primary}>
    <FlexBox flexDirection="column" alignItems="center" justifyContent="flex-start" height="100%" style={{ paddingTop: 24 }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginBottom: 16 }}>
        <div style={{ padding: '4px 12px', background: `${colors.accent.blue}15`, border: `1px solid ${colors.accent.blue}40`, borderRadius: 999, fontFamily: '"JetBrains Mono", monospace', fontSize: 16, fontWeight: 700, color: colors.accent.blue }}>@specforge/product</div>
        <Text fontSize="16px" color={colors.text.muted} margin="0">9 entity kinds &middot; 20 edge types &middot; 27 diagnostics</Text>
      </div>
      <Heading fontSize="40px" color={colors.text.primary} margin="0 0 14px 0" style={{ textAlign: 'center' }}>
        Product Management — Planning Entities
      </Heading>
      <Text fontSize="18px" color={colors.text.secondary} margin="0 0 32px 0" style={{ maxWidth: 800, textAlign: 'center', lineHeight: 1.5 }}>
        Models the full product lifecycle. Status transitions are compiler-validated (W087-W094).
      </Text>

      <div style={{ display: 'flex', gap: 18, maxWidth: 900, width: '100%', marginBottom: 28 }}>
        {[
          { kind: 'feature', desc: 'Problem/solution pair. Status: draft → in_progress → done → deprecated (W087). Priority + Fibonacci effort sizing (xs/s/m/l/xl).', color: colors.accent.blue },
          { kind: 'milestone', desc: 'Planning phase with target date and exit criteria. Milestone DAGs validated — cycles → E007. Depends_on for sequencing.', color: colors.accent.purple },
          { kind: 'deliverable', desc: 'Shippable artifact tying journeys to modules. Links planning to architecture: deliverable → module → feature.', color: colors.accent.teal },
          { kind: 'release', desc: 'Versioned shipment bundling deliverables + milestones. Semver validated (W093). Status transitions → W094.', color: colors.accent.red },
        ].map((item) => (
          <div key={item.kind} style={{ flex: 1, padding: '16px 14px', background: colors.bg.card, borderTop: `3px solid ${item.color}`, border: '1px solid rgba(255,255,255,0.06)', borderTopWidth: 3, borderTopStyle: 'solid', borderTopColor: item.color, borderRadius: 10, display: 'flex', flexDirection: 'column', gap: 6, minWidth: 0 }}>
            <Text fontSize="18px" color={item.color} margin="0" fontWeight="bold" style={{ fontFamily: '"JetBrains Mono", monospace' }}>{item.kind}</Text>
            <Text fontSize="15px" color={colors.text.secondary} margin="0" style={{ lineHeight: 1.5 }}>{item.desc}</Text>
          </div>
        ))}
      </div>

      <div style={{ display: 'flex', gap: 18, maxWidth: 900, width: '100%' }}>
        {[
          { kind: 'persona', desc: 'User archetype: goals + pain points', color: colors.accent.yellow },
          { kind: 'journey', desc: 'User flow: persona + channel → features (orphan → W042)', color: colors.accent.green },
          { kind: 'channel', desc: 'Interaction surface: web, mobile, api, cli', color: colors.accent.cyan },
          { kind: 'module', desc: 'Structural component. Dep DAG (cycles → E007)', color: colors.accent.orange },
          { kind: 'term', desc: 'Ubiquitous language: definition + aliases', color: colors.text.muted },
        ].map((item) => (
          <div key={item.kind} style={{ flex: 1, padding: '12px 14px', background: `${item.color}06`, borderLeft: `3px solid ${item.color}`, borderRadius: '0 8px 8px 0', minWidth: 0 }}>
            <Text fontSize="16px" color={item.color} margin="0" fontWeight="bold" style={{ fontFamily: '"JetBrains Mono", monospace' }}>{item.kind}</Text>
            <Text fontSize="14px" color={colors.text.secondary} margin="4px 0 0 0" style={{ lineHeight: 1.4 }}>{item.desc}</Text>
          </div>
        ))}
      </div>
    </FlexBox>
    <Notes>@specforge/product has 9 entity kinds. Top row: the core planning entities (feature, milestone, deliverable, release). Bottom row: the context entities (persona, journey, channel, module, term). All status transitions are compiler-validated.</Notes>
  </Slide>,

  // ─── Slide 21: @specforge/product — Traceability Chain ────────────────
  <Slide key="ext-product-traceability" backgroundColor={colors.bg.primary}>
    <FlexBox flexDirection="column" alignItems="center" justifyContent="center" height="100%">
      <Heading fontSize="42px" color={colors.text.primary} margin="0 0 14px 0" style={{ textAlign: 'center' }}>
        Product Traceability Chain
      </Heading>
      <Text fontSize="19px" color={colors.text.secondary} margin="0 0 36px 0" style={{ textAlign: 'center', maxWidth: 780, lineHeight: 1.6 }}>
        20 edge types connect all 9 kinds. A PM agent traces from "who uses it" to "what code implements it."
      </Text>

      <div style={{ maxWidth: 860, width: '100%', background: '#0a0a12', borderRadius: 14, border: '1px solid rgba(255,255,255,0.08)', padding: '28px 32px', fontFamily: '"JetBrains Mono", monospace', fontSize: 18, lineHeight: 2.2, whiteSpace: 'pre' as const }}>
        <div style={{ color: colors.text.muted, marginBottom: 12, fontSize: 14 }}>// Two traceability paths through the product graph:</div>
        <div>
          <span style={{ color: colors.accent.yellow }}>persona</span>
          <span style={{ color: colors.text.muted }}> → </span>
          <span style={{ color: colors.accent.green }}>journey</span>
          <span style={{ color: colors.text.muted }}> → </span>
          <span style={{ color: colors.accent.blue }}>feature</span>
          <span style={{ color: colors.text.muted }}> → </span>
          <span style={{ color: colors.accent.teal }}>behavior</span>
          <span style={{ color: colors.text.muted }}> (cross-extension)</span>
        </div>
        <div>
          <span style={{ color: colors.accent.purple }}>milestone</span>
          <span style={{ color: colors.text.muted }}> → </span>
          <span style={{ color: colors.accent.blue }}>feature</span>
          <span style={{ color: colors.text.muted }}> → </span>
          <span style={{ color: colors.accent.teal }}>deliverable</span>
          <span style={{ color: colors.text.muted }}> → </span>
          <span style={{ color: colors.accent.red }}>release</span>
        </div>
      </div>

      <div style={{ display: 'flex', gap: 18, maxWidth: 860, width: '100%', marginTop: 28 }}>
        <div style={{ flex: 1, padding: '14px 20px', background: `${colors.accent.blue}06`, borderLeft: `4px solid ${colors.accent.blue}`, borderRadius: '0 10px 10px 0' }}>
          <Text fontSize="16px" color={colors.accent.blue} margin="0" fontWeight="bold">Cross-extension bridge</Text>
          <Text fontSize="15px" color={colors.text.secondary} margin="6px 0 0 0">feature → behavior crosses from @specforge/product into @specforge/software via peer_dependency.</Text>
        </div>
        <div style={{ flex: 1, padding: '14px 20px', background: `${colors.accent.green}06`, borderLeft: `4px solid ${colors.accent.green}`, borderRadius: '0 10px 10px 0' }}>
          <Text fontSize="16px" color={colors.accent.green} margin="0" fontWeight="bold">Full lifecycle coverage</Text>
          <Text fontSize="15px" color={colors.text.secondary} margin="6px 0 0 0">From user persona through journey, feature, behavior, all the way to deliverable and versioned release.</Text>
        </div>
      </div>
    </FlexBox>
    <Notes>Two traceability paths. User path: persona → journey → feature → behavior. Planning path: milestone → feature → deliverable → release. The cross-extension bridge is how product planning connects to engineering implementation.</Notes>
  </Slide>,

  // ─── Slide 21: Product DSL — Features + Journeys ──────────────────────
  <FullBleedCode
    key="product-features"
    filename="spec/product/features.spec"
    language="hcl"
    badge="feature + journey"
    badgeColor={colors.accent.blue}
    code={specFeatureJourneyExample}
    takeaway={'Features define WHAT (problem + solution). Journeys define WHO uses it (persona + channel → features). Status transitions (draft → in_progress → done → deprecated) are compiler-validated — W087 catches invalid transitions.'}
    notes={<Notes>Features are domain-neutral: problem/solution framing only. Status transitions are validated. Fibonacci effort sizing. Journeys connect WHO (persona, channel) to WHAT (features). Orphan detection: journey not in deliverable → W042.</Notes>}
  />,

  // ─── Slide 22: Product DSL — Milestones + Deliverables + Modules ──────
  <FullBleedCode
    key="product-planning"
    filename="spec/product/planning.spec"
    language="hcl"
    badge="milestone + module + deliverable"
    badgeColor={colors.accent.purple}
    code={specPlanningExample}
    takeaway={'Milestones = planning phases with exit criteria. Modules = structural components with dependency DAGs (cycles → E007). Deliverables = shippable artifacts. Terms = ubiquitous language. A PM agent reads this graph and writes accurate status reports.'}
    notes={<Notes>Milestones have exit criteria — agents can assess readiness. Module DAG is cycle-checked. Deliverables tie journeys to modules. Terms build a shared glossary. 20 edge types connect all 9 kinds.</Notes>}
  />,

  // ─── Slide 23: Product DSL — Persona + Channel + Release + Term ───────
  <FullBleedCode
    key="product-personas"
    filename="spec/product/personas.spec + channels.spec + releases.spec"
    language="hcl"
    badge="persona + channel + release"
    badgeColor={colors.accent.yellow}
    code={specContextEntitiesExample}
    takeaway={'Personas define WHO uses the system. Channels define WHERE they interact. Releases bundle deliverables into versioned shipments. All compiler-validated with status transitions and semver checking.'}
    notes={<Notes>Persona = user archetype with goals/pain points. Channel = interaction surface (web/mobile/api/cli). Release = versioned shipment bundling deliverables + milestones. Status transitions → W094, invalid semver → W093.</Notes>}
  />,

  // ─── Slide 24: @specforge/governance Overview ─────────────────────────
  <Slide key="ext-governance" backgroundColor={colors.bg.primary}>
    <FlexBox flexDirection="column" alignItems="center" justifyContent="flex-start" height="100%" style={{ paddingTop: 20 }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginBottom: 16 }}>
        <div style={{ padding: '4px 12px', background: `${colors.accent.purple}15`, border: `1px solid ${colors.accent.purple}40`, borderRadius: 999, fontFamily: '"JetBrains Mono", monospace', fontSize: 16, fontWeight: 700, color: colors.accent.purple }}>@specforge/governance</div>
        <Text fontSize="16px" color={colors.text.muted} margin="0">3 entity kinds &middot; 11 edge types</Text>
      </div>
      <Heading fontSize="36px" color={colors.text.primary} margin="0 0 12px 0" style={{ textAlign: 'center' }}>
        Architecture Governance
      </Heading>
      <Text fontSize="17px" color={colors.text.secondary} margin="0 0 20px 0" style={{ maxWidth: 800, textAlign: 'center', lineHeight: 1.5 }}>
        Three entity kinds that capture WHY decisions were made, WHAT constraints exist, and WHERE risks lie.
        Cross-extension: decisions protect invariants, constraints apply to behaviors.
      </Text>

      <div style={{ display: 'flex', gap: 18, maxWidth: 900, width: '100%' }}>
        {[
          {
            kind: 'decision',
            desc: 'Architecture Decision Record (ADR). Captures WHY a choice was made, not just WHAT. Status: proposed → accepted → deprecated → superseded. Cross-references invariants it protects.',
            example: 'decision use_event_sourcing { status accepted  context "Order state changes must be auditable..." }',
            color: colors.accent.purple,
          },
          {
            kind: 'constraint',
            desc: 'Non-functional requirement with measurable threshold. Categories: performance, security, reliability, compatibility. Priority: must, should, may (RFC 2119). TESTABLE via verify declarations.',
            example: 'constraint api_latency { category performance  metric "p99 < 200ms"  priority must }',
            color: colors.accent.blue,
          },
          {
            kind: 'failure_mode',
            desc: 'FMEA risk assessment tied to an invariant. Severity × Occurrence × Detection = RPN. post_mitigation field tracks risk reduction after mitigation.',
            example: 'failure_mode write_ack_lost { severity 8  occurrence 2  detection 3  rpn 48 }',
            color: colors.accent.red,
          },
        ].map((item) => (
          <div key={item.kind} style={{ flex: 1, padding: '16px 14px', background: colors.bg.card, borderTop: `3px solid ${item.color}`, border: '1px solid rgba(255,255,255,0.06)', borderTopWidth: 3, borderTopStyle: 'solid', borderTopColor: item.color, borderRadius: 10, display: 'flex', flexDirection: 'column', gap: 8 }}>
            <Text fontSize="18px" color={item.color} margin="0" fontWeight="bold" style={{ fontFamily: '"JetBrains Mono", monospace' }}>{item.kind}</Text>
            <Text fontSize="14px" color={colors.text.secondary} margin="0" style={{ lineHeight: 1.5, flex: 1 }}>{item.desc}</Text>
            <div style={{ padding: '8px 12px', background: '#0a0a12', borderRadius: 6, fontFamily: '"JetBrains Mono", monospace', fontSize: 14, color: colors.text.muted, lineHeight: 1.5 }}>
              {item.example}
            </div>
          </div>
        ))}
      </div>
    </FlexBox>
    <Notes>Governance entities cross-reference other extensions. A decision protects invariants from @specforge/software. A constraint applies to behaviors. failure_mode threatens invariants. The RPN arithmetic is compiler-validated.</Notes>
  </Slide>,

  // ─── Slide 25: Governance DSL ──────────────────────────────────────────
  <FullBleedCode
    key="governance-dsl"
    filename="spec/governance/decisions.spec + constraints.spec + failure_modes.spec"
    language="hcl"
    badge="governance"
    badgeColor={colors.accent.purple}
    code={specGovernanceExample}
    takeaway={'Decisions = WHY (context + consequences). Constraints = WHAT limits apply (metric + threshold, testable). Failure modes = WHERE risks are (FMEA with compiler-validated RPN). A security agent reads all three to assess risk; an architecture agent traces decisions to their protected invariants.'}
    notes={<Notes>Three entity kinds, three purposes. Decisions capture architectural reasoning. Constraints define measurable NFRs. Failure modes quantify risk with FMEA math. All cross-reference @specforge/software entities.</Notes>}
  />,

  // ─── Slide 28: @specforge/formal — 5 Entity Kinds ─────────────────────
  <Slide key="ext-formal" backgroundColor={colors.bg.primary}>
    <FlexBox flexDirection="column" alignItems="center" justifyContent="flex-start" height="100%" style={{ paddingTop: 24 }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginBottom: 16 }}>
        <div style={{ padding: '4px 12px', background: `${colors.accent.yellow}15`, border: `1px solid ${colors.accent.yellow}40`, borderRadius: 999, fontFamily: '"JetBrains Mono", monospace', fontSize: 16, fontWeight: 700, color: colors.accent.yellow }}>@specforge/formal</div>
        <Text fontSize="16px" color={colors.text.muted} margin="0">5 entity kinds &middot; 12 edge types &middot; requires @specforge/software</Text>
      </div>
      <Heading fontSize="40px" color={colors.text.primary} margin="0 0 14px 0" style={{ textAlign: 'center' }}>
        Formal Methods &amp; Rigorous Specification
      </Heading>
      <Text fontSize="18px" color={colors.text.secondary} margin="0 0 32px 0" style={{ maxWidth: 800, textAlign: 'center', lineHeight: 1.5 }}>
        Adds formal specification constructs on top of @specforge/software. Enhances software entities via <span style={{ fontFamily: '"JetBrains Mono", monospace', color: colors.accent.teal }}>entity_enhancements</span>.
      </Text>

      <div style={{ display: 'flex', gap: 18, maxWidth: 900, width: '100%', marginBottom: 28 }}>
        {[
          { kind: 'property', desc: 'Temporal assertion: safety, liveness, or fairness. Tracked with formal coverage.', color: colors.accent.yellow },
          { kind: 'axiom', desc: 'Assumed-true foundation. No proof required, no coverage tracking needed.', color: colors.accent.orange },
          { kind: 'protocol', desc: 'Shared sync contract: event ordering, timeout, delivery semantics.', color: colors.accent.green },
          { kind: 'refinement', desc: 'Abstract → concrete behavior mapping. Tracks condition deltas and proof status.', color: colors.accent.teal },
          { kind: 'process', desc: 'Communicating process: event alphabet, states, composition operators.', color: colors.accent.cyan },
        ].map((item) => (
          <div key={item.kind} style={{ flex: 1, padding: '16px 14px', background: colors.bg.card, borderTop: `3px solid ${item.color}`, border: '1px solid rgba(255,255,255,0.06)', borderTopWidth: 3, borderTopStyle: 'solid', borderTopColor: item.color, borderRadius: 10, minWidth: 0 }}>
            <Text fontSize="18px" color={item.color} margin="0 0 6px 0" fontWeight="bold" style={{ fontFamily: '"JetBrains Mono", monospace' }}>{item.kind}</Text>
            <Text fontSize="15px" color={colors.text.secondary} margin="0" style={{ lineHeight: 1.5 }}>{item.desc}</Text>
          </div>
        ))}
      </div>

      <div style={{ display: 'flex', gap: 18, maxWidth: 900, width: '100%' }}>
        <div style={{ flex: 1, padding: '14px 20px', background: `${colors.accent.yellow}06`, borderLeft: `4px solid ${colors.accent.yellow}`, borderRadius: '0 10px 10px 0' }}>
          <Text fontSize="16px" color={colors.accent.yellow} margin="0" fontWeight="bold">Entity Enhancements</Text>
          <Text fontSize="15px" color={colors.text.secondary} margin="6px 0 0 0">Adds <span style={{ fontFamily: '"JetBrains Mono", monospace', color: colors.accent.teal }}>requires</span>/<span style={{ fontFamily: '"JetBrains Mono", monospace', color: colors.accent.teal }}>ensures</span>/<span style={{ fontFamily: '"JetBrains Mono", monospace', color: colors.accent.teal }}>maintains</span> to behaviors, <span style={{ fontFamily: '"JetBrains Mono", monospace', color: colors.accent.teal }}>sync</span> blocks to events — without modifying @specforge/software.</Text>
        </div>
        <div style={{ flex: 1, padding: '14px 20px', background: 'rgba(255,255,255,0.03)', borderLeft: `4px solid ${colors.accent.orange}`, borderRadius: '0 10px 10px 0' }}>
          <Text fontSize="16px" color={colors.accent.orange} margin="0" fontWeight="bold">4 Compiler Passes</Text>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 4, marginTop: 6 }}>
            {[
              'condition_check — satisfiability, reachability',
              'layering_verify — refinement DAG (E032)',
              'event_graph_analyze — deadlock detection (E034)',
              'coverage_tracking — proof obligations',
            ].map((p) => (
              <Text key={p} fontSize="14px" color={colors.text.muted} margin="0" style={{ fontFamily: '"JetBrains Mono", monospace' }}>{p}</Text>
            ))}
          </div>
        </div>
      </div>
    </FlexBox>
    <Notes>@specforge/formal is the most sophisticated extension. Entity enhancements let it ADD fields to software entities. 4 compiler passes do formal analysis. 8 diagnostic codes. All behind the warning_level=strict flag.</Notes>
  </Slide>,

  // ─── Slide 27: Formal DSL ──────────────────────────────────────────────
  <FullBleedCode
    key="formal-dsl"
    filename="spec/formal/properties.spec + axioms.spec + protocols.spec + refinements.spec + processes.spec"
    language="hcl"
    badge="formal"
    badgeColor={colors.accent.yellow}
    code={specFormalExample}
    takeaway={'Properties = temporal assertions. Axioms = assumed-true foundations. Protocols = sync contracts. Refinements = abstract→concrete mapping with proof tracking (Specification Layering). Processes = communicating state machines for Event Graph Linting.'}
    notes={<Notes>@specforge/formal has two mechanisms: (1) inline condition fields on behaviors/ports (requires/ensures/maintains) and (2) standalone entities (property, axiom, protocol, refinement, process). Both feed into formal compiler passes.</Notes>}
  />,

  // ─── Slide 28: 22 Entity Kinds Across 4 Extensions ────────────────────
  <Slide key="ext-summary" backgroundColor={colors.bg.primary}>
    <FlexBox flexDirection="column" alignItems="center" justifyContent="center" height="100%">
      <Heading fontSize="40px" color={colors.text.primary} margin="0 0 8px 0" style={{ textAlign: 'center' }}>
        22 Entity Kinds Across 4 Extensions
      </Heading>
      <Text fontSize="18px" color={colors.text.secondary} margin="0 0 28px 0" style={{ textAlign: 'center', maxWidth: 750, lineHeight: 1.5 }}>
        Plus 2 structural kinds (<span style={{ fontFamily: '"JetBrains Mono", monospace', color: colors.accent.teal }}>spec</span> and <span style={{ fontFamily: '"JetBrains Mono", monospace', color: colors.accent.teal }}>ref</span>) in the compiler core. Total: 24 DSL keywords. Zero hardcoded in the compiler.
      </Text>

      <div style={{ display: 'flex', gap: 16, maxWidth: 900, width: '100%', marginBottom: 24 }}>
        {[
          { name: '@specforge/software', count: 5, kinds: ['behavior', 'invariant', 'event', 'type', 'port'], edges: 12, diagnostics: 12, color: colors.accent.teal },
          { name: '@specforge/product', count: 9, kinds: ['feature', 'journey', 'milestone', 'deliverable', 'module', 'term', 'persona', 'channel', 'release'], edges: 20, diagnostics: 27, color: colors.accent.blue },
          { name: '@specforge/governance', count: 3, kinds: ['decision', 'constraint', 'failure_mode'], edges: 11, diagnostics: 4, color: colors.accent.purple },
          { name: '@specforge/formal', count: 5, kinds: ['property', 'axiom', 'protocol', 'refinement', 'process'], edges: 12, diagnostics: 8, color: colors.accent.yellow },
        ].map((ext) => (
          <div key={ext.name} style={{ flex: 1, padding: '14px 14px', background: colors.bg.card, borderTop: `3px solid ${ext.color}`, border: '1px solid rgba(255,255,255,0.06)', borderTopWidth: 3, borderTopStyle: 'solid', borderTopColor: ext.color, borderRadius: 10, minWidth: 0 }}>
            <Text fontSize="14px" color={ext.color} margin="0 0 6px 0" fontWeight="bold" style={{ fontFamily: '"JetBrains Mono", monospace' }}>{ext.name}</Text>
            <Text fontSize="28px" color={colors.text.primary} margin="0 0 6px 0" fontWeight="bold">{ext.count} <span style={{ fontSize: 14, color: colors.text.muted }}>kinds</span></Text>
            <div style={{ display: 'flex', flexWrap: 'wrap', gap: 4, marginBottom: 8 }}>
              {ext.kinds.map((k) => (
                <span key={k} style={{ fontSize: 14, padding: '1px 6px', background: `${ext.color}12`, border: `1px solid ${ext.color}30`, borderRadius: 4, color: ext.color, fontFamily: '"JetBrains Mono", monospace' }}>{k}</span>
              ))}
            </div>
            <Text fontSize="14px" color={colors.text.muted} margin="0">{ext.edges} edge types &middot; {ext.diagnostics} diagnostics</Text>
          </div>
        ))}
      </div>

      <div style={{ display: 'flex', gap: 16, maxWidth: 900, width: '100%' }}>
        {[
          { stat: '55', label: 'Total edge types across all extensions', color: colors.accent.teal },
          { stat: '51', label: 'Diagnostic codes (errors, warnings, info)', color: colors.accent.red },
          { stat: '47', label: 'CLI commands contributed by extensions', color: colors.accent.green },
          { stat: '39', label: 'MCP resources auto-registered', color: colors.accent.purple },
        ].map((s) => (
          <div key={s.label} style={{ flex: 1, textAlign: 'center', padding: '14px 12px', background: 'rgba(255,255,255,0.03)', border: '1px solid rgba(255,255,255,0.06)', borderRadius: 10 }}>
            <div style={{ fontSize: 28, fontWeight: 800, color: s.color, fontFamily: '"Inter", sans-serif', lineHeight: 1 }}>{s.stat}</div>
            <Text fontSize="14px" color={colors.text.secondary} margin="6px 0 0 0">{s.label}</Text>
          </div>
        ))}
      </div>
    </FlexBox>
    <Notes>Full ecosystem: 4 extensions, 22 entity kinds, 55 edge types, 51 diagnostics. Every single one contributed by extensions. Anyone can write new extensions in any language that compiles to Wasm.</Notes>
  </Slide>,
];
