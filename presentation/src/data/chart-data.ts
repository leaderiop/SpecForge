export const timeLossData = [
  { activity: 'Fixing AI output', hours: 3.5 },
  { activity: 'Assembling context', hours: 2.5 },
  { activity: 'Reviewing for correctness', hours: 2.5 },
  { activity: 'Maintaining context docs', hours: 1.5 },
];

export const painCostData = [
  { category: 'Developer hours', annual: 280000 },
  { category: 'AI token waste', annual: 15000 },
  { category: 'Major incidents', annual: 42000 },
];

export const tokenComparisonData = [
  { scale: 'Solo dev', without: 6200, with: 805 },
  { scale: '10-person team', without: 62000, with: 8050 },
  { scale: '100-person org', without: 140000, with: 17500 },
];

export const competitiveLandscape = [
  { solution: '.cursorrules / CLAUDE.md', validated: false, graph: false, multiRes: false, crossPlatform: false, agentNative: false },
  { solution: 'RAG pipelines', validated: false, graph: false, multiRes: false, crossPlatform: true, agentNative: false },
  { solution: 'MCP (transport layer)', validated: false, graph: false, multiRes: false, crossPlatform: true, agentNative: true },
  { solution: 'Architecture-as-code', validated: true, graph: true, multiRes: false, crossPlatform: true, agentNative: false },
  { solution: 'Schema languages', validated: true, graph: false, multiRes: false, crossPlatform: true, agentNative: false },
  { solution: 'SpecForge', validated: true, graph: true, multiRes: true, crossPlatform: true, agentNative: true },
];

export const keyMetrics = [
  { metric: 'Install → First Spec', target: '>40%', danger: '<20%' },
  { metric: 'First Spec → First Export', target: '>25%', danger: '<10%' },
  { metric: 'First Export → Week 2 active', target: '>30%', danger: '<15%' },
  { metric: 'Solo → Team adoption', target: '>15% in 60d', danger: '<5%' },
  { metric: 'Time to first export', target: '<10 min', danger: '>15 min' },
];
