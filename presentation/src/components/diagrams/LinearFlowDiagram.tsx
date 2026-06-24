import type { CSSProperties } from 'react';
import { Fragment } from 'react';
import { colors } from '../../theme/colors';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface FlowBox {
  label: string;
  sublabel?: string;
  color: string;
}

export interface FlowGroup {
  title?: string;
  titleColor?: string;
  boxes: FlowBox[];
}

export interface LinearFlowDiagramProps {
  groups: FlowGroup[];
  connectorLabel?: string;
  connectorDashed?: boolean;
  height?: number;
}

// ---------------------------------------------------------------------------
// Styles (inline objects)
// ---------------------------------------------------------------------------

const boxStyle = (accentColor: string): CSSProperties => ({
  padding: '14px 18px',
  background: `${accentColor}15`,
  border: `2px solid ${accentColor}`,
  borderRadius: 8,
  textAlign: 'center' as const,
});

const boxLabelStyle = (accentColor: string): CSSProperties => ({
  fontSize: 18,
  fontFamily: 'Inter, sans-serif',
  fontWeight: 700,
  color: accentColor,
  whiteSpace: 'pre-line' as const,
});

const boxSublabelStyle: CSSProperties = {
  fontSize: 15,
  fontFamily: 'Inter, sans-serif',
  color: colors.text.secondary,
  whiteSpace: 'pre-line' as const,
  marginTop: 4,
};

const groupContainerStyle: CSSProperties = {
  padding: 16,
  border: '1px solid rgba(255,255,255,0.08)',
  borderRadius: 12,
  background: 'rgba(255,255,255,0.02)',
  display: 'flex',
  alignItems: 'center',
  gap: 12,
};

const groupTitleStyle = (titleColor: string): CSSProperties => ({
  fontSize: 16,
  fontFamily: 'Inter, sans-serif',
  fontWeight: 700,
  textTransform: 'uppercase' as const,
  letterSpacing: '0.1em',
  color: titleColor,
  marginBottom: 10,
});

// ---------------------------------------------------------------------------
// Sub-components
// ---------------------------------------------------------------------------

function Box({ box }: { box: FlowBox }) {
  return (
    <div style={boxStyle(box.color)}>
      <div style={boxLabelStyle(box.color)}>{box.label}</div>
      {box.sublabel && <div style={boxSublabelStyle}>{box.sublabel}</div>}
    </div>
  );
}

function InlineArrow({ dashed = false }: { dashed?: boolean }) {
  return (
    <span
      style={{
        fontSize: 24,
        color: colors.text.secondary,
        flexShrink: 0,
        userSelect: 'none' as const,
      }}
    >
      {dashed ? '⇢' : '→'}
    </span>
  );
}

function GroupArrow({ label }: { label?: string }) {
  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        flexShrink: 0,
        gap: 4,
      }}
    >
      <span style={{ fontSize: 32, color: colors.text.secondary, userSelect: 'none' as const }}>
        {'→'}
      </span>
      {label && (
        <span
          style={{
            fontSize: 14,
            fontFamily: 'Inter, sans-serif',
            color: colors.text.secondary,
            whiteSpace: 'nowrap' as const,
          }}
        >
          {label}
        </span>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main component
// ---------------------------------------------------------------------------

export function LinearFlowDiagram({
  groups,
  connectorLabel,
  connectorDashed = false,
  height,
}: LinearFlowDiagramProps) {
  const containerStyle: CSSProperties = {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    gap: 16,
    ...(height != null ? { height } : {}),
  };

  // Single-group mode: render boxes inline with arrows between them
  if (groups.length === 1) {
    const { boxes } = groups[0];
    const lastIdx = boxes.length - 1;

    return (
      <div style={containerStyle}>
        {boxes.map((box, i) => (
          <Fragment key={i}>
            <Box box={box} />
            {i < lastIdx && (
              <InlineArrow dashed={connectorDashed && i === lastIdx - 1} />
            )}
          </Fragment>
        ))}
      </div>
    );
  }

  // Multi-group mode: render group containers with big arrows between them
  return (
    <div style={containerStyle}>
      {groups.map((group, gi) => (
        <Fragment key={gi}>
          <div>
            {group.title && (
              <div style={groupTitleStyle(group.titleColor ?? colors.text.primary)}>
                {group.title}
              </div>
            )}
            <div style={groupContainerStyle}>
              {group.boxes.map((box, bi) => (
                <Fragment key={bi}>
                  <Box box={box} />
                  {bi < group.boxes.length - 1 && <InlineArrow />}
                </Fragment>
              ))}
            </div>
          </div>
          {gi < groups.length - 1 && <GroupArrow label={connectorLabel} />}
        </Fragment>
      ))}
    </div>
  );
}
