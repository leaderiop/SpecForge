import * as vscode from "vscode";
import * as cp from "child_process";
import * as path from "path";
import { getBinDir } from "./config";

type Name = "specforge" | "specforge-lsp";

/**
 * The binary `name` from the machine setting `specforge.binDir`, else by name on PATH
 * (ADR 0249 D3). Nothing in the workspace is ever joined, so no repository chooses a binary.
 */
function binary(name: Name): string {
  const dir = getBinDir();
  return dir ? path.join(dir, name + (process.platform === "win32" ? ".exe" : "")) : name;
}

/** The language server's command. */
export function serverBinary(): string {
  return binary("specforge-lsp");
}

/** Where a missing language server is looked for, and how to install it. */
export function serverHint(): string {
  return "specforge-lsp is looked up in specforge.binDir, else on PATH: install it beside specforge (cargo install --path crates/surfaces/lsp, or the release archive).";
}

function workspaceCwd(): string {
  const folder = vscode.workspace.workspaceFolders?.[0];
  if (!folder) {
    throw new Error("No workspace folder open.");
  }
  return folder.uri.fsPath;
}

/** `specforge <args>` in the workspace folder: its stdout; a refusal's stderr as the error. */
export function cliOutput(args: string[], timeout = 60_000): Promise<string> {
  return new Promise((resolve, reject) => {
    let cwd: string;
    try {
      cwd = workspaceCwd();
    } catch (err) {
      reject(err);
      return;
    }
    cp.execFile(
      binary("specforge"),
      args,
      { cwd, timeout, maxBuffer: 64 * 1024 * 1024 },
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

/** `specforge <args>` as a task in the terminal panel: no shell, so nothing is quoted. */
export function inTask(args: string[]): void {
  const folder = vscode.workspace.workspaceFolders?.[0];
  if (!folder) {
    vscode.window.showErrorMessage("No workspace folder open.");
    return;
  }
  const execution = new vscode.ProcessExecution(binary("specforge"), args, {
    cwd: folder.uri.fsPath,
  });
  vscode.tasks.executeTask(
    new vscode.Task({ type: "specforge" }, folder, `specforge ${args[0]}`, "SpecForge", execution)
  );
}
