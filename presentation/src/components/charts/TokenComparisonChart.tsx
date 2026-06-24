import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Legend,
  LabelList,
} from 'recharts';
import { colors } from '../../theme/colors';
import { tokenComparisonData } from '../../data/chart-data';

interface TokenComparisonChartProps {
  data?: typeof tokenComparisonData;
  width?: number;
  height?: number;
}

/**
 * Grouped bar chart comparing annual token spend without vs with SpecForge.
 * Two bars per scale category: red = without, teal = with.
 *
 * Uses explicit width/height (NOT ResponsiveContainer) because Spectacle's
 * CSS transform: scale() causes ResizeObserver to report 0 width.
 */
export function TokenComparisonChart({
  data = tokenComparisonData,
  width = 1100,
  height = 420,
}: TokenComparisonChartProps) {
  return (
    <BarChart
      data={data}
      width={width}
      height={height}
      margin={{ top: 8, right: 40, left: 20, bottom: 8 }}
    >
      <CartesianGrid
        strokeDasharray="3 3"
        stroke={colors.bg.tertiary}
        vertical={false}
      />
      <XAxis
        dataKey="scale"
        tick={{ fill: colors.text.primary, fontSize: 16 }}
        axisLine={{ stroke: 'rgba(255,255,255,0.1)' }}
        tickLine={false}
      />
      <YAxis
        tick={{ fill: colors.text.muted, fontSize: 14 }}
        axisLine={{ stroke: 'rgba(255,255,255,0.1)' }}
        tickFormatter={(value: number) => `$${(value / 1000).toFixed(0)}K`}
      />
      <Legend
        verticalAlign="top"
        formatter={(value: string) => (
          <span style={{ color: colors.text.secondary, fontSize: 14 }}>
            {value === 'without' ? 'Without SpecForge' : 'With SpecForge'}
          </span>
        )}
      />
      <Bar
        dataKey="without"
        name="without"
        fill={colors.accent.red}
        animationDuration={800}
        radius={[4, 4, 0, 0]}
      >
        <LabelList
          dataKey="without"
          position="top"
          fill={colors.accent.red}
          fontSize={13}
          fontWeight={600}
          formatter={(value: number) => `$${value.toLocaleString()}`}
        />
      </Bar>
      <Bar
        dataKey="with"
        name="with"
        fill={colors.accent.teal}
        animationDuration={800}
        radius={[4, 4, 0, 0]}
      >
        <LabelList
          dataKey="with"
          position="top"
          fill={colors.accent.teal}
          fontSize={13}
          fontWeight={600}
          formatter={(value: number) => `$${value.toLocaleString()}`}
        />
      </Bar>
    </BarChart>
  );
}
