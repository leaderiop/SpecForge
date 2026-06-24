import * as vscode from "vscode";
import * as path from "path";
import * as fs from "fs";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from "vscode-languageclient/node";
import { getLspPath, getLspTrace } from "./config";

let client: LanguageClient | undefined;

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

/** Regex to detect entity header format: **kind** `entity_id`... */
const entityHeaderRegex = /^\*\*(\w+)\*\*\s+`([^`]+)`/;

function enhanceHover(hover: vscode.Hover): vscode.Hover {
  const contents = hover.contents;

  // Only handle MarkdownString (what our LSP returns)
  if (!(contents instanceof vscode.MarkdownString)) {
    return hover;
  }

  let md = contents.value;

  // Detect if this is an entity hover by checking the first line
  const firstLine = md.split("\n")[0];
  const headerMatch = firstLine.match(entityHeaderRegex);
  const isEntityHover = headerMatch !== null;

  // Add kind icon before the header line (only for entity hovers with a known kind)
  if (isEntityHover) {
    const kind = headerMatch[1];
    const icon = kindIcons[kind];
    if (icon) {
      md = `${icon} ${md}`;
    }
  }

  // Add codicons to section headers
  md = md.replace(/\*\*References\*\*/g, "$(link) **References**");
  md = md.replace(/\*\*Referenced by\*\*/g, "$(references) **Referenced by**");
  md = md.replace(/\*\*Fields\*\*/g, "$(symbol-field) **Fields**");

  // Add command link at the bottom (only for entity hovers)
  if (isEntityHover) {
    const entityId = headerMatch[2];
    const args = encodeURIComponent(JSON.stringify(entityId));
    md += `\n\n---\n\n[$(graph-line) Show in Graph](command:specforge.focusInGraph?${args})`;
  }

  const enhanced = new vscode.MarkdownString(md);
  enhanced.isTrusted = true;
  enhanced.supportThemeIcons = true;

  return new vscode.Hover(enhanced, hover.range);
}

function resolveBinaryPath(): string | undefined {
  // 1. User setting
  const configPath = getLspPath();
  if (configPath && fs.existsSync(configPath)) {
    return configPath;
  }

  // 2. Bundled platform-specific binary
  const platformMap: Record<string, string> = {
    "darwin-arm64": "darwin-arm64",
    "darwin-x64": "darwin-x64",
    "linux-x64": "linux-x64",
    "linux-arm64": "linux-arm64",
    "win32-x64": "win32-x64",
  };
  const platformKey = `${process.platform}-${process.arch}`;
  const platformDir = platformMap[platformKey];
  const ext = process.platform === "win32" ? ".exe" : "";

  if (platformDir) {
    const bundledPath = path.join(
      __dirname,
      "..",
      "bin",
      platformDir,
      `specforge-lsp${ext}`
    );
    if (fs.existsSync(bundledPath)) {
      return bundledPath;
    }
  }

  // 3. Cargo build output (dev mode — walk up from workspace to find target/)
  const folders = vscode.workspace.workspaceFolders;
  if (folders) {
    for (const folder of folders) {
      let dir = folder.uri.fsPath;
      for (let i = 0; i < 10; i++) {
        for (const profile of ["debug", "release"]) {
          const candidate = path.join(dir, "target", profile, `specforge-lsp${ext}`);
          if (fs.existsSync(candidate)) {
            return candidate;
          }
        }
        const parent = path.dirname(dir);
        if (parent === dir) break;
        dir = parent;
      }
    }
  }

  // 4. Fall back to PATH — will fail with ENOENT if not installed
  return undefined;
}

export async function startClient(
  context: vscode.ExtensionContext
): Promise<LanguageClient | undefined> {
  const binaryPath = resolveBinaryPath();
  if (!binaryPath) {
    vscode.window.showErrorMessage(
      "SpecForge language server not found. Install the CLI or set specforge.lsp.path."
    );
    return undefined;
  }

  const traceLevel = getLspTrace();
  const args: string[] = [];
  if (traceLevel === "verbose") {
    args.push("--log-level=debug");
  }

  const serverOptions: ServerOptions = {
    run: { command: binaryPath, args, transport: TransportKind.stdio },
    debug: {
      command: binaryPath,
      args: [...args, "--log-level=debug"],
      transport: TransportKind.stdio,
    },
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
        return enhanceHover(result);
      },
    },
  };

  client = new LanguageClient(
    "specforge",
    "SpecForge Language Server",
    serverOptions,
    clientOptions
  );

  await client.start();
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
): Promise<LanguageClient | undefined> {
  await stopClient();
  return startClient(context);
}

export function getClient(): LanguageClient | undefined {
  return client;
}
