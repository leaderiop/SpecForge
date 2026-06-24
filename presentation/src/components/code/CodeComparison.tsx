import { CodePane } from 'spectacle';
import { TwoColumn } from '../layout/TwoColumn';
import { colors } from '../../theme/colors';
import { specforgeCodeTheme } from '../../theme/code-theme';

interface CodeComparisonProps {
  before: string;
  after: string;
  language: string;
  beforeTitle?: string;
  afterTitle?: string;
}

export function CodeComparison({ before, after, language, beforeTitle, afterTitle }: CodeComparisonProps) {
  return (
    <TwoColumn
      left={
        <div>
          {beforeTitle && (
            <div style={{ color: colors.accent.red, fontSize: 18, fontWeight: 600, marginBottom: 8 }}>
              {beforeTitle}
            </div>
          )}
          <div style={{ fontSize: 16 }}>
            <CodePane language={language} theme={specforgeCodeTheme} showLineNumbers={false}>
              {before}
            </CodePane>
          </div>
        </div>
      }
      right={
        <div>
          {afterTitle && (
            <div style={{ color: colors.accent.green, fontSize: 18, fontWeight: 600, marginBottom: 8 }}>
              {afterTitle}
            </div>
          )}
          <div style={{ fontSize: 16 }}>
            <CodePane language={language} theme={specforgeCodeTheme} showLineNumbers={false}>
              {after}
            </CodePane>
          </div>
        </div>
      }
    />
  );
}
