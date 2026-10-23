import * as path from "path";
import * as vscode from "vscode";

/** The URI of a project file: a relative `file` (a project path) is joined onto the workspace folder. */
export function fileUri(file: string): vscode.Uri {
  const root = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
  return vscode.Uri.file(path.isAbsolute(file) || !root ? file : path.join(root, file));
}
