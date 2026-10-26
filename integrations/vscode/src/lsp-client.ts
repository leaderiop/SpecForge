import * as vscode from "vscode";
import {
  ClientCapabilities,
  FeatureState,
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  StaticFeature,
  SymbolKind as ProtocolSymbolKind,
  TransportKind,
} from "vscode-languageclient/node";
import { serverBinary, serverHint } from "./binary";
import { getLspTrace } from "./config";

let client: LanguageClient | undefined;

const graphChangedEmitter = new vscode.EventEmitter<void>();
/** Fires on each `specforge/graphChanged` (the session rebuilt), across restarts of the client. */
export const graphChanged: vscode.Event<void> = graphChangedEmitter.event;

/** Declares `experimental.specforgeGraphChanged`: only then is the server sending the notification. */
class GraphChangedCapability implements StaticFeature {
  fillClientCapabilities(capabilities: ClientCapabilities): void {
    capabilities.experimental = {
      ...(capabilities.experimental as object | undefined),
      specforgeGraphChanged: true,
    };
  }
  initialize(): void {}
  getState(): FeatureState {
    return { kind: "static" };
  }
  clear(): void {}
}

const kindIcons: Record<string, string> = {
  behavior: "$(symbol-method)",
  feature: "$(star-full)",
  type: "$(symbol-class)",
  event: "$(zap)",
  invariant: "$(shield)",
  port: "$(plug)",
  journey: "$(map)",
  milestone: "$(milestone)",
  module: "$(package)",
  term: "$(book)",
  deliverable: "$(archive)",
  decision: "$(law)",
  constraint: "$(lock)",
  failure_mode: "$(warning)",
  persona: "$(person)",
  channel: "$(broadcast)",
  release: "$(tag)",
  property: "$(beaker)",
  axiom: "$(verified)",
  protocol: "$(git-merge)",
  refinement: "$(layers)",
  process: "$(server-process)",
};

/**
 * Entity header format: `**kind** \`entity_id\``. The server's hover is
 * editor-neutral markdown; the header is the first line that matches, after
 * any diagnostic sections the cursor's position adds above it.
 */
const entityHeaderRegex = /^\*\*(\w+)\*\*\s+`([^`]+)`/;

/**
 * The codicon of each `vscode.SymbolKind` (every SymbolKind has a
 * `symbol-*` codicon), for kinds outside `kindIcons`: the server reports a
 * kind's declared `lsp_icon` as the SymbolKind of its workspace symbols.
 */
const symbolKindIcons: Record<vscode.SymbolKind, string> = {
  [vscode.SymbolKind.File]: "$(symbol-file)",
  [vscode.SymbolKind.Module]: "$(symbol-module)",
  [vscode.SymbolKind.Namespace]: "$(symbol-namespace)",
  [vscode.SymbolKind.Package]: "$(symbol-package)",
  [vscode.SymbolKind.Class]: "$(symbol-class)",
  [vscode.SymbolKind.Method]: "$(symbol-method)",
  [vscode.SymbolKind.Property]: "$(symbol-property)",
  [vscode.SymbolKind.Field]: "$(symbol-field)",
  [vscode.SymbolKind.Constructor]: "$(symbol-constructor)",
  [vscode.SymbolKind.Enum]: "$(symbol-enum)",
  [vscode.SymbolKind.Interface]: "$(symbol-interface)",
  [vscode.SymbolKind.Function]: "$(symbol-function)",
  [vscode.SymbolKind.Variable]: "$(symbol-variable)",
  [vscode.SymbolKind.Constant]: "$(symbol-constant)",
  [vscode.SymbolKind.String]: "$(symbol-string)",
  [vscode.SymbolKind.Number]: "$(symbol-number)",
  [vscode.SymbolKind.Boolean]: "$(symbol-boolean)",
  [vscode.SymbolKind.Array]: "$(symbol-array)",
  [vscode.SymbolKind.Object]: "$(symbol-object)",
  [vscode.SymbolKind.Key]: "$(symbol-key)",
  [vscode.SymbolKind.Null]: "$(symbol-null)",
  [vscode.SymbolKind.EnumMember]: "$(symbol-enum-member)",
  [vscode.SymbolKind.Struct]: "$(symbol-struct)",
  [vscode.SymbolKind.Event]: "$(symbol-event)",
  [vscode.SymbolKind.Operator]: "$(symbol-operator)",
  [vscode.SymbolKind.TypeParameter]: "$(symbol-type-parameter)",
};

/**
 * The icon of an entity of `kind`: the static map for the shipped kinds;
 * otherwise the codicon of the SymbolKind the server reports for the
 * entity's workspace symbol (the symbol named `id` whose container is
 * `kind`). `undefined` when there is none.
 */
async function iconOf(kind: string, id: string): Promise<string | undefined> {
  const known = kindIcons[kind];
  if (known) {
    return known;
  }
  try {
    const symbols = await vscode.commands.executeCommand<
      vscode.SymbolInformation[] | undefined
    >("vscode.executeWorkspaceSymbolProvider", id);
    const symbol = symbols?.find(
      (s) => s.name === id && s.containerName === kind
    );
    return symbol ? symbolKindIcons[symbol.kind] : undefined;
  } catch {
    return undefined;
  }
}

async function enhanceHover(hover: vscode.Hover): Promise<vscode.Hover> {
  // `Hover.contents` is a list; only a MarkdownString (what our LSP
  // returns) is enhanced.
  const contents = hover.contents;
  const index = contents.findIndex((c) => c instanceof vscode.MarkdownString);
  if (index < 0) {
    return hover;
  }
  const markdown = contents[index] as vscode.MarkdownString;

  // Find the entity header on any line: diagnostics under the cursor come
  // first. Capture groups: 1 = kind, 2 = entity id.
  const lines = markdown.value.split("\n");
  const at = lines.findIndex((l) => entityHeaderRegex.test(l));
  const headerMatch = at >= 0 ? lines[at].match(entityHeaderRegex) : null;

  if (headerMatch) {
    const icon = await iconOf(headerMatch[1], headerMatch[2]);
    if (icon) {
      lines[at] = `${icon} ${lines[at]}`;
    }
  }
  let md = lines.join("\n");

  // Add codicons to section headers
  md = md.replace(/\*\*Coverage\*\*/g, "$(beaker) **Coverage**");
  md = md.replace(/\*\*Refers to\*\*/g, "$(link) **Refers to**");
  md = md.replace(/\*\*Referenced by\*\*/g, "$(references) **Referenced by**");
  md = md.replace(/\*\*Fields\*\*/g, "$(symbol-field) **Fields**");
  md = md.replace(/\*\*Diagnostics\*\*/g, "$(warning) **Diagnostics**");

  // Add command link at the bottom (only for entity hovers)
  if (headerMatch) {
    const entityId = headerMatch[2];
    const args = encodeURIComponent(JSON.stringify(entityId));
    md += `\n\n---\n\n[$(graph-line) Show in Graph](command:specforge.focusInGraph?${args})`;
  }

  const enhanced = new vscode.MarkdownString(md);
  // Only the extension's own link may run: a spec title can hold a `command:` link.
  enhanced.isTrusted = { enabledCommands: ["specforge.focusInGraph"] };
  enhanced.supportThemeIcons = true;

  const parts = [...contents];
  parts[index] = enhanced;
  return new vscode.Hover(parts, hover.range);
}

export async function startClient(
  context: vscode.ExtensionContext
): Promise<LanguageClient> {
  const traceLevel = getLspTrace();
  const serverOptions: ServerOptions = {
    command: serverBinary(),
    transport: TransportKind.stdio,
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: "file", language: "specforge" }],
    synchronize: {
      fileEvents: vscode.workspace.createFileSystemWatcher("**/*.spec"),
    },
    diagnosticCollectionName: "specforge",
    outputChannelName: "SpecForge",
    traceOutputChannel:
      traceLevel !== "off"
        ? vscode.window.createOutputChannel("SpecForge LSP Trace")
        : undefined,
    middleware: {
      provideHover: async (document, position, token, next) => {
        const result = await next(document, position, token);
        if (!result) {
          return result;
        }
        return await enhanceHover(result);
      },
      // The formatter writes LF line endings, but VS Code keeps a document's own line ending whatever text
      // an edit inserts: a CRLF document is told to switch, so it ends as `specforge format` writes it
      // (ADR 0021 D8).
      provideDocumentFormattingEdits: async (document, options, token, next) => {
        const edits = await next(document, options, token);
        if (!edits || document.eol !== vscode.EndOfLine.CRLF) {
          return edits;
        }
        return [...edits, vscode.TextEdit.setEndOfLine(vscode.EndOfLine.LF)];
      },
    },
  };

  client = new LanguageClient(
    "specforge",
    "SpecForge Language Server",
    serverOptions,
    clientOptions
  );

  client.registerFeature(new GraphChangedCapability());
  client.onNotification("specforge/graphChanged", () => graphChangedEmitter.fire());

  try {
    await client.start();
  } catch (err) {
    client = undefined;
    const reason = err instanceof Error ? err.message : String(err);
    throw new Error(`${reason}. ${serverHint()}`);
  }
  return client;
}

export async function stopClient(): Promise<void> {
  if (client) {
    await client.stop();
    client = undefined;
  }
}

export async function restartClient(
  context: vscode.ExtensionContext
): Promise<LanguageClient> {
  await stopClient();
  return startClient(context);
}

export function getClient(): LanguageClient | undefined {
  return client;
}

/** A node of the Graph Protocol document the language server answers. */
export interface GraphNode {
  id: string;
  kind: string;
  title?: string;
  file?: string;
  line?: number;
  fields?: Record<string, unknown>;
  verify?: unknown[];
}

/** What `specforge/graph` answers, with each kind's symbol kind in VS Code's numbering. */
export interface SessionGraph {
  graph: {
    nodes: GraphNode[];
    edges: { source: string; target: string; label: string }[];
  };
  kinds: Record<string, vscode.SymbolKind>;
  /** Each node's edges to and from other entities (a self-edge counts for nothing). */
  degrees: Record<string, number>;
}

function running(): LanguageClient {
  if (!client) {
    throw new Error("The SpecForge language server is not running.");
  }
  return client;
}

/** `specforge/graph`: the session's graph, open buffers included (ADR 0268 D1). */
export async function sessionGraph(): Promise<SessionGraph> {
  const c = running();
  const reply = await c.sendRequest<
    Omit<SessionGraph, "kinds"> & { kinds: Record<string, ProtocolSymbolKind> }
  >("specforge/graph");
  const kinds: Record<string, vscode.SymbolKind> = {};
  for (const [kind, symbolKind] of Object.entries(reply.kinds)) {
    kinds[kind] = c.protocol2CodeConverter.asSymbolKind(symbolKind);
  }
  return { ...reply, kinds };
}

/** `specforge/entity` at the editor's cursor: the entity it names, else undefined (ADR 0268 D3). */
export async function entityAt(
  editor: vscode.TextEditor
): Promise<GraphNode | undefined> {
  const c = running();
  const node = await c.sendRequest<GraphNode | null>(
    "specforge/entity",
    c.code2ProtocolConverter.asTextDocumentPositionParams(
      editor.document,
      editor.selection.active
    )
  );
  return node ?? undefined;
}
