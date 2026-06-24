import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Cell,
  LabelList,
} from 'recharts';
import { colors } from '../../theme/colors';
import { timeLossData } from '../../data/chart-data';

interface TimeLossChartProps {
  data?: typeof timeLossData;
  width?: number;
  height?: number;
}

/**
 * Horizontal bar chart showing per-developer weekly time loss by activity.
 * Bars use an orange-to-red gradient based on hours (higher = redder).
 *
 * Uses explicit width/height (NOT ResponsiveContainer) because Spectacle's
 * CSS transform: scale() causes ResizeObserver to report 0 width.
 */
export function TimeLossChart({
  data = timeLossData,
  width = 1100,
  height = 340,
}: TimeLossChartProps) {
  const getBarColor = (hours: number) => {
    if (hours >= 3) return colors.accent.red;
    if (hours >= 2) return colors.accent.orange;
    return colors.accent.yellow;
  };

  return (
    <BarChart
      data={data}
      layout="vertical"
      width={width}
      height={height}
      margin={{ top: 8, right: 100, left: 20, bottom: 8 }}
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
        domain={[0, 4]}
        tickFormatter={(value: number) => `${value}h`}
      />
      <YAxis
        type="category"
        dataKey="activity"
        width={220}
        tick={{ fill: colors.text.primary, fontSize: 16 }}
        axisLine={false}
        tickLine={false}
      />
      <Bar dataKey="hours" animationDuration={800} radius={[0, 4, 4, 0]}>
        {data.map((entry, index) => (
          <Cell key={index} fill={getBarColor(entry.hours)} />
        ))}
        <LabelList
          dataKey="hours"
          position="right"
          fill={colors.text.primary}
          fontSize={16}
          fontWeight={700}
          formatter={(value: number) => `${value} hrs/week`}
        />
      </Bar>
    </BarChart>
  );
}
