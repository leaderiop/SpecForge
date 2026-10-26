import * as vscode from "vscode";
import { fileUri } from "./paths";
import { symbolIcon } from "./symbols";
import { GraphNode, SessionGraph, graphChanged, sessionGraph } from "./lsp-client";

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
    public readonly entities: GraphNode[]
  ) {}
}

class EntityItem {
  constructor(
    public readonly entity: GraphNode,
    public readonly kind: string
  ) {}
}

class DetailItem {
  constructor(
    public readonly label: string,
    public readonly value: string
  ) {}
}

// --- Tree Data Provider ---

export class EntityTreeProvider
  implements vscode.TreeDataProvider<TreeElement>
{
  private _onDidChangeTreeData = new vscode.EventEmitter<
    TreeElement | undefined | null
  >();
  readonly onDidChangeTreeData = this._onDidChangeTreeData.event;

  private entities: GraphNode[] = [];
  private groupedByKind: Map<string, GraphNode[]> = new Map();
  private graph: SessionGraph | undefined;
  private errorMessage: string | undefined;

  refresh(): void {
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
    if (!vscode.workspace.workspaceFolders?.length) {
      this.errorMessage = "No workspace folder open.";
      return [];
    }

    try {
      this.graph = await sessionGraph();
      this.entities = this.graph.graph.nodes ?? [];
      this.errorMessage = undefined;
    } catch (err) {
      this.errorMessage = err instanceof Error ? err.message.split("\n")[0] : String(err);
      this.graph = undefined;
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

    const sortedKinds = [...this.groupedByKind.keys()].sort();

    return sortedKinds.map(
      (kind) => new KindGroupItem(kind, this.groupedByKind.get(kind)!)
    );
  }

  // --- Third level: entity fields/references ---

  private async getEntityDetails(
    element: EntityItem
  ): Promise<DetailItem[]> {
    const id = element.entity.id;
    const inspected = element.entity;
    const references = (this.graph?.graph.edges ?? [])
      .filter((e) => e.source === id)
      .map((e) => `${e.label}: ${e.target}`);

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
    if (references.length > 0) {
      details.push(new DetailItem("references", references.join(", ")));
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
    item.iconPath = symbolIcon(this.graph?.kinds[element.kind]);
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

    item.iconPath = symbolIcon(this.graph?.kinds[element.kind]);
    item.contextValue = "entity";
    item.tooltip = entity.title
      ? `${entity.kind}: ${entity.id}\n${entity.title}`
      : `${entity.kind}: ${entity.id}`;

    // Navigate to definition on click
    if (entity.file) {
      const uri = fileUri(entity.file);
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

  // The language server notifies after each rebuild (saved or not, ADR 0268 D2); VS Code asks the
  // tree again only while the view is visible.
  let refreshTimer: ReturnType<typeof setTimeout> | undefined;
  context.subscriptions.push(
    graphChanged(() => {
      if (refreshTimer) {
        clearTimeout(refreshTimer);
      }
      refreshTimer = setTimeout(() => {
        provider.refresh();
        refreshTimer = undefined;
      }, 300);
    })
  );

  return provider;
}
