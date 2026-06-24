import { ReactNode } from 'react';

interface TwoColumnProps {
  left: ReactNode;
  right: ReactNode;
  split?: [number, number];
}

export function TwoColumn({ left, right, split = [50, 50] }: TwoColumnProps) {
  return (
    <div
      style={{
        display: 'flex',
        gap: '40px',
        height: '100%',
        alignItems: 'flex-start',
        padding: '0 16px',
        maxWidth: 1200,
        margin: '0 auto',
        width: '100%',
      }}
    >
      <div style={{ flex: `${split[0]}%`, minWidth: 0, overflow: 'hidden' }}>{left}</div>
      <div style={{ flex: `${split[1]}%`, minWidth: 0, overflow: 'hidden' }}>{right}</div>
    </div>
  );
}
