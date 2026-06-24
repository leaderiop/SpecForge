import { CodePane } from 'spectacle';
import { useState, useEffect } from 'react';
import { colors } from '../../theme/colors';
import { specforgeCodeTheme } from '../../theme/code-theme';

interface CodeRevealProps {
  code: string;
  language: string;
  ranges: number[][];
  labels?: string[];
}

export function CodeReveal({ code, language, ranges, labels }: CodeRevealProps) {
  // Track which label to show by listening to Spectacle's step changes.
  // CodePane with highlightRanges creates sub-steps internally.
  // We observe DOM mutations on the highlight to sync our label display.
  const [activeStep, setActiveStep] = useState(0);

  useEffect(() => {
    // Spectacle updates an attribute on the slide when steps change.
    // We use a simple interval to poll the highlighted lines and update the label.
    const interval = setInterval(() => {
      const highlighted = document.querySelectorAll(
        '[data-testid="CodePane"] .token-line[style*="opacity: 1"],' +
        '[data-testid="CodePane"] .token-line:not([style*="opacity: 0.3"])'
      );
      if (highlighted.length > 0 && ranges.length > 0) {
        // Find first visible line number and match to a range
        const allLines = document.querySelectorAll('[data-testid="CodePane"] .token-line');
        if (allLines.length === 0) return;

        for (let i = ranges.length - 1; i >= 0; i--) {
          const range = ranges[i];
          const startLine = Array.isArray(range) ? range[0] : range;
          if (startLine <= allLines.length) {
            const lineEl = allLines[startLine - 1] as HTMLElement;
            if (lineEl && lineEl.style.opacity !== '0.3') {
              setActiveStep(i);
              break;
            }
          }
        }
      }
    }, 200);

    return () => clearInterval(interval);
  }, [ranges]);

  const currentLabel = labels?.[activeStep] ?? labels?.[0];

  return (
    <div>
      {currentLabel && (
        <div
          style={{
            color: colors.accent.blue,
            fontSize: 18,
            fontWeight: 500,
            marginBottom: 8,
            minHeight: 28,
            transition: 'opacity 0.3s ease',
          }}
        >
          {currentLabel}
        </div>
      )}
      <CodePane
        language={language}
        theme={specforgeCodeTheme}
        highlightRanges={ranges}
        showLineNumbers
      >
        {code}
      </CodePane>
    </div>
  );
}
