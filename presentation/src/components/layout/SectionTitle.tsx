import { Slide, Heading, Text, FlexBox } from 'spectacle';
import { colors } from '../../theme/colors';

interface SectionTitleProps {
  heading: string;
  subtitle?: string;
  preview?: string;
  sectionNumber?: number;
  totalSections?: number;
}

const TOTAL_SECTIONS = 6;

export function SectionTitle({ heading, subtitle, preview, sectionNumber, totalSections = TOTAL_SECTIONS }: SectionTitleProps) {
  // Color rule: brandLight for hook/case, red for problem, teal for insight/solution, blue for GTM.
  const getAccentColor = (num: number | undefined) => {
    if (!num) return colors.brandLight;
    if (num === 1) return colors.brandLight;
    if (num === 2) return colors.accent.red;
    if (num === 3) return colors.accent.teal;
    if (num === 4) return colors.accent.teal;
    if (num === 5) return colors.accent.blue;
    if (num === 6) return colors.brandLight;
    return colors.brandLight;
  };
  const accentColor = getAccentColor(sectionNumber);

  const getSectionTint = (num: number | undefined) => {
    if (!num) return 'transparent';
    if (num === 1) return 'rgba(56, 189, 248, 0.06)';
    if (num === 2) return 'rgba(255, 123, 123, 0.06)';
    if (num === 3) return 'rgba(45, 212, 191, 0.06)';
    if (num === 4) return 'rgba(45, 212, 191, 0.06)';
    if (num === 5) return 'rgba(125, 180, 250, 0.06)';
    if (num === 6) return 'rgba(56, 189, 248, 0.06)';
    return 'rgba(56, 189, 248, 0.06)';
  };
  const sectionTint = getSectionTint(sectionNumber);

  return (
    <Slide backgroundColor={colors.bg.primary}>
      <div style={{ position: 'absolute', inset: 0, background: sectionTint, pointerEvents: 'none' as const }} />
      <FlexBox flexDirection="column" alignItems="center" justifyContent="center" height="100%">
        {sectionNumber !== undefined && (
          <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', marginBottom: 32 }}>
            <div style={{
              fontSize: 80,
              fontWeight: 800,
              color: accentColor,
              opacity: 0.8,
              lineHeight: 1,
              marginBottom: 16,
              fontFamily: '"Inter", sans-serif',
              background: `radial-gradient(ellipse at center, ${accentColor}18 0%, transparent 70%)`,
              padding: '24px 48px',
            }}>
              {String(sectionNumber).padStart(2, '0')}
            </div>
            <div style={{ display: 'flex', gap: 5 }}>
              {Array.from({ length: totalSections }, (_, i) => (
                <div
                  key={i}
                  style={{
                    width: i + 1 === sectionNumber ? 36 : 12,
                    height: 5,
                    borderRadius: 2,
                    background: i + 1 <= sectionNumber ? accentColor : 'rgba(255,255,255,0.12)',
                    transition: 'all 0.3s ease',
                  }}
                />
              ))}
            </div>
          </div>
        )}
        <Heading fontSize="h1" color={colors.text.primary} margin="0 0 16px 0" style={{ textAlign: 'center' }}>
          {heading}
        </Heading>
        <div style={{ width: 80, height: 4, background: accentColor, borderRadius: 2, margin: '0 auto 24px' }} />
        {subtitle && (
          <Text fontSize="h3" color={colors.text.secondary} margin="0" style={{ maxWidth: 800, textAlign: 'center' }}>
            {subtitle}
          </Text>
        )}
        {preview && (
          <Text fontSize="20px" color={colors.text.muted} margin="16px 0 0 0" style={{ maxWidth: 700, textAlign: 'center', fontStyle: 'italic' }}>
            {preview}
          </Text>
        )}
      </FlexBox>
    </Slide>
  );
}
