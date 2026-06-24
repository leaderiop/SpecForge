import * as vscode from "vscode";
import * as path from "path";
import * as fs from "fs";

export function findCliBinary(): string {
  const config = vscode.workspace.getConfiguration("specforge");
  const lspPath = config.get<string>("lsp.path", "");
  if (lspPath) {
    const dir = path.dirname(lspPath);
    const candidate = path.join(dir, "specforge");
    if (fs.existsSync(candidate)) {
      return candidate;
    }
  }

  const ext = process.platform === "win32" ? ".exe" : "";
  const folders = vscode.workspace.workspaceFolders;
  if (folders) {
    for (const folder of folders) {
      let dir = folder.uri.fsPath;
      for (let i = 0; i < 10; i++) {
        for (const profile of ["debug", "release"]) {
          const candidate = path.join(dir, "target", profile, `specforge${ext}`);
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

  return "specforge";
}
