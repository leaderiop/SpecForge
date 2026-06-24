import { Handle, Position, type NodeProps } from '@xyflow/react';
import { colors } from '../../theme/colors';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type EntityKind = 'behavior' | 'invariant' | 'event' | 'type' | 'port';

export interface EntityNodeData {
  label: string;
  kind: EntityKind;
  [key: string]: unknown;
}

// ---------------------------------------------------------------------------
// Color mapping by entity kind
// ---------------------------------------------------------------------------

const kindColors: Record<EntityKind, string> = {
  behavior: colors.accent.teal,
  invariant: colors.accent.red,
  event: colors.accent.yellow,
  type: colors.accent.blue,
  port: colors.accent.purple,
};

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function EntityNode({ data }: NodeProps) {
  const nodeData = data as unknown as EntityNodeData;
  const accentColor = kindColors[nodeData.kind] ?? colors.text.muted;

  return (
    <div
      style={{
        background: colors.bg.card,
        border: `1.5px solid ${accentColor}40`,
        borderRadius: 8,
        padding: '8px 12px',
        minWidth: 140,
        maxWidth: 180,
        cursor: 'default',
        display: 'flex',
        flexDirection: 'column',
        gap: 4,
      }}
    >
      {/* Kind badge */}
      <div
        style={{
          display: 'inline-flex',
          alignSelf: 'flex-start',
          background: `${accentColor}20`,
          border: `1px solid ${accentColor}50`,
          borderRadius: 10,
          padding: '1px 8px',
        }}
      >
        <span
          style={{
            fontSize: 10,
            fontWeight: 700,
            fontFamily: 'Inter, sans-serif',
            textTransform: 'uppercase',
            letterSpacing: '0.05em',
            color: accentColor,
          }}
        >
          {nodeData.kind}
        </span>
      </div>

      {/* Entity name */}
      <div
        style={{
          fontSize: 14,
          fontWeight: 700,
          fontFamily: 'JetBrains Mono, monospace',
          color: colors.text.primary,
          wordBreak: 'break-word',
          lineHeight: 1.3,
        }}
      >
        {nodeData.label}
      </div>

      {/* Handles on all four sides for flexible edge routing */}
      <Handle type="target" position={Position.Top} style={{ opacity: 0 }} id="top" />
      <Handle type="source" position={Position.Bottom} style={{ opacity: 0 }} id="bottom" />
      <Handle type="target" position={Position.Left} style={{ opacity: 0 }} id="left" />
      <Handle type="source" position={Position.Right} style={{ opacity: 0 }} id="right" />

      {/* Additional source/target on opposing sides for bidirectional edges */}
      <Handle type="source" position={Position.Top} style={{ opacity: 0 }} id="top-source" />
      <Handle type="target" position={Position.Bottom} style={{ opacity: 0 }} id="bottom-target" />
      <Handle type="source" position={Position.Left} style={{ opacity: 0 }} id="left-source" />
      <Handle type="target" position={Position.Right} style={{ opacity: 0 }} id="right-target" />
    </div>
  );
}
