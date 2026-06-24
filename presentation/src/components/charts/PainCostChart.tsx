import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  LabelList,
} from 'recharts';
import { colors } from '../../theme/colors';
import { painCostData } from '../../data/chart-data';

interface PainCostChartProps {
  data?: typeof painCostData;
  width?: number;
  height?: number;
}

/**
 * Horizontal bar chart showing the annual cost per 7-person engineering team
 * broken down by category: developer hours, AI token waste, major incidents.
 *
 * Uses explicit width/height (NOT ResponsiveContainer) because Spectacle's
 * CSS transform: scale() causes ResizeObserver to report 0 width.
 */
export function PainCostChart({
  data = painCostData,
  width = 1100,
  height = 340,
}: PainCostChartProps) {
  return (
    <BarChart
      data={data}
      layout="vertical"
      width={width}
      height={height}
      margin={{ top: 8, right: 120, left: 20, bottom: 8 }}
    >
      <CartesianGrid
        strokeDasharray="3 3"
        stroke={colors.bg.tertiary}
        horizontal={false}
      />
      <XAxis
        type="number"
        tick={{ fill: colors.text.muted, fontSize: 14 }}
        axisLine={{ stroke: 'rgba(255,255,255,0.1)' }}
        tickFormatter={(value: number) => `$${(value / 1000).toFixed(0)}K`}
      />
      <YAxis
        type="category"
        dataKey="category"
        width={180}
        tick={{ fill: colors.text.primary, fontSize: 16 }}
        axisLine={false}
        tickLine={false}
      />
      <Bar
        dataKey="annual"
        animationDuration={800}
        radius={[0, 4, 4, 0]}
        fill={colors.accent.red}
      >
        <LabelList
          dataKey="annual"
          position="right"
          fill={colors.text.primary}
          fontSize={16}
          fontWeight={700}
          formatter={(value: number) => `$${value.toLocaleString()}`}
        />
      </Bar>
    </BarChart>
  );
}
