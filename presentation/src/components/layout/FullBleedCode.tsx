import { CodePane, Slide, Text } from 'spectacle';
import type { ReactNode } from 'react';
import { colors, safe } from '../../theme/colors';
import { specforgeCodeTheme } from '../../theme/code-theme';

// Full-bleed code slide. The code is the hero — everything else is chrome.
// Font-size uses clamp() so projected output stays legible from the back row.
export function FullBleedCode({
  filename,
  language = 'typescript',
  code,
  takeaway,
  badge,
  badgeColor,
  notes,
  slideKey,
}: {
  filename: string;
  language?: string;
  code: string;
  takeaway: string;
  badge?: string;
  badgeColor?: string;
  notes?: ReactNode;
  slideKey?: string;
}) {
  const accent = badgeColor ?? colors.accent.blue;
  return (
    <Slide key={slideKey} backgroundColor={colors.bg.primary} padding={0}>
      <div style={{
        display: 'flex',
        flexDirection: 'column',
        height: '100%',
        padding: `${safe.top}px 32px ${safe.bottom}px`,
        gap: 6,
      }}>
        {/* Filename-tab header */}
        <div style={{
          display: 'flex',
          alignItems: 'center',
          gap: 12,
          padding: '6px 14px',
          background: 'rgba(255,255,255,0.03)',
          border: '1px solid rgba(255,255,255,0.06)',
          borderBottom: 'none',
          borderRadius: '10px 10px 0 0',
          width: 'fit-content',
        }}>
          <div style={{ display: 'flex', gap: 6 }}>
            <div style={{ width: 8, height: 8, borderRadius: '50%', background: '#ff6155' }} />
            <div style={{ width: 8, height: 8, borderRadius: '50%', background: '#f5bf3f' }} />
            <div style={{ width: 8, height: 8, borderRadius: '50%', background: '#57c038' }} />
          </div>
          <Text
            fontSize="13px"
            color={colors.text.secondary}
            margin="0"
            style={{ fontFamily: '"JetBrains Mono", monospace' }}
          >
            {filename}
          </Text>
          {badge && (
            <div style={{
              padding: '1px 8px',
              background: `${accent}18`,
              border: `1px solid ${accent}60`,
              borderRadius: 999,
              fontSize: 11,
              color: accent,
              fontWeight: 700,
              letterSpacing: '0.08em',
              textTransform: 'uppercase',
            }}>
              {badge}
            </div>
          )}
        </div>

        {/* Full-bleed code */}
        <div className="fullbleed-code-pane" style={{
          flex: 1,
          minHeight: 0,
          background: '#0a0a12',
          border: '1px solid rgba(255,255,255,0.06)',
          borderRadius: '0 10px 10px 10px',
          padding: '10px 20px',
          overflow: 'hidden',
        }}>
          <CodePane
            language={language}
            theme={specforgeCodeTheme}
            showLineNumbers={false}
          >
            {code}
          </CodePane>
        </div>

        {/* Takeaway footer band */}
        <div style={{
          padding: '8px 18px',
          borderLeft: `4px solid ${accent}`,
          background: `${accent}10`,
          borderRadius: '0 8px 8px 0',
        }}>
          <Text fontSize="16px" color={accent} margin="0" fontWeight="bold">
            {takeaway}
          </Text>
        </div>
      </div>
      {notes}
    </Slide>
  );
}
