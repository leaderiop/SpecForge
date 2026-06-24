import { colors } from '../../theme/colors';

export const diagramColors = {
  domain:  { fill: '#1a4d3e', text: '#51cf66', stroke: '#51cf66' },
  app:     { fill: '#1a3352', text: '#60a5fa', stroke: '#60a5fa' },
  infra:   { fill: '#3d3520', text: '#fbbf24', stroke: '#fbbf24' },
  adapter: { fill: '#3d2920', text: '#fb923c', stroke: '#fb923c' },
  error:   { fill: '#4d1a1a', text: '#ff6b6b', stroke: '#ff6b6b' },
  scale:   { fill: '#2d1a4d', text: '#a855f7', stroke: '#a855f7' },
  graph:   { fill: '#0d3b3b', text: '#2dd4bf', stroke: '#2dd4bf' },
  neutral: { fill: colors.bg.secondary, text: colors.text.primary, stroke: colors.bg.tertiary },
  muted:   { fill: '#2d3748', text: '#94a3b8', stroke: '#4a5568' },
} as const;

export function ArrowDefs({ id = 'diagram' }: { id?: string }) {
  return (
    <defs>
      <marker
        id={`${id}-arrow`}
        viewBox="0 0 10 7"
        refX="10"
        refY="3.5"
        markerWidth="8"
        markerHeight="6"
        orient="auto-start-reverse"
      >
        <path d="M 0 0 L 10 3.5 L 0 7 z" fill={colors.text.muted} />
      </marker>
      <marker
        id={`${id}-arrow-green`}
        viewBox="0 0 10 7"
        refX="10"
        refY="3.5"
        markerWidth="8"
        markerHeight="6"
        orient="auto-start-reverse"
      >
        <path d="M 0 0 L 10 3.5 L 0 7 z" fill={colors.accent.green} />
      </marker>
      <marker
        id={`${id}-arrow-red`}
        viewBox="0 0 10 7"
        refX="10"
        refY="3.5"
        markerWidth="8"
        markerHeight="6"
        orient="auto-start-reverse"
      >
        <path d="M 0 0 L 10 3.5 L 0 7 z" fill={colors.accent.red} />
      </marker>
      <marker
        id={`${id}-arrow-blue`}
        viewBox="0 0 10 7"
        refX="10"
        refY="3.5"
        markerWidth="8"
        markerHeight="6"
        orient="auto-start-reverse"
      >
        <path d="M 0 0 L 10 3.5 L 0 7 z" fill={colors.accent.blue} />
      </marker>
      <marker
        id={`${id}-arrow-teal`}
        viewBox="0 0 10 7"
        refX="10"
        refY="3.5"
        markerWidth="8"
        markerHeight="6"
        orient="auto-start-reverse"
      >
        <path d="M 0 0 L 10 3.5 L 0 7 z" fill={colors.accent.teal} />
      </marker>
      <marker
        id={`${id}-diamond`}
        viewBox="0 0 12 8"
        refX="0"
        refY="4"
        markerWidth="10"
        markerHeight="8"
        orient="auto-start-reverse"
      >
        <path d="M 0 4 L 6 0 L 12 4 L 6 8 z" fill={colors.text.primary} />
      </marker>
    </defs>
  );
}

export function SvgArrow({
  x1, y1, x2, y2,
  markerId = 'diagram-arrow',
  dashed = false,
  stroke,
  strokeWidth = 1.5,
}: {
  x1: number; y1: number;
  x2: number; y2: number;
  markerId?: string;
  dashed?: boolean;
  stroke?: string;
  strokeWidth?: number;
}) {
  return (
    <line
      x1={x1} y1={y1} x2={x2} y2={y2}
      stroke={stroke ?? colors.text.muted}
      strokeWidth={strokeWidth}
      strokeDasharray={dashed ? '6 4' : undefined}
      markerEnd={`url(#${markerId})`}
    />
  );
}

export function SvgCurveArrow({
  x1, y1, cx, cy, x2, y2,
  markerId = 'diagram-arrow',
  dashed = false,
  stroke,
  strokeWidth = 1.5,
}: {
  x1: number; y1: number;
  cx: number; cy: number;
  x2: number; y2: number;
  markerId?: string;
  dashed?: boolean;
  stroke?: string;
  strokeWidth?: number;
}) {
  return (
    <path
      d={`M ${x1} ${y1} Q ${cx} ${cy} ${x2} ${y2}`}
      fill="none"
      stroke={stroke ?? colors.text.muted}
      strokeWidth={strokeWidth}
      strokeDasharray={dashed ? '6 4' : undefined}
      markerEnd={`url(#${markerId})`}
    />
  );
}
