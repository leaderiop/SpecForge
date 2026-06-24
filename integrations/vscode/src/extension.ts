import * as vscode from "vscode";
import { startClient, stopClient } from "./lsp-client";
import { registerCommands } from "./commands";
import { SpecForgeCodeLensProvider } from "./codelens";
import { registerEntityTree } from "./entity-tree";
import { registerOnboarding } from "./onboarding";
import {
  createStatusBar,
  setStarting,
  setIdle,
  setError,
  dispose as disposeStatusBar,
} from "./status-bar";

export async function activate(
  context: vscode.ExtensionContext
): Promise<void> {
  const statusBar = createStatusBar();
  context.subscriptions.push(statusBar);

  const hasProject = await detectProject();
  await vscode.commands.executeCommand(
    "setContext",
    "specforge:projectDetected",
    hasProject
  );

  const commands = registerCommands(context);
  context.subscriptions.push(...commands);

  const codeLensProvider = new SpecForgeCodeLensProvider();
  context.subscriptions.push(
    vscode.languages.registerCodeLensProvider(
      { language: "specforge" },
      codeLensProvider
    )
  );
  context.subscriptions.push(codeLensProvider);

  // Entity Explorer sidebar
  registerEntityTree(context);

  // First-time onboarding
  registerOnboarding(context);

  setStarting();

  try {
    const client = await startClient(context);
    if (client) {
      setIdle();
      await vscode.commands.executeCommand(
        "setContext",
        "specforge:lspRunning",
        true
      );
      await vscode.commands.executeCommand(
        "setContext",
        "specforge:hasEntities",
        hasProject
      );
    } else {
      setError("Language server not found");
    }
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    setError(message);
    vscode.window.showErrorMessage(
      `SpecForge LSP failed to start: ${message}`
    );
  }

  const configWatcher = vscode.workspace.createFileSystemWatcher(
    "**/specforge.json"
  );
  configWatcher.onDidChange(async () => {
    await vscode.commands.executeCommand(
      "setContext",
      "specforge:projectDetected",
      true
    );
  });
  configWatcher.onDidCreate(async () => {
    await vscode.commands.executeCommand(
      "setContext",
      "specforge:projectDetected",
      true
    );
  });
  configWatcher.onDidDelete(async () => {
    await vscode.commands.executeCommand(
      "setContext",
      "specforge:projectDetected",
      false
    );
  });
  context.subscriptions.push(configWatcher);
}

export async function deactivate(): Promise<void> {
  disposeStatusBar();
  await stopClient();
}

async function detectProject(): Promise<boolean> {
  const folders = vscode.workspace.workspaceFolders;
  if (!folders) return false;

  for (const folder of folders) {
    const configUri = vscode.Uri.joinPath(folder.uri, "specforge.json");
    try {
      await vscode.workspace.fs.stat(configUri);
      return true;
    } catch {
      // specforge.json not found in this folder
    }
  }

  const specFiles = await vscode.workspace.findFiles("**/*.spec", null, 1);
  return specFiles.length > 0;
}
