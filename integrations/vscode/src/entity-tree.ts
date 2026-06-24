import * as vscode from "vscode";
import * as cp from "child_process";
import { findCliBinary } from "./find-binary";

// --- Types for CLI output ---

interface ListEntity {
  id: string;
  kind: string;
  title?: string;
  file?: string;
  line?: number;
}

interface InspectEntity {
  id: string;
  kind: string;
  title?: string;
  file?: string;
  line?: number;
  fields?: Record<string, unknown>;
  references?: string[];
}

// --- Codicon mapping per entity kind ---

const KIND_ICONS: Record<string, string> = {
  behavior: "symbol-method",
  feature: "symbol-class",
  event: "symbol-event",
  type: "symbol-struct",
  invariant: "symbol-interface",
  port: "symbol-interface",
  journey: "symbol-misc",
  deliverable: "symbol-package",
  milestone: "symbol-ruler",
  module: "symbol-namespace",
  term: "symbol-text",
  persona: "symbol-account",
  channel: "symbol-broadcast",
  release: "symbol-tag",
  decision: "symbol-law",
  constraint: "symbol-lock",
  failure_mode: "symbol-warning",
  property: "symbol-property",
  axiom: "symbol-key",
  protocol: "symbol-link",
  refinement: "symbol-type-hierarchy",
  process: "symbol-debug-step-over",
};

// Canonical ordering for entity kind groups in the tree
const KIND_ORDER: string[] = [
  "behavior",
  "feature",
  "event",
  "type",
  "invariant",
  "port",
  "journey",
  "deliverable",
  "milestone",
  "module",
  "term",
  "persona",
  "channel",
  "release",
  "decision",
  "constraint",
  "failure_mode",
  "property",
  "axiom",
  "protocol",
  "refinement",
  "process",
];

function kindSortIndex(kind: string): number {
  const idx = KIND_ORDER.indexOf(kind);
  return idx >= 0 ? idx : KIND_ORDER.length;
}

function pluralizeKind(kind: string): string {
  if (kind.endsWith("y") && !kind.endsWith("ey")) {
    return kind.slice(0, -1) + "ies";
  }
  if (kind.endsWith("s") || kind.endsWith("ss")) {
    return kind + "es";
  }
  return kind + "s";
}

function formatKindLabel(kind: string): string {
  const plural = pluralizeKind(kind);
  return plural
    .split("_")
    .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
    .join(" ");
}

// --- Tree item types ---

type TreeElement = KindGroupItem | EntityItem | DetailItem;

class KindGroupItem {
  constructor(
    public readonly kind: string,
    public readonly entities: ListEntity[]
  ) {}
}

class EntityItem {
  constructor(
    public readonly entity: ListEntity,
    public readonly kind: string
  ) {}
}

class DetailItem {
  constructor(
    public readonly label: string,
    public readonly value: string
  ) {}
}

// --- CLI execution helper ---

function execCli(args: string[], cwd: string): Promise<string> {
  return new Promise((resolve, reject) => {
    cp.execFile(
      findCliBinary(),
      args,
      { cwd, timeout: 30_000 },
      (error, stdout, stderr) => {
        if (error) {
          reject(new Error(stderr || error.message));
        } else {
          resolve(stdout);
        }
      }
    );
  });
}

function getWorkspaceCwd(): string | undefined {
  return vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
}

// --- Tree Data Provider ---

export class EntityTreeProvider
  implements vscode.TreeDataProvider<TreeElement>
{
  private _onDidChangeTreeData = new vscode.EventEmitter<
    TreeElement | undefined | null
  >();
  readonly onDidChangeTreeData = this._onDidChangeTreeData.event;

  private entities: ListEntity[] = [];
  private groupedByKind: Map<string, ListEntity[]> = new Map();
  private inspectCache: Map<string, InspectEntity> = new Map();
  private errorMessage: string | undefined;

  refresh(): void {
    this.inspectCache.clear();
    this._onDidChangeTreeData.fire(undefined);
  }

  getTreeItem(element: TreeElement): vscode.TreeItem {
    if (element instanceof KindGroupItem) {
      return this.toKindGroupTreeItem(element);
    }
    if (element instanceof EntityItem) {
      return this.toEntityTreeItem(element);
    }
    return this.toDetailTreeItem(element);
  }

  async getChildren(element?: TreeElement): Promise<TreeElement[]> {
    if (!element) {
      return this.getRootChildren();
    }
    if (element instanceof KindGroupItem) {
      return element.entities.map((e) => new EntityItem(e, element.kind));
    }
    if (element instanceof EntityItem) {
      return this.getEntityDetails(element);
    }
    return [];
  }

  // --- Root level: kind groups ---

  private async getRootChildren(): Promise<TreeElement[]> {
    const cwd = getWorkspaceCwd();
    if (!cwd) {
      this.errorMessage = "No workspace folder open.";
      return [];
    }

    try {
      const output = await execCli(["list", "--format=json"], cwd);
      this.entities = JSON.parse(output) as ListEntity[];
      this.errorMessage = undefined;
    } catch {
      this.errorMessage = "Run specforge check first";
      this.entities = [];
    }

    await vscode.commands.executeCommand(
      "setContext",
      "specforge:hasEntities",
      this.entities.length > 0
    );

    if (this.entities.length === 0) {
      return [];
    }

    this.groupedByKind = new Map();
    for (const entity of this.entities) {
      const kind = entity.kind.toLowerCase();
      const group = this.groupedByKind.get(kind);
      if (group) {
        group.push(entity);
      } else {
        this.groupedByKind.set(kind, [entity]);
      }
    }

    const sortedKinds = [...this.groupedByKind.keys()].sort(
      (a, b) => kindSortIndex(a) - kindSortIndex(b)
    );

    return sortedKinds.map(
      (kind) => new KindGroupItem(kind, this.groupedByKind.get(kind)!)
    );
  }

  // --- Third level: entity fields/references ---

  private async getEntityDetails(
    element: EntityItem
  ): Promise<DetailItem[]> {
    const cwd = getWorkspaceCwd();
    if (!cwd) return [];

    const id = element.entity.id;
    let inspected = this.inspectCache.get(id);

    if (!inspected) {
      try {
        const output = await execCli(
          ["inspect", id, "--format=json"],
          cwd
        );
        inspected = JSON.parse(output) as InspectEntity;
        this.inspectCache.set(id, inspected);
      } catch {
        return [new DetailItem("(inspect failed)", "")];
      }
    }

    const details: DetailItem[] = [];

    // Show fields
    if (inspected.fields) {
      for (const [key, value] of Object.entries(inspected.fields)) {
        const displayValue = Array.isArray(value)
          ? value.join(", ")
          : String(value ?? "");
        if (displayValue) {
          details.push(new DetailItem(key, displayValue));
        }
      }
    }

    // Show references
    if (inspected.references && inspected.references.length > 0) {
      details.push(
        new DetailItem("references", inspected.references.join(", "))
      );
    }

    if (details.length === 0) {
      details.push(new DetailItem("(no fields)", ""));
    }

    return details;
  }

  // --- TreeItem factories ---

  private toKindGroupTreeItem(element: KindGroupItem): vscode.TreeItem {
    const label = `${formatKindLabel(element.kind)} (${element.entities.length})`;
    const item = new vscode.TreeItem(
      label,
      vscode.TreeItemCollapsibleState.Collapsed
    );
    const iconId = KIND_ICONS[element.kind] ?? "symbol-misc";
    item.iconPath = new vscode.ThemeIcon(iconId);
    item.contextValue = "entityKindGroup";
    return item;
  }

  private toEntityTreeItem(element: EntityItem): vscode.TreeItem {
    const entity = element.entity;
    const label = entity.id;
    const item = new vscode.TreeItem(
      label,
      vscode.TreeItemCollapsibleState.Collapsed
    );

    if (entity.title) {
      item.description = entity.title;
    }

    const iconId = KIND_ICONS[element.kind] ?? "symbol-misc";
    item.iconPath = new vscode.ThemeIcon(iconId);
    item.contextValue = "entity";
    item.tooltip = entity.title
      ? `${entity.kind}: ${entity.id}\n${entity.title}`
      : `${entity.kind}: ${entity.id}`;

    // Navigate to definition on click
    if (entity.file) {
      const uri = vscode.Uri.file(entity.file);
      const line = entity.line ? Math.max(0, entity.line - 1) : 0;
      item.command = {
        command: "vscode.open",
        title: "Go to Definition",
        arguments: [
          uri,
          {
            selection: new vscode.Range(line, 0, line, 0),
            preserveFocus: false,
          } satisfies vscode.TextDocumentShowOptions,
        ],
      };
    }

    return item;
  }

  private toDetailTreeItem(element: DetailItem): vscode.TreeItem {
    const item = new vscode.TreeItem(
      element.label,
      vscode.TreeItemCollapsibleState.None
    );
    item.description = element.value;
    item.iconPath = new vscode.ThemeIcon("dash");
    item.contextValue = "entityDetail";
    return item;
  }

  // --- Public getters for welcome view ---

  get hasError(): boolean {
    return this.errorMessage !== undefined;
  }

  get error(): string | undefined {
    return this.errorMessage;
  }
}

// --- Registration ---

export function registerEntityTree(
  context: vscode.ExtensionContext
): EntityTreeProvider {
  const provider = new EntityTreeProvider();

  const treeView = vscode.window.createTreeView("specforge.entities", {
    treeDataProvider: provider,
    showCollapseAll: true,
  });
  context.subscriptions.push(treeView);

  // Refresh command
  context.subscriptions.push(
    vscode.commands.registerCommand("specforge.refreshEntities", () => {
      provider.refresh();
    })
  );

  // Debounced auto-refresh on .spec file save
  let refreshTimer: ReturnType<typeof setTimeout> | undefined;
  const specWatcher = vscode.workspace.onDidSaveTextDocument((doc) => {
    if (
      doc.languageId === "specforge" ||
      doc.fileName.endsWith(".spec") ||
      doc.fileName.endsWith("specforge.json")
    ) {
      if (refreshTimer) {
        clearTimeout(refreshTimer);
      }
      refreshTimer = setTimeout(() => {
        provider.refresh();
        refreshTimer = undefined;
      }, 500);
    }
  });
  context.subscriptions.push(specWatcher);

  return provider;
}
