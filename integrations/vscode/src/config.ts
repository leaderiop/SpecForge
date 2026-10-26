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

export function getGraphLayout(): string {
  return getConfig().get<string>("graph.defaultLayout", "force");
}

export function isGraphAutoUpdate(): boolean {
  return getConfig().get<boolean>("graph.autoUpdate", true);
}

export function isFormatOnSave(): boolean {
  return getConfig().get<boolean>("format.onSave", false);
}

export function isMcpAutoStart(): boolean {
  return getConfig().get<boolean>("mcp.autoStart", true);
}

export function getExplorerGroupBy(): string {
  return getConfig().get<string>("entityExplorer.groupBy", "kind");
}
