// 7-step type scale. Pick a step, not a random px value.
// Keep ad-hoc fontSize out of slide files — if a new size is needed, add it here.
export const type = {
  display: '72px',
  h1: '56px',
  h2: '38px',
  h3: '28px',
  body: '22px',
  small: '18px',
  caption: '15px',
  micro: '13px',
} as const;

export const weight = {
  regular: 400,
  medium: 500,
  bold: 700,
} as const;

export const lineHeight = {
  tight: 1.15,
  normal: 1.45,
  loose: 1.6,
} as const;

export const letterSpacing = {
  tight: '-0.01em',
  normal: '0',
  wide: '0.08em',
  wider: '0.15em',
} as const;
