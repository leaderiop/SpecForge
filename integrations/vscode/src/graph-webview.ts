import * as vscode from "vscode";
import * as cp from "child_process";
import * as path from "path";
import { getGraphLayout, isGraphAutoUpdate } from "./config";
import { findCliBinary } from "./find-binary";

export class GraphWebviewPanel {
  private static instance: GraphWebviewPanel | undefined;

  private readonly panel: vscode.WebviewPanel;
  private readonly extensionUri: vscode.Uri;
  private disposables: vscode.Disposable[] = [];
  private isLoading = false;

  private constructor(
    panel: vscode.WebviewPanel,
    extensionUri: vscode.Uri
  ) {
    this.panel = panel;
    this.extensionUri = extensionUri;

    this.panel.webview.html = this.getWebviewContent();

    this.panel.webview.onDidReceiveMessage(
      (message) => this.handleMessage(message),
      null,
      this.disposables
    );

    this.panel.onDidDispose(
      () => this.dispose(),
      null,
      this.disposables
    );

    if (isGraphAutoUpdate()) {
      const saveWatcher = vscode.workspace.onDidSaveTextDocument((doc) => {
        if (doc.languageId === "specforge" || doc.fileName.endsWith(".spec")) {
          this.refresh();
        }
      });
      this.disposables.push(saveWatcher);
    }

    const configWatcher = vscode.workspace.onDidChangeConfiguration((e) => {
      if (e.affectsConfiguration("specforge.graph")) {
        this.panel.webview.postMessage({
          type: "configChanged",
          layout: getGraphLayout(),
          autoUpdate: isGraphAutoUpdate(),
        });
      }
    });
    this.disposables.push(configWatcher);

    this.refresh();
  }

  public static show(extensionUri: vscode.Uri): GraphWebviewPanel {
    if (GraphWebviewPanel.instance) {
      GraphWebviewPanel.instance.panel.reveal(vscode.ViewColumn.Beside);
      return GraphWebviewPanel.instance;
    }

    const panel = vscode.window.createWebviewPanel(
      "specforge.graphPanel",
      "SpecForge Graph",
      vscode.ViewColumn.Beside,
      {
        enableScripts: true,
        retainContextWhenHidden: true,
        localResourceRoots: [
          vscode.Uri.joinPath(extensionUri, "media"),
        ],
      }
    );

    panel.iconPath = vscode.Uri.joinPath(
      extensionUri,
      "media",
      "specforge-icon.svg"
    );

    GraphWebviewPanel.instance = new GraphWebviewPanel(panel, extensionUri);
    return GraphWebviewPanel.instance;
  }

  public static getInstance(): GraphWebviewPanel | undefined {
    return GraphWebviewPanel.instance;
  }

  public focusEntity(entityId: string): void {
    this.panel.reveal(vscode.ViewColumn.Beside);
    this.panel.webview.postMessage({
      type: "focusEntity",
      entityId,
    });
  }

  public async refresh(): Promise<void> {
    if (this.isLoading) return;
    this.isLoading = true;

    this.panel.webview.postMessage({ type: "loading" });

    try {
      const data = await this.getGraphData();
      this.panel.webview.postMessage({
        type: "graphData",
        data,
        layout: getGraphLayout(),
      });
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      this.panel.webview.postMessage({
        type: "error",
        message,
      });
    } finally {
      this.isLoading = false;
    }
  }

  private async getGraphData(): Promise<unknown> {
    const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
    if (!workspaceFolder) {
      throw new Error("No workspace folder open.");
    }

    return new Promise((resolve, reject) => {
      cp.execFile(
        findCliBinary(),
        ["export", "--format=graph"],
        {
          cwd: workspaceFolder.uri.fsPath,
          timeout: 60_000,
          maxBuffer: 50 * 1024 * 1024,
        },
        (error, stdout, stderr) => {
          if (error) {
            reject(new Error(stderr || error.message));
          } else {
            try {
              resolve(JSON.parse(stdout));
            } catch {
              reject(new Error("Failed to parse graph JSON output."));
            }
          }
        }
      );
    });
  }

  private async handleMessage(message: {
    type: string;
    entityId?: string;
    file?: string;
    line?: number;
  }): Promise<void> {
    switch (message.type) {
      case "refresh": {
        await this.refresh();
        break;
      }
      case "openEntity": {
        if (message.file) {
          const wsRoot = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
          const filePath = path.isAbsolute(message.file) ? message.file : (wsRoot ? path.join(wsRoot, message.file) : message.file);
          const uri = vscode.Uri.file(filePath);
          const doc = await vscode.workspace.openTextDocument(uri);
          const editor = await vscode.window.showTextDocument(
            doc,
            vscode.ViewColumn.One
          );
          if (message.line && message.line > 0) {
            const position = new vscode.Position(message.line - 1, 0);
            editor.selection = new vscode.Selection(position, position);
            editor.revealRange(
              new vscode.Range(position, position),
              vscode.TextEditorRevealType.InCenter
            );
          }
        }
        break;
      }
    }
  }

  private getWebviewContent(): string {
    const cssUri = this.panel.webview.asWebviewUri(
      vscode.Uri.joinPath(this.extensionUri, "media", "graph-panel", "graph.css")
    );
    const nonce = getNonce();

    return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src ${this.panel.webview.cspSource} 'nonce-${nonce}'; script-src 'nonce-${nonce}';">
  <link rel="stylesheet" href="${cssUri}">
  <title>SpecForge Graph</title>
</head>
<body>
  <div id="toolbar">
    <div class="toolbar-group toolbar-buttons">
      <button id="btn-collapse-all" title="Collapse All">Collapse</button>
      <button id="btn-zoom-in" title="Zoom In">+</button>
      <button id="btn-zoom-out" title="Zoom Out">&minus;</button>
      <button id="btn-fit" title="Fit to View">Fit</button>
      <button id="btn-refresh" title="Refresh Graph">&#8635;</button>
    </div>
    <div id="node-count" class="toolbar-info"></div>
    <div id="loading-indicator" class="hidden">
      <span class="spinner"></span> Loading...
    </div>
  </div>
  <div id="filter-bar" class="hidden"></div>
  <div id="graph-container">
    <svg id="graph-svg" xmlns="http://www.w3.org/2000/svg">
      <defs>
        <marker id="arrowhead" markerWidth="10" markerHeight="7" refX="10" refY="3.5" orient="auto">
          <polygon points="0 0, 10 3.5, 0 7" fill="var(--vscode-editor-foreground)" opacity="0.6"/>
        </marker>
      </defs>
      <g id="graph-root">
        <g id="edges-group"></g>
        <g id="nodes-group"></g>
      </g>
    </svg>
    <div id="tooltip" class="hidden"></div>
    <div id="empty-state" class="hidden">
      <p>No graph data available.</p>
      <p>Run <code>specforge check</code> first, then reopen.</p>
    </div>
    <div id="error-state" class="hidden">
      <p id="error-message"></p>
    </div>
  </div>

  <script nonce="${nonce}">
(function() {
  const vscode = acquireVsCodeApi();

  // ===== Constants =====
  const KIND_COLORS = {
    behavior: '#4285F4', type: '#7B61FF', event: '#E65100',
    invariant: '#EA4335', port: '#00ACC1',
    feature: '#34A853', journey: '#43A047', deliverable: '#1E88E5',
    milestone: '#8E24AA', module: '#607D8B', term: '#795548',
    persona: '#009688', channel: '#FF7043', release: '#FF5722',
    decision: '#FBBC04', constraint: '#FF9800', failure_mode: '#F44336',
    property: '#9C27B0', axiom: '#673AB7', protocol: '#3F51B5',
    refinement: '#5C6BC0', process: '#7986CB',
  };

  // Group node dimensions
  const GW = 140, GH = 70, GRX = 12;
  // Entity node radius
  const ER = 12;
  // Ego card dimensions (center)
  const EGO_CW = 200, EGO_CH_BASE = 60, EGO_CR = 6;
  // Ego card dimensions (neighbor)
  const EGO_NW = 170, EGO_NH_BASE = 52, EGO_NR = 5;
  // Header strip height inside ego cards
  const EGO_HEADER_H = 18;
  // Max entities to show when a group is expanded
  const MAX_EXPAND = 40;
  // Radial layout: minimum gap between entity nodes
  const ENTITY_SPACING = 60;
  // Radial layout: base radius for entity ring around group center
  const RADIAL_BASE = 100;
  // Edge threshold: hide entity edges if too many
  const EDGE_VISIBILITY_THRESHOLD = 50;

  // ===== State =====
  let allNodes = [];
  let allEdges = [];
  let nodeMap = new Map();       // id -> node
  let nodeKindMap = new Map();   // id -> kind string
  let kindGroups = new Map();    // kind -> [node, ...]
  let entityDegree = new Map();  // id -> degree count
  let expandedKinds = new Set();
  let hiddenKinds = new Set();   // kinds filtered out by toolbar
  let selectedEntityId = null;   // entity clicked for highlight mode
  let forcedVisible = new Set();
  let showEntityEdges = true;    // edge toggle state
  // Ego mode: growing graph from a seed entity
  let egoCenter = null;          // seed entity id (enables ego mode)
  let egoExpanded = new Set();   // entities whose neighbors are shown
  let egoPositions = new Map();  // id -> {x, y} cached positions
  let pendingFocus = null;       // entity id queued before data loaded
  let transform = { x: 0, y: 0, scale: 1 };
  let dragging = null;
  let panning = false;
  let panStart = { x: 0, y: 0 };
  let visNodes = [];
  let visEdges = [];
  // posCache stores deterministic group positions so they remain stable
  let groupPosCache = new Map();

  // ===== DOM =====
  const svg = document.getElementById('graph-svg');
  const graphRoot = document.getElementById('graph-root');
  const edgesGroup = document.getElementById('edges-group');
  const nodesGroup = document.getElementById('nodes-group');
  const tooltip = document.getElementById('tooltip');
  const loadingEl = document.getElementById('loading-indicator');
  const emptyEl = document.getElementById('empty-state');
  const errorEl = document.getElementById('error-state');
  const errorMsg = document.getElementById('error-message');
  const nodeCountEl = document.getElementById('node-count');
  const filterBarEl = document.getElementById('filter-bar');
  const btnCollapse = document.getElementById('btn-collapse-all');
  const btnZoomIn = document.getElementById('btn-zoom-in');
  const btnZoomOut = document.getElementById('btn-zoom-out');
  const btnFit = document.getElementById('btn-fit');
  const btnRefresh = document.getElementById('btn-refresh');

  // ===== SVG helper =====
  function mkSvg(tag, attrs) {
    const el = document.createElementNS('http://www.w3.org/2000/svg', tag);
    for (const k in attrs) el.setAttribute(k, String(attrs[k]));
    return el;
  }

  function capitalize(s) {
    return s.split('_').map(function(w) { return w[0].toUpperCase() + w.slice(1); }).join(' ');
  }

  function hexRgba(hex, a) {
    const r = parseInt(hex.slice(1, 3), 16);
    const g = parseInt(hex.slice(3, 5), 16);
    const b = parseInt(hex.slice(5, 7), 16);
    return 'rgba(' + r + ',' + g + ',' + b + ',' + a + ')';
  }

  // Compute the intersection of a ray from the center of a rectangle to an
  // external point. Returns the point on the rectangle edge closest to (tx,ty).
  // cx,cy = rect center; hw,hh = half-width, half-height; tx,ty = target point.
  function rectEdgePoint(cx, cy, hw, hh, tx, ty) {
    var dx = tx - cx;
    var dy = ty - cy;
    if (dx === 0 && dy === 0) return { x: cx + hw, y: cy };
    // Scale factors for hitting each edge
    var sx = hw / (Math.abs(dx) || 1);
    var sy = hh / (Math.abs(dy) || 1);
    var s = Math.min(sx, sy);
    return { x: cx + dx * s, y: cy + dy * s };
  }

  // Compute the card height for an ego card based on content
  function egoCardHeight(hasTitle, isCenter) {
    var base = isCenter ? EGO_CH_BASE : EGO_NH_BASE;
    if (hasTitle) base += 14;
    return base;
  }

  // Deterministic hash (djb2)
  function hashStr(s) {
    let h = 5381;
    for (let i = 0; i < s.length; i++) {
      h = ((h << 5) + h + s.charCodeAt(i)) | 0;
    }
    return h;
  }

  // Deterministic pseudo-random [0,1) from integer seed
  function pseudoRand(seed) {
    let x = Math.abs(seed);
    x = ((x * 1103515245 + 12345) & 0x7fffffff);
    return (x % 10000) / 10000;
  }

  // ===== Parse graph data =====
  function parseGraphData(data) {
    allNodes = [];
    allEdges = [];
    nodeMap.clear();
    nodeKindMap.clear();
    kindGroups.clear();
    entityDegree.clear();

    if (!data) {
      emptyEl.classList.remove('hidden');
      return;
    }

    const rawN = data.nodes || data.entities || (data.graph && data.graph.nodes) || [];
    const rawE = data.edges || data.relationships || (data.graph && data.graph.edges) || [];

    for (const n of rawN) {
      const node = {
        id: n.id || n.name,
        kind: (n.kind || n.type || n.entity_kind || 'unknown').toLowerCase(),
        title: n.title || n.description || '',
        file: n.file || n.source_file || '',
        line: n.line || n.source_line || 0,
      };
      allNodes.push(node);
      nodeMap.set(node.id, node);
      nodeKindMap.set(node.id, node.kind);
      if (!kindGroups.has(node.kind)) kindGroups.set(node.kind, []);
      kindGroups.get(node.kind).push(node);
    }

    for (const e of rawE) {
      allEdges.push({
        source: e.source || e.from,
        target: e.target || e.to,
        label: e.label || e.edge_type || e.type || '',
      });
    }

    for (const e of allEdges) {
      entityDegree.set(e.source, (entityDegree.get(e.source) || 0) + 1);
      entityDegree.set(e.target, (entityDegree.get(e.target) || 0) + 1);
    }

    if (allNodes.length === 0) emptyEl.classList.remove('hidden');
  }

  // ===== Filter bar: kind toggle chips =====
  function buildFilterBar() {
    filterBarEl.innerHTML = '';
    const kinds = Array.from(kindGroups.keys()).sort();
    if (kinds.length === 0) {
      filterBarEl.classList.add('hidden');
      return;
    }
    filterBarEl.classList.remove('hidden');
    for (const kind of kinds) {
      const count = kindGroups.get(kind).length;
      const color = KIND_COLORS[kind] || '#888';
      const chip = document.createElement('button');
      chip.className = 'filter-chip' + (hiddenKinds.has(kind) ? ' filter-chip-off' : '');
      chip.dataset.kind = kind;
      chip.title = capitalize(kind) + ' (' + count + ')';
      chip.style.setProperty('--chip-color', color);
      chip.textContent = capitalize(kind) + ' ' + count;
      chip.addEventListener('click', function() {
        if (hiddenKinds.has(kind)) {
          hiddenKinds.delete(kind);
          chip.classList.remove('filter-chip-off');
        } else {
          hiddenKinds.add(kind);
          expandedKinds.delete(kind);
          chip.classList.add('filter-chip-off');
        }
        groupPosCache.clear();
        recompute(false);
      });
      filterBarEl.appendChild(chip);
    }
  }

  // ===== Group layout: deterministic circular/grid for kind groups =====
  function layoutGroups(w, h) {
    const kinds = Array.from(kindGroups.keys()).filter(function(k) { return !hiddenKinds.has(k); }).sort();
    const count = kinds.length;
    if (count === 0) return;

    const cx = w / 2;
    const cy = h / 2;

    // Grid layout — use generous spacing, allow overflow for scroll/zoom
    const minCellW = GW + 40;
    const minCellH = GH + 40;
    const cols = Math.max(2, Math.min(6, Math.ceil(Math.sqrt(count * (w / h)))));
    const rows = Math.ceil(count / cols);
    const cellW = Math.max(minCellW, w / (cols + 1));
    const cellH = Math.max(minCellH, h / (rows + 1));
    const gridW = cols * cellW;
    const gridH = rows * cellH;
    const ox = cx - gridW / 2 + cellW / 2;
    const oy = cy - gridH / 2 + cellH / 2;
    for (let i = 0; i < count; i++) {
      const col = i % cols;
      const row = Math.floor(i / cols);
      const gid = '__grp_' + kinds[i];
      groupPosCache.set(gid, {
        x: ox + col * cellW,
        y: oy + row * cellH,
      });
    }
  }

  // ===== Radial layout for entities within an expanded group =====
  function layoutEntities(groupNode, entities) {
    const count = entities.length;
    if (count === 0) return;

    // Single entity: place right below group
    if (count === 1) {
      entities[0].x = groupNode.x;
      entities[0].y = groupNode.y + GH / 2 + ENTITY_SPACING;
      return;
    }

    // Compute radius so entities have at least ENTITY_SPACING between them
    // circumference = 2*pi*r >= count * spacing
    const minRadius = (count * ENTITY_SPACING) / (2 * Math.PI);
    const radius = Math.max(RADIAL_BASE, minRadius);

    // Sort entities deterministically by hash so positions are stable
    const sorted = entities.slice().sort(function(a, b) {
      return hashStr(a.id) - hashStr(b.id);
    });

    for (let i = 0; i < sorted.length; i++) {
      const angle = (2 * Math.PI * i) / sorted.length - Math.PI / 2;
      sorted[i].x = groupNode.x + radius * Math.cos(angle);
      sorted[i].y = groupNode.y + radius * Math.sin(angle);
    }
  }

  // ===== Ego graph: focused entity + its direct neighbors =====
  function getNeighbors(entityId) {
    const neighbors = new Map(); // id -> { node, label, direction }
    for (const e of allEdges) {
      if (e.source === entityId && nodeMap.has(e.target)) {
        if (!neighbors.has(e.target)) neighbors.set(e.target, { node: nodeMap.get(e.target), labels: [] });
        neighbors.get(e.target).labels.push(e.label);
      }
      if (e.target === entityId && nodeMap.has(e.source)) {
        if (!neighbors.has(e.source)) neighbors.set(e.source, { node: nodeMap.get(e.source), labels: [] });
        neighbors.get(e.source).labels.push(e.label);
      }
    }
    return neighbors;
  }

  function placeNeighborsAround(parentId, parentX, parentY) {
    const neighbors = getNeighbors(parentId);
    const newNeighbors = [];
    for (const [nid, info] of neighbors) {
      if (egoPositions.has(nid)) continue;
      newNeighbors.push({ id: nid, node: info.node });
    }
    if (newNeighbors.length === 0) return;
    newNeighbors.sort(function(a, b) { return hashStr(a.id) - hashStr(b.id); });
    const count = newNeighbors.length;
    // Cards are ~200px wide so we need more spacing than circles
    const radius = Math.max(240, (count * 110) / (2 * Math.PI));
    var baseAngle = pseudoRand(hashStr(parentId)) * 2 * Math.PI;
    for (var i = 0; i < count; i++) {
      var angle = baseAngle + (2 * Math.PI * i) / count;
      egoPositions.set(newNeighbors[i].id, {
        x: parentX + radius * Math.cos(angle),
        y: parentY + radius * Math.sin(angle),
      });
    }
  }

  function computeEgoGraph() {
    visNodes = [];
    visEdges = [];
    const center = nodeMap.get(egoCenter);
    if (!center) return;

    // Ensure center has a position
    if (!egoPositions.has(egoCenter)) {
      var rect = svg.getBoundingClientRect();
      egoPositions.set(egoCenter, { x: (rect.width || 800) / 2, y: (rect.height || 600) / 2 });
    }

    // Place neighbors for center and all expanded nodes
    var centerPos = egoPositions.get(egoCenter);
    placeNeighborsAround(egoCenter, centerPos.x, centerPos.y);
    for (var eid of egoExpanded) {
      var epos = egoPositions.get(eid);
      if (epos) placeNeighborsAround(eid, epos.x, epos.y);
    }

    // Collect all visible entity IDs: center + neighbors of center + neighbors of expanded
    var visibleIds = new Set();
    visibleIds.add(egoCenter);
    var centerNeighbors = getNeighbors(egoCenter);
    for (var nid of centerNeighbors.keys()) visibleIds.add(nid);
    for (var eid of egoExpanded) {
      visibleIds.add(eid);
      var expNeighbors = getNeighbors(eid);
      for (var nid of expNeighbors.keys()) visibleIds.add(nid);
    }

    // Build vis nodes
    for (var id of visibleIds) {
      var node = nodeMap.get(id);
      var pos = egoPositions.get(id);
      if (!node || !pos) continue;
      var isCenter = id === egoCenter;
      var isExpanded = egoExpanded.has(id);
      visNodes.push({
        id: node.id, kind: node.kind, title: node.title,
        file: node.file, line: node.line,
        nodeType: isCenter ? 'egoCenter' : (isExpanded ? 'egoNeighborExp' : 'egoNeighbor'),
        x: pos.x, y: pos.y,
      });
    }

    // Edges between visible entities
    for (var e of allEdges) {
      if (visibleIds.has(e.source) && visibleIds.has(e.target)) {
        visEdges.push({ source: e.source, target: e.target, label: e.label, etype: 'entity' });
      }
    }

    nodeCountEl.textContent = visNodes.length + ' entities | Click to expand neighbors';
  }

  // ===== Compute visible nodes and edges =====
  function computeVisible() {
    visNodes = [];
    visEdges = [];
    const visEntIds = new Set();

    // 1. Kind group nodes (skip hidden kinds)
    for (const [kind, ents] of kindGroups) {
      if (hiddenKinds.has(kind)) continue;
      const expanded = expandedKinds.has(kind);
      const gid = '__grp_' + kind;
      const pos = groupPosCache.get(gid) || { x: 400, y: 300 };
      const groupNode = {
        id: gid,
        kind: kind,
        label: capitalize(kind),
        count: ents.length,
        nodeType: expanded ? 'groupExp' : 'group',
        x: pos.x,
        y: pos.y,
      };
      visNodes.push(groupNode);

      if (expanded) {
        // Select top entities by degree + forced visible
        const sorted = ents.slice().sort(function(a, b) {
          return (entityDegree.get(b.id) || 0) - (entityDegree.get(a.id) || 0);
        });
        const shown = sorted.slice(0, MAX_EXPAND);
        for (const ent of ents) {
          if (forcedVisible.has(ent.id) && !shown.find(function(s) { return s.id === ent.id; })) {
            shown.push(ent);
          }
        }

        // Create entity vis nodes
        const entityVisNodes = [];
        for (const ent of shown) {
          const en = {
            id: ent.id,
            kind: ent.kind,
            title: ent.title,
            file: ent.file,
            line: ent.line,
            nodeType: 'entity',
            x: 0,
            y: 0,
          };
          entityVisNodes.push(en);
          visEntIds.add(ent.id);
        }

        // Deterministic radial layout around the group center
        layoutEntities(groupNode, entityVisNodes);

        for (const en of entityVisNodes) {
          visNodes.push(en);
        }

        // "+N more" badge if there are hidden entities
        if (ents.length > shown.length) {
          const remaining = ents.length - shown.length;
          visNodes.push({
            id: '__badge_' + kind,
            kind: kind,
            label: '+' + remaining,
            nodeType: 'badge',
            x: groupNode.x,
            y: groupNode.y + GH / 2 + 24,
          });
        }
      }
    }

    // 2. Entity-to-entity edges (only when BOTH endpoints are visible)
    let entityEdgeCount = 0;
    for (const e of allEdges) {
      if (visEntIds.has(e.source) && visEntIds.has(e.target)) {
        visEdges.push({
          source: e.source,
          target: e.target,
          label: e.label,
          etype: 'entity',
        });
        entityEdgeCount++;
      }
    }

    // Auto-hide entity edges if too many
    if (entityEdgeCount > EDGE_VISIBILITY_THRESHOLD) {
      showEntityEdges = false;
    }

    // 3. Aggregated group edges (between collapsed groups)
    const groupEdgeCounts = new Map();
    for (const e of allEdges) {
      // Skip edges where both endpoints are visible as entities
      if (visEntIds.has(e.source) && visEntIds.has(e.target)) continue;
      const sk = nodeKindMap.get(e.source);
      const tk = nodeKindMap.get(e.target);
      if (!sk || !tk || sk === tk) continue;
      if (hiddenKinds.has(sk) || hiddenKinds.has(tk)) continue;
      const key = sk + '->' + tk;
      groupEdgeCounts.set(key, (groupEdgeCounts.get(key) || 0) + 1);
    }
    for (const [key, cnt] of groupEdgeCounts) {
      const parts = key.split('->');
      visEdges.push({
        source: '__grp_' + parts[0],
        target: '__grp_' + parts[1],
        label: String(cnt),
        etype: 'group',
        count: cnt,
      });
    }

    const hiddenCount = hiddenKinds.size > 0 ? ' | ' + hiddenKinds.size + ' kinds hidden' : '';
    nodeCountEl.textContent = allNodes.length + ' entities, ' + allEdges.length + ' edges | ' + visNodes.length + ' visible' + hiddenCount;
  }

  // ===== Compute edges connected to a specific entity (for highlight mode) =====
  function getConnectedIds(entityId) {
    const connected = new Set();
    connected.add(entityId);
    for (const e of allEdges) {
      if (e.source === entityId) connected.add(e.target);
      if (e.target === entityId) connected.add(e.source);
    }
    return connected;
  }

  // ===== Render =====
  function render() {
    edgesGroup.innerHTML = '';
    nodesGroup.innerHTML = '';

    const nmap = new Map();
    for (const n of visNodes) nmap.set(n.id, n);

    // Determine highlight set
    const highlightMode = selectedEntityId !== null;
    let connectedIds = null;
    if (highlightMode) {
      connectedIds = getConnectedIds(selectedEntityId);
    }

    // Render group boundaries (faint dashed circles around expanded groups)
    for (const n of visNodes) {
      if (n.nodeType === 'groupExp') {
        // Find max radius of child entities
        let maxDist = 0;
        for (const vn of visNodes) {
          if (vn.nodeType === 'entity' && vn.kind === n.kind) {
            const dx = vn.x - n.x;
            const dy = vn.y - n.y;
            const dist = Math.sqrt(dx * dx + dy * dy);
            if (dist > maxDist) maxDist = dist;
          }
        }
        if (maxDist > 0) {
          const boundaryR = maxDist + ER + 20;
          const color = KIND_COLORS[n.kind] || '#888';
          const circle = mkSvg('circle', {
            cx: n.x,
            cy: n.y,
            r: boundaryR,
            class: 'group-boundary',
            stroke: color,
          });
          if (highlightMode && !connectedIds.has('__grp_' + n.kind)) {
            circle.classList.add('dimmed');
          }
          edgesGroup.appendChild(circle);
        }
      }
    }

    // Render edges
    for (const e of visEdges) {
      const src = nmap.get(e.source);
      const tgt = nmap.get(e.target);
      if (!src || !tgt || src.id === tgt.id) continue;

      const isGroup = e.etype === 'group';
      const isEntity = e.etype === 'entity';

      // Skip entity edges if toggled off
      if (isEntity && !showEntityEdges) continue;

      // Determine if this edge should be dimmed in highlight mode
      let dimEdge = false;
      if (highlightMode) {
        if (isEntity) {
          dimEdge = !connectedIds.has(e.source) || !connectedIds.has(e.target);
        } else {
          // Group edges: dim if neither group contains a connected entity
          dimEdge = true;
        }
      }

      const srcIsGroup = src.nodeType === 'group' || src.nodeType === 'groupExp';
      const tgtIsGroup = tgt.nodeType === 'group' || tgt.nodeType === 'groupExp';
      const srcIsEgo = src.nodeType === 'egoCenter' || src.nodeType === 'egoNeighbor' || src.nodeType === 'egoNeighborExp';
      const tgtIsEgo = tgt.nodeType === 'egoCenter' || tgt.nodeType === 'egoNeighbor' || tgt.nodeType === 'egoNeighborExp';

      // Compute edge start/end points based on node shape
      var x1, y1, x2, y2;
      if (srcIsEgo) {
        var srcCenter = src.nodeType === 'egoCenter';
        var srcHW = (srcCenter ? EGO_CW : EGO_NW) / 2;
        var srcHH = egoCardHeight(!!src.title, srcCenter) / 2;
        var sp = rectEdgePoint(src.x, src.y, srcHW, srcHH, tgt.x, tgt.y);
        x1 = sp.x; y1 = sp.y;
      } else if (srcIsGroup) {
        var dx0 = tgt.x - src.x;
        var dy0 = tgt.y - src.y;
        var d0 = Math.sqrt(dx0 * dx0 + dy0 * dy0) || 1;
        x1 = src.x + (dx0 / d0) * (GW / 2);
        y1 = src.y + (dy0 / d0) * (GW / 2);
      } else {
        var dx0 = tgt.x - src.x;
        var dy0 = tgt.y - src.y;
        var d0 = Math.sqrt(dx0 * dx0 + dy0 * dy0) || 1;
        x1 = src.x + (dx0 / d0) * ER;
        y1 = src.y + (dy0 / d0) * ER;
      }

      if (tgtIsEgo) {
        var tgtCenter = tgt.nodeType === 'egoCenter';
        var tgtHW = (tgtCenter ? EGO_CW : EGO_NW) / 2;
        var tgtHH = egoCardHeight(!!tgt.title, tgtCenter) / 2;
        var tp = rectEdgePoint(tgt.x, tgt.y, tgtHW, tgtHH, src.x, src.y);
        x2 = tp.x; y2 = tp.y;
      } else if (tgtIsGroup) {
        var dx0 = tgt.x - src.x;
        var dy0 = tgt.y - src.y;
        var d0 = Math.sqrt(dx0 * dx0 + dy0 * dy0) || 1;
        x2 = tgt.x - (dx0 / d0) * (GW / 2);
        y2 = tgt.y - (dy0 / d0) * (GW / 2);
      } else {
        var dx0 = tgt.x - src.x;
        var dy0 = tgt.y - src.y;
        var d0 = Math.sqrt(dx0 * dx0 + dy0 * dy0) || 1;
        x2 = tgt.x - (dx0 / d0) * ER;
        y2 = tgt.y - (dy0 / d0) * ER;
      }

      // Quadratic bezier control point (perpendicular offset for curved edges)
      const mx = (x1 + x2) / 2;
      const my = (y1 + y2) / 2;
      const edgeDist = Math.sqrt((x2 - x1) * (x2 - x1) + (y2 - y1) * (y2 - y1));
      // Offset perpendicular to the edge direction; use hash for consistent side
      const curveSign = (hashStr(e.source + e.target) % 2 === 0) ? 1 : -1;
      const curveAmount = isGroup ? edgeDist * 0.08 : edgeDist * 0.12;
      const nx = -(y2 - y1) / (edgeDist || 1);
      const ny = (x2 - x1) / (edgeDist || 1);
      const cpx = mx + nx * curveAmount * curveSign;
      const cpy = my + ny * curveAmount * curveSign;

      const pathD = 'M ' + x1 + ' ' + y1 + ' Q ' + cpx + ' ' + cpy + ' ' + x2 + ' ' + y2;

      const inEgo = !!egoCenter;
      const sw = isGroup ? Math.min(1.5 + Math.log(e.count || 1) * 0.7, 5) : (inEgo ? 1.5 : 1);
      const edgeClass = isGroup ? 'edge-group' : 'edge';

      const pathEl = mkSvg('path', {
        d: pathD,
        class: edgeClass,
        'stroke-width': sw,
        'marker-end': 'url(#arrowhead)',
      });

      if (inEgo && isEntity) {
        pathEl.style.opacity = '0.5';
      }

      if (dimEdge) {
        pathEl.classList.add('dimmed');
      } else if (highlightMode && isEntity) {
        pathEl.classList.add('highlighted-edge');
      }

      edgesGroup.appendChild(pathEl);

      // Invisible hit area for hover
      const hitEl = mkSvg('path', {
        d: pathD,
        class: 'edge-hit',
      });
      hitEl.addEventListener('mouseenter', function(ev) {
        const text = isGroup ? (e.count + ' relationships') : (e.label || '');
        showTooltip(ev.clientX, ev.clientY, text);
        pathEl.classList.add('edge-hover');
      });
      hitEl.addEventListener('mouseleave', function() {
        hideTooltip();
        pathEl.classList.remove('edge-hover');
      });
      edgesGroup.appendChild(hitEl);

      // Edge label in ego mode (show relationship name)
      if (inEgo && isEntity && e.label) {
        const labelEl = mkSvg('text', {
          x: cpx,
          y: cpy - 5,
          class: 'edge-label',
          'text-anchor': 'middle',
        });
        labelEl.textContent = e.label;
        edgesGroup.appendChild(labelEl);
      }

      // Count label for group edges
      if (isGroup && e.count > 1) {
        const labelEl = mkSvg('text', {
          x: cpx,
          y: cpy - 4,
          class: 'edge-count',
          'text-anchor': 'middle',
        });
        labelEl.textContent = String(e.count);
        if (dimEdge) labelEl.classList.add('dimmed');
        edgesGroup.appendChild(labelEl);
      }
    }

    // Edge toggle indicator (shows when entity edges are hidden)
    const entityEdgeCount = visEdges.filter(function(e) { return e.etype === 'entity'; }).length;
    if (entityEdgeCount > 0) {
      renderEdgeToggle(entityEdgeCount);
    }

    // Render nodes
    for (const n of visNodes) {
      const dimNode = highlightMode && n.nodeType === 'entity' && !connectedIds.has(n.id);
      const dimGroup = highlightMode && (n.nodeType === 'group' || n.nodeType === 'groupExp');

      if (n.nodeType === 'group' || n.nodeType === 'groupExp') {
        renderGroup(n, dimGroup);
      } else if (n.nodeType === 'badge') {
        renderBadge(n, highlightMode);
      } else if (n.nodeType === 'egoCenter') {
        renderEgoCenter(n);
      } else if (n.nodeType === 'egoNeighbor' || n.nodeType === 'egoNeighborExp') {
        renderEgoNeighbor(n);
      } else {
        renderEntity(n, dimNode);
      }
    }

    // Ego mode: render "Back" button
    if (egoCenter) {
      renderBackButton();
    }

    applyTransform();
  }

  function renderEgoCenter(n) {
    const color = KIND_COLORS[n.kind] || '#888';
    const hasTitle = !!n.title;
    const cardW = EGO_CW;
    const cardH = egoCardHeight(hasTitle, true);
    const hw = cardW / 2;
    const hh = cardH / 2;
    const deg = entityDegree.get(n.id) || 0;

    const g = mkSvg('g', { class: 'ego-card ego-card--center', transform: 'translate(' + n.x + ',' + n.y + ')' });

    // Card background (centered on node position)
    const bg = mkSvg('rect', {
      x: -hw, y: -hh, width: cardW, height: cardH,
      rx: EGO_CR, ry: EGO_CR,
      class: 'ego-card-bg',
      stroke: color,
    });
    g.appendChild(bg);

    // Colored header strip
    const header = mkSvg('rect', {
      x: -hw, y: -hh, width: cardW, height: EGO_HEADER_H,
      rx: EGO_CR, ry: EGO_CR,
      class: 'ego-card-header',
      fill: color, opacity: 0.85,
    });
    g.appendChild(header);
    // Cover bottom corners of header so only top is rounded
    const headerPatch = mkSvg('rect', {
      x: -hw, y: -hh + EGO_HEADER_H - EGO_CR,
      width: cardW, height: EGO_CR,
      fill: color, opacity: 0.85,
      class: 'ego-card-header',
    });
    g.appendChild(headerPatch);

    // Kind label in header
    const kindLabel = mkSvg('text', {
      x: -hw + 8, y: -hh + 13,
      class: 'ego-card-kind',
    });
    kindLabel.textContent = capitalize(n.kind);
    g.appendChild(kindLabel);

    // Entity ID
    var yPos = -hh + EGO_HEADER_H + 14;
    const idLabel = mkSvg('text', {
      x: -hw + 8, y: yPos,
      class: 'ego-card-id',
    });
    idLabel.textContent = n.id;
    g.appendChild(idLabel);

    // Title (if present)
    if (hasTitle) {
      yPos += 14;
      const titleLabel = mkSvg('text', {
        x: -hw + 8, y: yPos,
        class: 'ego-card-title',
      });
      var displayTitle = n.title.length > 28 ? n.title.slice(0, 26) + '..' : n.title;
      titleLabel.textContent = displayTitle;
      g.appendChild(titleLabel);
    }

    // Connection count
    yPos += 12;
    const metaLabel = mkSvg('text', {
      x: -hw + 8, y: yPos,
      class: 'ego-card-meta',
    });
    metaLabel.textContent = deg + ' connection' + (deg !== 1 ? 's' : '');
    g.appendChild(metaLabel);

    g.addEventListener('dblclick', function(ev) {
      ev.stopPropagation();
      var nd = nodeMap.get(n.id); vscode.postMessage({ type: 'openEntity', file: nd && nd.file, line: nd && nd.line });
    });
    g.addEventListener('mouseenter', function(ev) {
      showTooltip(ev.clientX, ev.clientY, n.kind + ': ' + n.id + (n.title ? '\\n' + n.title : '') + '\\nDouble-click: go to definition');
    });
    g.addEventListener('mouseleave', hideTooltip);
    nodesGroup.appendChild(g);
  }

  function renderEgoLabel(n) {
    const g = mkSvg('g', { transform: 'translate(' + n.x + ',' + n.y + ')' });
    const color = KIND_COLORS[n.kind] || '#888';
    const t = mkSvg('text', { y: 0, 'text-anchor': 'middle', class: 'group-count' });
    t.textContent = n.label;
    t.style.fill = color; t.style.fontSize = '10px'; t.style.fontWeight = '600'; t.style.opacity = '0.5';
    g.appendChild(t);
    nodesGroup.appendChild(g);
  }

  function renderEgoNeighbor(n) {
    const color = KIND_COLORS[n.kind] || '#888';
    const hasTitle = !!n.title;
    const cardW = EGO_NW;
    const cardH = egoCardHeight(hasTitle, false);
    const hw = cardW / 2;
    const hh = cardH / 2;
    const isExp = egoExpanded.has(n.id);
    const deg = entityDegree.get(n.id) || 0;

    const g = mkSvg('g', { class: 'ego-card', transform: 'translate(' + n.x + ',' + n.y + ')' });

    // Card background
    const bg = mkSvg('rect', {
      x: -hw, y: -hh, width: cardW, height: cardH,
      rx: EGO_NR, ry: EGO_NR,
      class: 'ego-card-bg' + (isExp ? ' expanded' : ''),
      stroke: color,
    });
    g.appendChild(bg);

    // Colored header strip
    const header = mkSvg('rect', {
      x: -hw, y: -hh, width: cardW, height: EGO_HEADER_H,
      rx: EGO_NR, ry: EGO_NR,
      class: 'ego-card-header',
      fill: color, opacity: 0.75,
    });
    g.appendChild(header);
    // Patch to square off bottom corners of header
    const headerPatch = mkSvg('rect', {
      x: -hw, y: -hh + EGO_HEADER_H - EGO_NR,
      width: cardW, height: EGO_NR,
      fill: color, opacity: 0.75,
      class: 'ego-card-header',
    });
    g.appendChild(headerPatch);

    // Kind label in header
    const kindLabel = mkSvg('text', {
      x: -hw + 7, y: -hh + 13,
      class: 'ego-card-kind',
    });
    kindLabel.textContent = capitalize(n.kind);
    g.appendChild(kindLabel);

    // Expanded indicator in header (right side)
    if (isExp) {
      const expInd = mkSvg('text', {
        x: hw - 8, y: -hh + 13,
        'text-anchor': 'end',
        class: 'ego-card-kind',
      });
      expInd.textContent = '\\u25BC'; // down-pointing triangle
      g.appendChild(expInd);
    }

    // Entity ID
    var yPos = -hh + EGO_HEADER_H + 13;
    const idLabel = mkSvg('text', {
      x: -hw + 7, y: yPos,
      class: 'ego-card-id',
    });
    idLabel.textContent = n.id;
    g.appendChild(idLabel);

    // Title (if present)
    if (hasTitle) {
      yPos += 13;
      const titleLabel = mkSvg('text', {
        x: -hw + 7, y: yPos,
        class: 'ego-card-title',
      });
      var displayTitle = n.title.length > 24 ? n.title.slice(0, 22) + '..' : n.title;
      titleLabel.textContent = displayTitle;
      g.appendChild(titleLabel);
    }

    // Connection count
    yPos += 11;
    const metaLabel = mkSvg('text', {
      x: -hw + 7, y: yPos,
      class: 'ego-card-meta',
    });
    metaLabel.textContent = deg + ' connection' + (deg !== 1 ? 's' : '');
    g.appendChild(metaLabel);

    g.addEventListener('mouseenter', function(ev) {
      const tip = n.kind + ': ' + n.id + (n.title ? '\\n' + n.title : '') +
        '\\nClick: ' + (isExp ? 'collapse' : 'expand') + ' neighbors' +
        '\\nDouble-click: go to definition';
      showTooltip(ev.clientX, ev.clientY, tip);
    });
    g.addEventListener('mouseleave', hideTooltip);
    g.addEventListener('click', function(ev) {
      ev.stopPropagation();
      if (dragging && dragging.moved) return;
      expandEgoNode(n.id);
    });
    g.addEventListener('dblclick', function(ev) {
      ev.stopPropagation();
      var nd = nodeMap.get(n.id); vscode.postMessage({ type: 'openEntity', file: nd && nd.file, line: nd && nd.line });
    });
    g.addEventListener('mousedown', function(ev) {
      if (ev.button === 0) {
        ev.stopPropagation();
        dragging = { node: n, startX: ev.clientX, startY: ev.clientY, origX: n.x, origY: n.y, moved: false };
      }
    });
    nodesGroup.appendChild(g);
  }

  function renderBackButton() {
    const g = mkSvg('g', { class: 'edge-toggle' });
    const bx = (-transform.x + 10) / transform.scale;
    const by = (-transform.y + 44) / transform.scale;
    g.setAttribute('transform', 'translate(' + bx + ',' + by + ')');
    const label = '\\u2190 Back to overview';
    const tw = 110;
    const rect = mkSvg('rect', { x: 0, y: -12, width: tw, height: 24, rx: 4, fill: 'rgba(66,133,244,0.2)', stroke: 'rgba(66,133,244,0.5)', 'stroke-width': 1 });
    const text = mkSvg('text', { x: tw / 2, y: 1 });
    text.textContent = label;
    g.appendChild(rect); g.appendChild(text);
    g.addEventListener('click', function(ev) { ev.stopPropagation(); exitEgoMode(); });
    nodesGroup.appendChild(g);
  }

  function renderEdgeToggle(count) {
    // Place a small toggle button in the top-left of the graph
    const g = mkSvg('g', { class: 'edge-toggle' });
    // Position in graph coordinates near top-left, accounting for transform
    const bx = (-transform.x + 10) / transform.scale;
    const by = (-transform.y + 44) / transform.scale;
    g.setAttribute('transform', 'translate(' + bx + ',' + by + ')');

    const label = showEntityEdges ? 'Hide ' + count + ' edges' : 'Show ' + count + ' edges';
    const textWidth = label.length * 5.5 + 16;
    const rect = mkSvg('rect', {
      x: 0,
      y: -10,
      width: textWidth,
      height: 20,
      rx: 4,
      fill: showEntityEdges ? 'rgba(128,128,128,0.2)' : 'rgba(66,133,244,0.2)',
      stroke: showEntityEdges ? 'rgba(128,128,128,0.4)' : 'rgba(66,133,244,0.5)',
      'stroke-width': 1,
    });
    const text = mkSvg('text', {
      x: textWidth / 2,
      y: 0,
    });
    text.textContent = label;

    g.appendChild(rect);
    g.appendChild(text);
    g.addEventListener('click', function(ev) {
      ev.stopPropagation();
      showEntityEdges = !showEntityEdges;
      render();
    });
    nodesGroup.appendChild(g);
  }

  function renderGroup(n, dimmed) {
    const g = mkSvg('g', {
      class: 'group-node',
      transform: 'translate(' + n.x + ',' + n.y + ')',
    });
    if (dimmed) g.style.opacity = '0.25';

    const color = KIND_COLORS[n.kind] || '#888';
    const expanded = n.nodeType === 'groupExp';

    const rect = mkSvg('rect', {
      x: -GW / 2,
      y: -GH / 2,
      width: GW,
      height: GH,
      rx: GRX,
      ry: GRX,
      fill: expanded ? hexRgba(color, 0.06) : hexRgba(color, 0.18),
      stroke: color,
      'stroke-width': expanded ? 1.5 : 2,
      'stroke-dasharray': expanded ? '6,3' : 'none',
    });
    if (n.id === selectedEntityId) {
      rect.setAttribute('class', 'selected');
    }

    // Kind abbreviation in top-left
    const abbr = mkSvg('text', {
      x: -GW / 2 + 10,
      y: -GH / 2 + 15,
      class: 'group-kind-abbr',
    });
    abbr.textContent = n.kind.slice(0, 3).toUpperCase();
    abbr.style.fill = color;

    // Toggle icon in top-right
    const toggleIcon = mkSvg('text', {
      x: GW / 2 - 14,
      y: -GH / 2 + 15,
      class: 'group-toggle',
      'text-anchor': 'middle',
    });
    toggleIcon.textContent = expanded ? '\\u25B4' : '\\u25BE';
    toggleIcon.style.fill = color;

    // Name label
    const label = mkSvg('text', {
      y: 2,
      'text-anchor': 'middle',
      class: 'group-text',
    });
    label.textContent = n.label;
    label.style.fill = color;

    // Count
    const cnt = mkSvg('text', {
      y: 20,
      'text-anchor': 'middle',
      class: 'group-count',
    });
    cnt.textContent = n.count + (expanded ? ' expanded' : ' entities');

    g.appendChild(rect);
    g.appendChild(abbr);
    g.appendChild(toggleIcon);
    g.appendChild(label);
    g.appendChild(cnt);

    g.addEventListener('mouseenter', function(ev) {
      showTooltip(ev.clientX, ev.clientY, capitalize(n.kind) + ': ' + n.count + ' entities\\nClick to ' + (expanded ? 'collapse' : 'expand'));
    });
    g.addEventListener('mouseleave', function() {
      hideTooltip();
    });
    g.addEventListener('click', function(ev) {
      ev.stopPropagation();
      if (dragging && dragging.moved) return;
      toggleKind(n.kind);
    });
    g.addEventListener('mousedown', function(ev) {
      if (ev.button === 0) {
        ev.stopPropagation();
        dragging = { node: n, startX: ev.clientX, startY: ev.clientY, origX: n.x, origY: n.y, moved: false };
      }
    });

    nodesGroup.appendChild(g);
  }

  function renderEntity(n, dimmed) {
    const g = mkSvg('g', {
      class: 'entity-node',
      transform: 'translate(' + n.x + ',' + n.y + ')',
    });
    if (dimmed) g.classList.add('dimmed');

    const color = KIND_COLORS[n.kind] || '#888';
    const isSel = n.id === selectedEntityId;

    const circle = mkSvg('circle', {
      r: ER,
      fill: color,
      class: 'entity-circle' + (isSel ? ' selected' : ''),
      opacity: 0.85,
    });

    // Single letter abbreviation
    const abbr = mkSvg('text', {
      y: 4,
      'text-anchor': 'middle',
      class: 'entity-kind-abbr',
    });
    abbr.textContent = (n.kind || '?')[0].toUpperCase();

    // Label below
    const lbl = mkSvg('text', {
      y: ER + 14,
      'text-anchor': 'middle',
      class: 'entity-label',
    });
    const displayId = n.id.length > 20 ? n.id.slice(0, 18) + '..' : n.id;
    lbl.textContent = displayId;

    g.appendChild(circle);
    g.appendChild(abbr);
    g.appendChild(lbl);

    g.addEventListener('mouseenter', function(ev) {
      showTooltip(
        ev.clientX, ev.clientY,
        n.kind + ': ' + n.id +
        (n.title ? '\\n' + n.title : '') +
        '\\nClick: highlight connections' +
        '\\nDouble-click: focus entity graph'
      );
    });
    g.addEventListener('mouseleave', function() {
      hideTooltip();
    });
    g.addEventListener('click', function(ev) {
      ev.stopPropagation();
      if (dragging && dragging.moved) return;
      // Toggle highlight mode
      if (selectedEntityId === n.id) {
        selectedEntityId = null;
      } else {
        selectedEntityId = n.id;
      }
      render();
    });
    g.addEventListener('dblclick', function(ev) {
      ev.stopPropagation();
      enterEgoMode(n.id);
    });
    g.addEventListener('mousedown', function(ev) {
      if (ev.button === 0) {
        ev.stopPropagation();
        dragging = { node: n, startX: ev.clientX, startY: ev.clientY, origX: n.x, origY: n.y, moved: false };
      }
    });

    nodesGroup.appendChild(g);
  }

  function renderBadge(n, dimmed) {
    const g = mkSvg('g', {
      class: 'badge-node',
      transform: 'translate(' + n.x + ',' + n.y + ')',
    });
    if (dimmed) g.style.opacity = '0.2';

    const color = KIND_COLORS[n.kind] || '#888';
    const rect = mkSvg('rect', {
      x: -28,
      y: -11,
      width: 56,
      height: 22,
      rx: 11,
      ry: 11,
      fill: hexRgba(color, 0.12),
      stroke: color,
      'stroke-width': 1,
      'stroke-dasharray': '3,2',
    });
    const t = mkSvg('text', {
      y: 4,
      'text-anchor': 'middle',
      class: 'badge-text',
    });
    t.textContent = n.label;
    t.style.fill = color;

    g.appendChild(rect);
    g.appendChild(t);

    g.addEventListener('click', function(ev) {
      ev.stopPropagation();
      // Clicking badge could expand more — for now, tooltip
      showTooltip(ev.clientX, ev.clientY, n.label + ' more ' + capitalize(n.kind) + ' entities');
    });
    g.addEventListener('mousedown', function(ev) {
      if (ev.button === 0) {
        ev.stopPropagation();
        dragging = { node: n, startX: ev.clientX, startY: ev.clientY, origX: n.x, origY: n.y, moved: false };
      }
    });

    nodesGroup.appendChild(g);
  }

  // ===== Ego mode entry/exit =====
  function enterEgoMode(entityId) {
    if (!egoCenter) {
      egoCenter = entityId;
      egoExpanded.clear();
      egoPositions.clear();
    }
    egoExpanded.add(entityId);
    selectedEntityId = entityId;
    computeEgoGraph();
    render();
    if (egoExpanded.size === 1) {
      setTimeout(fitToView, 30);
    }
  }

  function expandEgoNode(entityId) {
    if (egoExpanded.has(entityId)) {
      egoExpanded.delete(entityId);
    } else {
      egoExpanded.add(entityId);
    }
    computeEgoGraph();
    render();
  }

  function exitEgoMode() {
    egoCenter = null;
    egoExpanded.clear();
    egoPositions.clear();
    selectedEntityId = null;
    recompute(false);
  }

  // ===== Interaction =====
  function toggleKind(kind) {
    if (expandedKinds.has(kind)) {
      expandedKinds.delete(kind);
      // Deselect any selected entity in this kind
      if (selectedEntityId) {
        const selKind = nodeKindMap.get(selectedEntityId);
        if (selKind === kind) selectedEntityId = null;
      }
    } else {
      expandedKinds.add(kind);
    }
    // Reset entity edge visibility for fresh computation
    showEntityEdges = true;
    recompute(true);
  }

  function collapseAll() {
    egoCenter = null;
    egoExpanded.clear();
    egoPositions.clear();
    expandedKinds.clear();
    forcedVisible.clear();
    selectedEntityId = null;
    showEntityEdges = true;
    groupPosCache.clear();
    recompute(false);
  }

  // isIncremental: true = expand/collapse (preserve viewport)
  //                false = fresh load or collapse-all (fit to view)
  function recompute(isIncremental) {
    const rect = svg.getBoundingClientRect();
    const w = rect.width || 800;
    const h = rect.height || 600;

    if (egoCenter) {
      computeEgoGraph();
      render();
      if (!isIncremental) setTimeout(fitToView, 30);
      return;
    }

    // Layout groups if needed
    if (groupPosCache.size === 0) {
      layoutGroups(w, h);
    }

    computeVisible();
    render();

    if (!isIncremental) {
      setTimeout(fitToView, 30);
    }
  }

  // ===== Zoom / Pan =====
  function applyTransform() {
    graphRoot.setAttribute('transform', 'translate(' + transform.x + ',' + transform.y + ') scale(' + transform.scale + ')');
  }

  function zoom(delta, cx, cy) {
    const factor = delta > 0 ? 1.15 : 1 / 1.15;
    const ns = Math.max(0.05, Math.min(8, transform.scale * factor));
    const ratio = ns / transform.scale;
    transform.x = cx - (cx - transform.x) * ratio;
    transform.y = cy - (cy - transform.y) * ratio;
    transform.scale = ns;
    applyTransform();
  }

  function fitToView() {
    if (visNodes.length === 0) return;
    const rect = svg.getBoundingClientRect();
    const pad = 100;
    let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
    for (const n of visNodes) {
      var marginX, marginY;
      if (n.nodeType === 'group' || n.nodeType === 'groupExp') {
        marginX = GW; marginY = GH;
      } else if (n.nodeType === 'egoCenter') {
        marginX = EGO_CW / 2 + 10; marginY = egoCardHeight(!!n.title, true) / 2 + 10;
      } else if (n.nodeType === 'egoNeighbor' || n.nodeType === 'egoNeighborExp') {
        marginX = EGO_NW / 2 + 10; marginY = egoCardHeight(!!n.title, false) / 2 + 10;
      } else {
        marginX = ER + 30; marginY = ER + 30;
      }
      if (n.x - marginX < minX) minX = n.x - marginX;
      if (n.y - marginY < minY) minY = n.y - marginY;
      if (n.x + marginX > maxX) maxX = n.x + marginX;
      if (n.y + marginY > maxY) maxY = n.y + marginY;
    }
    const gw = maxX - minX;
    const gh = maxY - minY;
    const vw = rect.width - pad * 2;
    const vh = rect.height - pad * 2;
    if (gw <= 0 || gh <= 0) return;
    const scale = Math.min(vw / gw, vh / gh, 2);
    transform.scale = scale;
    transform.x = rect.width / 2 - ((minX + maxX) / 2) * scale;
    transform.y = rect.height / 2 - ((minY + maxY) / 2) * scale;
    applyTransform();
  }

  function scrollToNode(node) {
    const rect = svg.getBoundingClientRect();
    transform.x = rect.width / 2 - node.x * transform.scale;
    transform.y = rect.height / 2 - node.y * transform.scale;
    applyTransform();
  }

  function showTooltip(x, y, text) {
    tooltip.textContent = text;
    tooltip.style.left = (x + 14) + 'px';
    tooltip.style.top = (y - 32) + 'px';
    tooltip.classList.remove('hidden');
  }

  function hideTooltip() {
    tooltip.classList.add('hidden');
  }

  // ===== Global event handlers =====
  svg.addEventListener('wheel', function(e) {
    e.preventDefault();
    const r = svg.getBoundingClientRect();
    zoom(-e.deltaY, e.clientX - r.left, e.clientY - r.top);
  }, { passive: false });

  svg.addEventListener('mousedown', function(e) {
    if (e.button === 0 && !dragging) {
      panning = true;
      panStart = { x: e.clientX - transform.x, y: e.clientY - transform.y };
      svg.style.cursor = 'grabbing';
    }
  });

  // Click on background: deselect
  svg.addEventListener('click', function(e) {
    if (e.target === svg || e.target === graphRoot) {
      if (selectedEntityId !== null) {
        selectedEntityId = null;
        render();
      }
    }
  });

  document.addEventListener('mousemove', function(e) {
    if (panning) {
      transform.x = e.clientX - panStart.x;
      transform.y = e.clientY - panStart.y;
      applyTransform();
    }
    if (dragging) {
      const dx = (e.clientX - dragging.startX) / transform.scale;
      const dy = (e.clientY - dragging.startY) / transform.scale;
      if (Math.abs(dx) > 3 || Math.abs(dy) > 3) dragging.moved = true;
      const newX = dragging.origX + dx;
      const newY = dragging.origY + dy;
      dragging.node.x = newX;
      dragging.node.y = newY;

      // If dragging an ego node, update position cache
      if (egoCenter && egoPositions.has(dragging.node.id)) {
        egoPositions.set(dragging.node.id, { x: newX, y: newY });
      }

      // If dragging a group, update groupPosCache and move its child entities
      if (dragging.node.nodeType === 'group' || dragging.node.nodeType === 'groupExp') {
        groupPosCache.set(dragging.node.id, { x: newX, y: newY });
        // Move all children relative to group movement
        if (!dragging.childOffsets) {
          dragging.childOffsets = [];
          for (const vn of visNodes) {
            if (vn.kind === dragging.node.kind && (vn.nodeType === 'entity' || vn.nodeType === 'badge')) {
              dragging.childOffsets.push({
                node: vn,
                dx: vn.x - dragging.origX,
                dy: vn.y - dragging.origY,
              });
            }
          }
        }
        for (const child of dragging.childOffsets) {
          child.node.x = newX + child.dx;
          child.node.y = newY + child.dy;
        }
      }

      render();
    }
  });

  document.addEventListener('mouseup', function() {
    panning = false;
    dragging = null;
    svg.style.cursor = '';
  });

  document.addEventListener('keydown', function(e) {
    if (e.key === 'Escape') {
      selectedEntityId = null;
      hideTooltip();
      render();
    }
  });

  btnCollapse.addEventListener('click', collapseAll);
  btnZoomIn.addEventListener('click', function() {
    const r = svg.getBoundingClientRect();
    zoom(100, r.width / 2, r.height / 2);
  });
  btnZoomOut.addEventListener('click', function() {
    const r = svg.getBoundingClientRect();
    zoom(-100, r.width / 2, r.height / 2);
  });
  btnFit.addEventListener('click', fitToView);
  btnRefresh.addEventListener('click', function() {
    vscode.postMessage({ type: 'refresh' });
  });

  const resizeObs = new ResizeObserver(function() {
    // Only auto-fit on resize when in initial collapsed state
    if (visNodes.length > 0 && expandedKinds.size === 0) {
      fitToView();
    }
  });
  resizeObs.observe(svg);

  // ===== Message handling =====
  window.addEventListener('message', function(event) {
    const msg = event.data;
    switch (msg.type) {
      case 'graphData': {
        loadingEl.classList.add('hidden');
        emptyEl.classList.add('hidden');
        errorEl.classList.add('hidden');
        parseGraphData(msg.data);
        egoCenter = null;
        egoExpanded.clear();
        egoPositions.clear();
        expandedKinds.clear();
        hiddenKinds.clear();
        forcedVisible.clear();
        selectedEntityId = null;
        showEntityEdges = true;
        groupPosCache.clear();
        buildFilterBar();
        // If a focus was requested before data loaded, apply it now
        if (pendingFocus && nodeMap.has(pendingFocus)) {
          const eid = pendingFocus;
          pendingFocus = null;
          enterEgoMode(eid);
        } else {
          pendingFocus = null;
          recompute(false);
        }
        break;
      }
      case 'loading': {
        loadingEl.classList.remove('hidden');
        emptyEl.classList.add('hidden');
        errorEl.classList.add('hidden');
        break;
      }
      case 'error': {
        loadingEl.classList.add('hidden');
        errorEl.classList.remove('hidden');
        errorMsg.textContent = msg.message || 'Unknown error';
        break;
      }
      case 'focusEntity': {
        if (nodeMap.has(msg.entityId)) {
          enterEgoMode(msg.entityId);
        } else {
          pendingFocus = msg.entityId;
        }
        break;
      }
      case 'configChanged': {
        break;
      }
    }
  });
})();
  </script>
</body>
</html>`;
  }

  private dispose(): void {
    GraphWebviewPanel.instance = undefined;

    for (const d of this.disposables) {
      d.dispose();
    }
    this.disposables = [];
  }
}

function getNonce(): string {
  let text = "";
  const possible =
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
  for (let i = 0; i < 32; i++) {
    text += possible.charAt(Math.floor(Math.random() * possible.length));
  }
  return text;
}
