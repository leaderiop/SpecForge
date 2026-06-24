import * as vscode from "vscode";
import * as cp from "child_process";
import { restartClient } from "./lsp-client";
import { GraphWebviewPanel } from "./graph-webview";
import { findCliBinary } from "./find-binary";

function runCliCommand(args: string[]): void {
  const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
  if (!workspaceFolder) {
    vscode.window.showErrorMessage("No workspace folder open.");
    return;
  }

  const terminal = vscode.window.createTerminal({
    name: "SpecForge",
    cwd: workspaceFolder.uri.fsPath,
  });
  terminal.sendText(`${findCliBinary()} ${args.join(" ")}`);
  terminal.show();
}

async function runCliCommandOutput(args: string[]): Promise<string> {
  const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
  if (!workspaceFolder) {
    throw new Error("No workspace folder open.");
  }

  return new Promise((resolve, reject) => {
    cp.execFile(
      findCliBinary(),
      args,
      { cwd: workspaceFolder.uri.fsPath, timeout: 30_000 },
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

export function registerCommands(
  context: vscode.ExtensionContext
): vscode.Disposable[] {
  return [
    vscode.commands.registerCommand("specforge.init", async () => {
      const extensions = await vscode.window.showQuickPick(
        [
          {
            label: "@specforge/software",
            description: "behavior, invariant, event, type, port",
            picked: true,
          },
          {
            label: "@specforge/product",
            description:
              "feature, journey, deliverable, milestone, module, term, persona, channel, release",
          },
          {
            label: "@specforge/governance",
            description: "decision, constraint, failure_mode",
          },
          {
            label: "@specforge/formal",
            description: "property, axiom, protocol, refinement, process",
          },
        ],
        {
          canPickMany: true,
          placeHolder: "Select extensions to install",
          title: "SpecForge: Initialize Project",
        }
      );
      if (!extensions) return;

      const extArgs = extensions.flatMap((e) => ["--extension", e.label]);
      runCliCommand(["init", ...extArgs]);
    }),

    vscode.commands.registerCommand("specforge.check", () => {
      runCliCommand(["check"]);
    }),

    vscode.commands.registerCommand("specforge.export", async () => {
      const format = await vscode.window.showQuickPick(
        ["graph", "context", "brief"],
        { placeHolder: "Export format" }
      );
      if (!format) return;
      runCliCommand(["export", `--format=${format}`]);
    }),

    vscode.commands.registerCommand("specforge.showStats", async () => {
      try {
        const output = await runCliCommandOutput(["stats", "--format=json"]);
        const doc = await vscode.workspace.openTextDocument({
          content: output,
          language: "json",
        });
        await vscode.window.showTextDocument(doc);
      } catch {
        runCliCommand(["stats"]);
      }
    }),

    vscode.commands.registerCommand("specforge.doctor", () => {
      runCliCommand(["doctor"]);
    }),

    vscode.commands.registerCommand("specforge.restartLsp", async () => {
      await restartClient(context);
      vscode.window.showInformationMessage(
        "SpecForge language server restarted."
      );
    }),

    vscode.commands.registerCommand("specforge.search", async () => {
      const query = await vscode.window.showInputBox({
        placeHolder: "Search entities...",
        prompt: "Enter a search term to find entities",
      });
      if (!query) return;
      try {
        const output = await runCliCommandOutput([
          "search",
          query,
          "--format=json",
        ]);
        const results = JSON.parse(output);
        if (!results.length) {
          vscode.window.showInformationMessage(
            `No entities found for "${query}".`
          );
          return;
        }
        interface SearchItem extends vscode.QuickPickItem {
          file?: string;
        }
        const items: SearchItem[] = results.map(
          (r: { id: string; kind: string; title: string; file: string }) => ({
            label: r.id,
            description: r.kind,
            detail: r.title,
            file: r.file,
          })
        );
        const picked = await vscode.window.showQuickPick(items, {
          placeHolder: "Select an entity to navigate to",
        });
        if (picked?.file) {
          const doc = await vscode.workspace.openTextDocument(picked.file);
          await vscode.window.showTextDocument(doc);
        }
      } catch {
        runCliCommand(["search", query]);
      }
    }),

    vscode.commands.registerCommand("specforge.explainError", async () => {
      const code = await vscode.window.showInputBox({
        placeHolder: "E001",
        prompt: "Enter a diagnostic code to explain",
      });
      if (!code) return;
      try {
        const output = await runCliCommandOutput(["explain", code]);
        const doc = await vscode.workspace.openTextDocument({
          content: output,
          language: "markdown",
        });
        await vscode.window.showTextDocument(doc);
      } catch {
        vscode.window.showErrorMessage(`Could not explain code "${code}".`);
      }
    }),

    vscode.commands.registerCommand("specforge.model", async () => {
      const format = await vscode.window.showQuickPick(
        ["mermaid", "markdown", "dot", "json", "dbml"],
        { placeHolder: "Model visualization format" }
      );
      if (!format) return;
      try {
        const output = await runCliCommandOutput([
          "model",
          `--format=${format}`,
        ]);
        const lang =
          format === "mermaid"
            ? "mermaid"
            : format === "json"
              ? "json"
              : format === "dot"
                ? "dot"
                : "markdown";
        const doc = await vscode.workspace.openTextDocument({
          content: output,
          language: lang,
        });
        await vscode.window.showTextDocument(doc);
      } catch {
        runCliCommand(["model", `--format=${format}`]);
      }
    }),

    vscode.commands.registerCommand("specforge.coverage", () => {
      runCliCommand(["coverage"]);
    }),

    vscode.commands.registerCommand("specforge.outline", async () => {
      try {
        const output = await runCliCommandOutput([
          "outline",
          "--format=json",
        ]);
        const doc = await vscode.workspace.openTextDocument({
          content: output,
          language: "json",
        });
        await vscode.window.showTextDocument(doc);
      } catch {
        runCliCommand(["outline"]);
      }
    }),

    vscode.commands.registerCommand("specforge.addExtension", async () => {
      const ext = await vscode.window.showQuickPick(
        [
          "@specforge/software",
          "@specforge/product",
          "@specforge/governance",
          "@specforge/formal",
        ],
        { placeHolder: "Select extension to add" }
      );
      if (!ext) return;
      runCliCommand(["add", ext]);
    }),

    vscode.commands.registerCommand("specforge.removeExtension", async () => {
      const ext = await vscode.window.showInputBox({
        placeHolder: "@specforge/software",
        prompt: "Extension to remove",
      });
      if (!ext) return;
      runCliCommand(["remove", ext]);
    }),

    vscode.commands.registerCommand("specforge.newFile", async () => {
      const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
      if (!workspaceFolder) return;

      const name = await vscode.window.showInputBox({
        placeHolder: "my-entities",
        prompt: "Name for the new spec file (without .spec extension)",
      });
      if (!name) return;

      const uri = vscode.Uri.joinPath(
        workspaceFolder.uri,
        "spec",
        `${name}.spec`
      );
      await vscode.workspace.fs.writeFile(
        uri,
        Buffer.from(`// ${name}\n\n`)
      );
      const doc = await vscode.workspace.openTextDocument(uri);
      await vscode.window.showTextDocument(doc);
    }),

    vscode.commands.registerCommand("specforge.inspect", async () => {
      const editor = vscode.window.activeTextEditor;
      if (!editor) return;
      const wordRange = editor.document.getWordRangeAtPosition(
        editor.selection.active
      );
      const word = wordRange
        ? editor.document.getText(wordRange)
        : await vscode.window.showInputBox({
            placeHolder: "entity_id",
            prompt: "Entity ID to inspect",
          });
      if (!word) return;
      try {
        const output = await runCliCommandOutput([
          "inspect",
          word,
          "--format=json",
        ]);
        const doc = await vscode.workspace.openTextDocument({
          content: output,
          language: "json",
        });
        await vscode.window.showTextDocument(doc);
      } catch {
        vscode.window.showErrorMessage(`Entity "${word}" not found.`);
      }
    }),

    vscode.commands.registerCommand("specforge.trace", async () => {
      const editor = vscode.window.activeTextEditor;
      if (!editor) return;
      const wordRange = editor.document.getWordRangeAtPosition(
        editor.selection.active
      );
      const word = wordRange
        ? editor.document.getText(wordRange)
        : await vscode.window.showInputBox({
            placeHolder: "entity_id",
            prompt: "Entity ID to trace",
          });
      if (!word) return;
      try {
        const output = await runCliCommandOutput(["trace", word]);
        const doc = await vscode.workspace.openTextDocument({
          content: output,
          language: "markdown",
        });
        await vscode.window.showTextDocument(doc);
      } catch {
        vscode.window.showErrorMessage(`Could not trace "${word}".`);
      }
    }),

    vscode.commands.registerCommand("specforge.copyEntityJson", async () => {
      const editor = vscode.window.activeTextEditor;
      if (!editor) return;
      const wordRange = editor.document.getWordRangeAtPosition(
        editor.selection.active
      );
      const word = wordRange ? editor.document.getText(wordRange) : undefined;
      if (!word) return;
      try {
        const output = await runCliCommandOutput([
          "inspect",
          word,
          "--format=json",
        ]);
        await vscode.env.clipboard.writeText(output);
        vscode.window.showInformationMessage(`Copied ${word} JSON to clipboard.`);
      } catch {
        vscode.window.showErrorMessage(`Entity "${word}" not found.`);
      }
    }),

    vscode.commands.registerCommand("specforge.showGraph", () => {
      GraphWebviewPanel.show(context.extensionUri);
    }),

    vscode.commands.registerCommand("specforge.showExplorer", () => {
      vscode.commands.executeCommand(
        "workbench.view.extension.specforge-explorer"
      );
    }),

    vscode.commands.registerCommand("specforge.focusInGraph", (entityId?: string) => {
      const word = entityId ?? (() => {
        const editor = vscode.window.activeTextEditor;
        if (!editor) return undefined;
        const wordRange = editor.document.getWordRangeAtPosition(
          editor.selection.active
        );
        return wordRange ? editor.document.getText(wordRange) : undefined;
      })();

      if (!word) {
        vscode.window.showInformationMessage(
          "Place cursor on an entity ID to focus it in the graph."
        );
        return;
      }

      const panel = GraphWebviewPanel.getInstance() ?? GraphWebviewPanel.show(context.extensionUri);
      panel.focusEntity(word);
    }),
  ];
}
