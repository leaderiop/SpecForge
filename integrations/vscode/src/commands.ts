import * as vscode from "vscode";
import { restartClient } from "./lsp-client";
import { GraphWebviewPanel } from "./graph-webview";
import { cliOutput, inTask } from "./binary";

/**
 * One entity as JSON: `specforge query <id> --depth 0` returns a subgraph
 * holding just that node.
 */
async function entityJson(id: string): Promise<string> {
  const output = await cliOutput(["query", id, "--depth", "0"]);
  const node = (JSON.parse(output) as { nodes?: unknown[] }).nodes?.[0];
  if (!node) {
    throw new Error(`Entity "${id}" not found.`);
  }
  return JSON.stringify(node, null, 2);
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

      // An empty `--extensions` is refused (E054): pass the flag only for a pick.
      if (!extensions.length) {
        inTask(["init"]);
        return;
      }
      inTask(["init", "--extensions", extensions.map((e) => e.label).join(",")]);
    }),

    vscode.commands.registerCommand("specforge.check", () => {
      inTask(["check"]);
    }),

    vscode.commands.registerCommand("specforge.export", async () => {
      const format = await vscode.window.showQuickPick(
        ["graph", "context", "brief"],
        { placeHolder: "Export format" }
      );
      if (!format) return;
      inTask(["export", `--format=${format}`]);
    }),

    vscode.commands.registerCommand("specforge.showStats", async () => {
      try {
        const output = await cliOutput(["stats", "--format=json"]);
        const doc = await vscode.workspace.openTextDocument({
          content: output,
          language: "json",
        });
        await vscode.window.showTextDocument(doc);
      } catch {
        inTask(["stats"]);
      }
    }),

    vscode.commands.registerCommand("specforge.doctor", () => {
      inTask(["doctor"]);
    }),

    vscode.commands.registerCommand("specforge.restartLsp", async () => {
      await restartClient(context);
      vscode.window.showInformationMessage(
        "SpecForge language server restarted."
      );
    }),

    // The language server's workspace symbol search, in the editor's own picker.
    vscode.commands.registerCommand("specforge.search", () =>
      vscode.commands.executeCommand("workbench.action.quickOpen", "#")
    ),

    vscode.commands.registerCommand("specforge.explainError", async () => {
      const code = await vscode.window.showInputBox({
        placeHolder: "E001",
        prompt: "Enter a diagnostic code to explain",
      });
      if (!code) return;
      try {
        const output = await cliOutput(["explain", code]);
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
        const output = await cliOutput([
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
        inTask(["model", `--format=${format}`]);
      }
    }),

    vscode.commands.registerCommand("specforge.coverage", () => {
      inTask(["analyze", "coverage"]);
    }),

    vscode.commands.registerCommand("specforge.outline", async () => {
      try {
        const output = await cliOutput([
          "outline",
          "--format=json",
        ]);
        const doc = await vscode.workspace.openTextDocument({
          content: output,
          language: "json",
        });
        await vscode.window.showTextDocument(doc);
      } catch {
        inTask(["outline"]);
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
      inTask(["add", ext]);
    }),

    vscode.commands.registerCommand("specforge.removeExtension", async () => {
      const ext = await vscode.window.showInputBox({
        placeHolder: "@specforge/software",
        prompt: "Extension to remove",
      });
      if (!ext) return;
      inTask(["remove", ext]);
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
        const output = await entityJson(word);
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
        const output = await cliOutput(["trace", word]);
        const doc = await vscode.workspace.openTextDocument({
          content: output,
          language: "json",
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
        const output = await entityJson(word);
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
