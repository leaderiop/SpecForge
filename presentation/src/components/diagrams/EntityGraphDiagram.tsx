import {
  ReactFlow,
  ReactFlowProvider,
  type NodeTypes,
  type DefaultEdgeOptions,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import { ErrorBoundary } from '../ErrorBoundary';
import { EntityNode } from './EntityNode';
import { entityGraphNodes, entityGraphEdges } from '../../data/diagram-data';
import { colors } from '../../theme/colors';

// ---------------------------------------------------------------------------
// Custom node type registry
// ---------------------------------------------------------------------------

const nodeTypes: NodeTypes = {
  entity: EntityNode,
};

// ---------------------------------------------------------------------------
// Default edge styling
// ---------------------------------------------------------------------------

const defaultEdgeOptions: DefaultEdgeOptions = {
  style: { stroke: 'rgba(255,255,255,0.15)', strokeWidth: 1.5 },
  labelStyle: { fill: colors.text.primary, fontSize: 10, fontFamily: 'Inter, sans-serif' },
  labelBgStyle: { fill: colors.bg.tertiary, fillOpacity: 0.9 },
  labelBgPadding: [4, 6] as [number, number],
  labelBgBorderRadius: 3,
};

// ---------------------------------------------------------------------------
// Inner component (rendered inside ReactFlowProvider)
// ---------------------------------------------------------------------------

function EntityGraphDiagramInner() {
  return (
    <div style={{ width: '100%', height: '100%', background: colors.bg.primary }}>
      <ReactFlow
        nodes={entityGraphNodes}
        edges={entityGraphEdges}
        nodeTypes={nodeTypes}
        defaultEdgeOptions={defaultEdgeOptions}
        fitView
        fitViewOptions={{ padding: 0.15, minZoom: 0.3, maxZoom: 1.5 }}
        colorMode="dark"
        nodesDraggable={false}
        nodesConnectable={false}
        nodesFocusable={false}
        edgesFocusable={false}
        elementsSelectable={false}
        panOnDrag={false}
        panOnScroll={false}
        zoomOnScroll={false}
        zoomOnPinch={false}
        zoomOnDoubleClick={false}
        preventScrolling={false}
        proOptions={{ hideAttribution: true }}
      />
    </div>
  );
}

// ---------------------------------------------------------------------------
// Public component with error boundary
// ---------------------------------------------------------------------------

export function EntityGraphDiagram() {
  return (
    <ErrorBoundary
      fallback={
        <div style={{ color: colors.accent.red }}>
          Failed to render entity graph diagram
        </div>
      }
    >
      <ReactFlowProvider>
        <EntityGraphDiagramInner />
      </ReactFlowProvider>
    </ErrorBoundary>
  );
}
