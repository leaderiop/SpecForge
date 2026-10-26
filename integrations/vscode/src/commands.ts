import * as vscode from "vscode";
import { GraphNode, entityAt, restartClient, sessionGraph } from "./lsp-client";
import { GraphWebviewPanel } from "./graph-webview";
import { cliOutput, inTask } from "./binary";

function errorText(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

/** One entity as JSON: the graph node the language server answers. */
function entityJson(node: GraphNode): string {
  return JSON.stringify(node, null, 2);
}

/** The entity the cursor names; else one picked from the session's entities. */
async function entityOrPick(
  editor: vscode.TextEditor
): Promise<GraphNode | undefined> {
  const named = await entityAt(editor);
  if (named) return named;
  const nodes = (await sessionGraph()).graph.nodes;
  const picked = await vscode.window.showQuickPick(
    nodes.map((node) => ({ label: node.id, description: node.kind, node })),
    { placeHolder: "The cursor names no entity: pick one", matchOnDescription: true }
  );
  return picked?.node;
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
      if (!/^[\w-]+$/.test(name)) {
        vscode.window.showErrorMessage(
          "A spec file name is letters, digits, _ and - only."
        );
        return;
      }

      // The spec root is the project's own setting (default: the folder itself).
      let specRoot = ".";
      try {
        const raw = await vscode.workspace.fs.readFile(
          vscode.Uri.joinPath(workspaceFolder.uri, "specforge.json")
        );
        const root = JSON.parse(Buffer.from(raw).toString("utf8")).spec_root;
        if (typeof root === "string") specRoot = root;
      } catch {
        // no readable specforge.json: the folder itself
      }
      const uri = vscode.Uri.joinPath(workspaceFolder.uri, specRoot, `${name}.spec`);
      // a cloned specforge.json must not aim the write outside the workspace
      if (!uri.path.startsWith(workspaceFolder.uri.path.replace(/\/$/, "") + "/")) {
        vscode.window.showErrorMessage(
          `spec_root "${specRoot}" leaves the workspace folder.`
        );
        return;
      }
      try {
        await vscode.workspace.fs.stat(uri);
        vscode.window.showErrorMessage(`${name}.spec already exists.`);
        return;
      } catch (e) {
        if (!(e instanceof vscode.FileSystemError && e.code === "FileNotFound")) {
          vscode.window.showErrorMessage(`Cannot check ${name}.spec: ${e}`);
          return;
        }
      }
      await vscode.workspace.fs.writeFile(uri, Buffer.from(`// ${name}\n\n`));
      const doc = await vscode.workspace.openTextDocument(uri);
      await vscode.window.showTextDocument(doc);
    }),

    vscode.commands.registerCommand("specforge.inspect", async () => {
      const editor = vscode.window.activeTextEditor;
      if (!editor) return;
      try {
        const node = await entityOrPick(editor);
        if (!node) return;
        const doc = await vscode.workspace.openTextDocument({
          content: entityJson(node),
          language: "json",
        });
        await vscode.window.showTextDocument(doc);
      } catch (err) {
        vscode.window.showErrorMessage(errorText(err));
      }
    }),

    vscode.commands.registerCommand("specforge.trace", async () => {
      const editor = vscode.window.activeTextEditor;
      if (!editor) return;
      try {
        const node = await entityOrPick(editor);
        if (!node) return;
        const output = await cliOutput(["trace", node.id]);
        const doc = await vscode.workspace.openTextDocument({
          content: output,
          language: "json",
        });
        await vscode.window.showTextDocument(doc);
      } catch (err) {
        vscode.window.showErrorMessage(`Could not trace: ${errorText(err)}`);
      }
    }),

    vscode.commands.registerCommand("specforge.copyEntityJson", async () => {
      const editor = vscode.window.activeTextEditor;
      if (!editor) return;
      try {
        const node = await entityAt(editor);
        if (!node) {
          vscode.window.showInformationMessage("The cursor names no entity.");
          return;
        }
        await vscode.env.clipboard.writeText(entityJson(node));
        vscode.window.showInformationMessage(`Copied ${node.id} JSON to clipboard.`);
      } catch (err) {
        vscode.window.showErrorMessage(errorText(err));
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

    // The command the language server's references lens names (ADR 0249 D2).
    vscode.commands.registerCommand(
      "specforge.findReferences",
      (uri: string, at: { line: number; character: number }) =>
        vscode.commands.executeCommand(
          "editor.action.findReferences",
          vscode.Uri.parse(uri),
          new vscode.Position(at.line, at.character)
        )
    ),

    vscode.commands.registerCommand("specforge.focusInGraph", async (entityId?: string) => {
      const editor = vscode.window.activeTextEditor;
      let id = entityId;
      try {
        if (!id && editor) id = (await entityAt(editor))?.id;
      } catch (err) {
        vscode.window.showErrorMessage(errorText(err));
        return;
      }

      if (!id) {
        vscode.window.showInformationMessage(
          "Place cursor on an entity ID to focus it in the graph."
        );
        return;
      }

      const panel = GraphWebviewPanel.getInstance() ?? GraphWebviewPanel.show(context.extensionUri);
      panel.focusEntity(id);
    }),
  ];
}
