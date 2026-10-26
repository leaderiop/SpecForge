import * as vscode from "vscode";

export function getConfig() {
  return vscode.workspace.getConfiguration("specforge");
}

export function getBinDir(): string {
  return getConfig().get<string>("binDir", "");
}

export function getLspTrace(): string {
  return getConfig().get<string>("lsp.trace", "off");
}

export function isGraphAutoUpdate(): boolean {
  return getConfig().get<boolean>("graph.autoUpdate", true);
}
