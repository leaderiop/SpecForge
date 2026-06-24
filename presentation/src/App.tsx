import { Deck, Slide, FlexBox, Text } from 'spectacle';
import type { JSX } from 'react';
import { darkTheme } from './theme/dark-theme';
import { colors } from './theme/colors';
import { ErrorBoundary } from './components/ErrorBoundary';
import { SlideTemplate } from './components/layout/SlideTemplate';
import {
  hookSlides,
  theProblemSlides,
  theInsightSlides,
  threeLayerSlides,
  coreConceptSlides,
  extensionSlides,
  theCompilerSlides,
  integrationSlides,
  closingSlides,
} from './slides';

const allSlides = [
  // 1-2: Title + What Is SpecForge
  ...hookSlides,

  // 3-5: The Problem + .cursorrules + Missing Infrastructure
  ...theProblemSlides,

  // 6-7: Core Idea (typed graph) + What Graph Gives Agents
  ...theInsightSlides,

  // 8-10: Three Layers + DSL Syntax + Imports
  ...threeLayerSlides,

  // 11-18: @specforge/software — entity kinds, edge types, definitions + graph
  ...coreConceptSlides,

  // 19-30: Extension architecture + all 4 extensions in depth (12 slides)
  ...extensionSlides,

  // 31-35: Compiler pipeline + errors + graph output + traceability + before/after
  ...theCompilerSlides,

  // 36-37: Integration (MCP/CLI/LSP) + agent workflow
  ...integrationSlides,

  // 38-39: What ships today + Try it
  ...closingSlides,
];

function wrapSlidesWithErrorBoundaries(slides: JSX.Element[]) {
  return slides.map((slide, i) => (
    <ErrorBoundary
      key={slide.key ?? i}
      fallback={
        <Slide backgroundColor={colors.bg.primary}>
          <FlexBox alignItems="center" justifyContent="center" height="100%">
            <Text color={colors.accent.red} fontSize="24px">
              Slide {i + 1} failed to render
            </Text>
          </FlexBox>
        </Slide>
      }
    >
      {slide}
    </ErrorBoundary>
  ));
}

export default function App() {
  return (
    <ErrorBoundary>
      <Deck
        theme={darkTheme}
        template={SlideTemplate}
        transition={{
          from: { opacity: 0, transform: 'translateX(2%)' },
          enter: { opacity: 1, transform: 'translateX(0%)' },
          leave: { opacity: 0, transform: 'translateX(-2%)' },
        }}
      >
        {wrapSlidesWithErrorBoundaries(allSlides)}
      </Deck>
    </ErrorBoundary>
  );
}
