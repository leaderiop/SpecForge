import * as vscode from "vscode";

let statusBarItem: vscode.StatusBarItem | undefined;

export function createStatusBar(): vscode.StatusBarItem {
  statusBarItem = vscode.window.createStatusBarItem(
    "specforge.status",
    vscode.StatusBarAlignment.Left,
    100
  );
  statusBarItem.name = "SpecForge";
  statusBarItem.command = "specforge.showStats";
  setIdle();
  statusBarItem.show();
  return statusBarItem;
}

export function setStarting(): void {
  if (!statusBarItem) return;
  statusBarItem.text = "$(sync~spin) SpecForge";
  statusBarItem.tooltip = "SpecForge: Starting language server...";
  statusBarItem.backgroundColor = undefined;
}

export function setIdle(): void {
  if (!statusBarItem) return;
  statusBarItem.text = "$(check) SpecForge";
  statusBarItem.tooltip = "SpecForge: Ready";
  statusBarItem.backgroundColor = undefined;
}

export function setError(message: string): void {
  if (!statusBarItem) return;
  statusBarItem.text = "$(error) SpecForge";
  statusBarItem.tooltip = `SpecForge: ${message}`;
  statusBarItem.backgroundColor = new vscode.ThemeColor(
    "statusBarItem.errorBackground"
  );
}

export function updateDiagnostics(
  diagnostics: vscode.DiagnosticCollection
): void {
  if (!statusBarItem) return;

  let errors = 0;
  let warnings = 0;
  diagnostics.forEach((_uri, diags) => {
    for (const d of diags) {
      if (d.severity === vscode.DiagnosticSeverity.Error) errors++;
      else if (d.severity === vscode.DiagnosticSeverity.Warning) warnings++;
    }
  });

  if (errors > 0) {
    statusBarItem.text = `$(error) SpecForge: ${errors} error${errors > 1 ? "s" : ""}`;
    statusBarItem.backgroundColor = new vscode.ThemeColor(
      "statusBarItem.errorBackground"
    );
  } else if (warnings > 0) {
    statusBarItem.text = `$(warning) SpecForge: ${warnings} warning${warnings > 1 ? "s" : ""}`;
    statusBarItem.backgroundColor = new vscode.ThemeColor(
      "statusBarItem.warningBackground"
    );
  } else {
    setIdle();
  }
}

export function dispose(): void {
  statusBarItem?.dispose();
  statusBarItem = undefined;
}
