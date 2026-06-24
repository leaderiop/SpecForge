import { colors } from './colors';

export const spacing = {
  xs: 4,
  sm: 8,
  md: 16,
  lg: 24,
  xl: 40,
  xxl: 64,
} as const;

export const darkTheme = {
  colors: {
    primary: colors.text.primary,
    secondary: colors.accent.blue,
    tertiary: colors.bg.primary,
    quaternary: colors.bg.secondary,
    quinary: colors.text.secondary,
  },
  fonts: {
    header: '"Inter", sans-serif',
    text: '"Inter", sans-serif',
    monospace: '"JetBrains Mono", monospace',
  },
  fontSizes: {
    h1: '56px',
    h2: '38px',
    h3: '28px',
    text: '22px',
    monospace: '20px',
  },
  // styled-system space scale. Slide uses padding={2} by default, so space[2]
  // controls the gutter between chrome (top chip 48px, bottom bar 44px +
  // progress line 3px) and slide content. 56px clears the bottom bar + gives
  // breathing room; combined with SlideTemplate's own positioning this keeps
  // every slide inside a safe zone.
  space: [0, 24, 56],
};

export const layout = {
  contentMaxWidth: 1000,
  narrowMaxWidth: 800,
  cardPadding: '20px 28px',
  sectionGap: 40,
  itemGap: 16,
} as const;
