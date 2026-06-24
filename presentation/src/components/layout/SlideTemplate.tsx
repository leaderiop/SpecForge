import { FlexBox, FullScreen } from 'spectacle';
import { colors } from '../../theme/colors';

const SECTION_MAP: Array<{ upTo: number; name: string; color: string }> = [
  { upTo: 2, name: 'SpecForge', color: colors.brandLight },
  { upTo: 5, name: 'The Problem', color: colors.accent.red },
  { upTo: 7, name: 'The Core Idea', color: colors.accent.green },
  { upTo: 10, name: 'Three Layers', color: colors.brand },
  { upTo: 18, name: 'Core Concepts', color: colors.accent.teal },
  { upTo: 30, name: 'Extensions', color: colors.accent.purple },
  { upTo: 35, name: 'The Compiler', color: colors.accent.red },
  { upTo: 37, name: 'Integration', color: colors.accent.blue },
  { upTo: 999, name: 'Try It', color: colors.accent.green },
];

function getSectionInfo(slideNumber: number) {
  for (const section of SECTION_MAP) {
    if (slideNumber <= section.upTo) return section;
  }
  return SECTION_MAP[SECTION_MAP.length - 1];
}

export function SlideTemplate({
  slideNumber,
  numberOfSlides,
}: {
  slideNumber: number;
  numberOfSlides: number;
}) {
  const progress = numberOfSlides > 0 ? (slideNumber / numberOfSlides) * 100 : 0;
  const section = getSectionInfo(slideNumber);

  return (
    <>
      <FlexBox
        position="absolute"
        top={12}
        right={16}
        alignItems="center"
        style={{ zIndex: 10 }}
      >
        <div
          style={{
            fontSize: 14,
            color: colors.text.muted,
            fontFamily: '"Inter", sans-serif',
            letterSpacing: '0.08em',
            textTransform: 'uppercase',
            display: 'flex',
            alignItems: 'center',
            gap: 8,
            padding: '6px 12px',
            background: 'rgba(10, 10, 26, 0.85)',
            border: '1px solid rgba(255, 255, 255, 0.06)',
            borderRadius: 999,
            backdropFilter: 'blur(4px)',
          }}
        >
          <div style={{ width: 6, height: 6, borderRadius: '50%', background: colors.brand }} />
          <span style={{ color: colors.brandLight, fontWeight: 600 }}>SpecForge</span>
          <span style={{ opacity: 0.4 }}>&middot;</span>
          <span>2026</span>
        </div>
      </FlexBox>

      <FlexBox
        position="absolute"
        bottom={0}
        left={0}
        right={0}
        height="44px"
        alignItems="center"
        justifyContent="space-between"
        padding="0 32px"
        style={{
          background: 'linear-gradient(180deg, rgba(10,14,26,0.4) 0%, rgba(10,14,26,0.98) 35%, #0a0e1a 100%)',
          zIndex: 10,
          borderTop: '1px solid rgba(255, 255, 255, 0.04)',
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <div style={{ width: 6, height: 6, borderRadius: '50%', background: section.color }} />
          <span style={{ fontSize: 14, color: colors.text.muted, fontFamily: '"Inter", sans-serif', letterSpacing: '0.04em' }}>
            {section.name}
          </span>
        </div>

        <div style={{ display: 'flex', alignItems: 'center' }}>
          <div
            style={{
              fontSize: 18,
              color: colors.text.secondary,
              fontFamily: '"Inter", sans-serif',
            }}
          >
            {slideNumber} / {numberOfSlides}
          </div>

          <div style={{ marginLeft: 12, opacity: 0.8 }}>
            <FullScreen size={20} />
          </div>
        </div>
      </FlexBox>

      <div
        style={{
          position: 'absolute',
          bottom: 0,
          left: 0,
          right: 0,
          height: 3,
          background: 'rgba(255,255,255,0.08)',
        }}
      >
        <div
          style={{
            height: '100%',
            width: `${progress}%`,
            background: `linear-gradient(90deg, ${colors.brand}, ${colors.accent.teal}, ${colors.accent.cyan})`,
            transition: 'width 0.3s ease',
          }}
        />
      </div>
    </>
  );
}
