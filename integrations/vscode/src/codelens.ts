import * as vscode from "vscode";
import { getClient } from "./lsp-client";
import { isCodeLensEnabled } from "./config";

/**
 * Regex to match entity declaration lines in .spec files.
 * Captures: (1) entity kind, (2) entity ID, optional quoted title, opening brace.
 * Examples:
 *   behavior user_login "User Login" {
 *   type payment_method {
 *   invariant balance_non_negative "Balance Non-Negative" {
 */
const ENTITY_DECLARATION_RE =
  /^\s*(\w+)\s+(\w+)\s+(?:"[^"]*"\s*)?\{/;

/**
 * Regex to match verify lines inside an entity block.
 */
const VERIFY_LINE_RE = /^\s*verify\s+/;

/**
 * Structural keywords that should NOT be treated as entity declarations.
 * These are field names, control-flow, or DSL constructs that happen to
 * match the `keyword identifier {` pattern but are not top-level entities.
 */
const NON_ENTITY_KEYWORDS = new Set([
  "use",
  "define",
  "verify",
  "if",
  "else",
  "for",
  "while",
  "match",
  "return",
  "fn",
  "let",
  "const",
  "enum",
  "struct",
  "impl",
  "trait",
  "mod",
  "pub",
  "import",
  "export",
]);

interface EntityInfo {
  kind: string;
  id: string;
  line: number;
  verifyCount: number;
}

type CodeLensKind = "references" | "verify" | "graph";

interface CodeLensData {
  kind: CodeLensKind;
  entityId: string;
  entityKind: string;
  uri: string;
  line: number;
  verifyCount?: number;
}

export class SpecForgeCodeLensProvider
  implements vscode.CodeLensProvider<vscode.CodeLens>
{
  private readonly _onDidChangeCodeLenses = new vscode.EventEmitter<void>();
  readonly onDidChangeCodeLenses = this._onDidChangeCodeLenses.event;

  private readonly _disposables: vscode.Disposable[] = [];

  constructor() {
    // Refresh code lenses when the document changes or configuration changes
    this._disposables.push(
      vscode.workspace.onDidChangeConfiguration((e) => {
        if (e.affectsConfiguration("specforge.codeLens")) {
          this._onDidChangeCodeLenses.fire();
        }
      })
    );

    this._disposables.push(
      vscode.workspace.onDidChangeTextDocument(() => {
        this._onDidChangeCodeLenses.fire();
      })
    );
  }

  dispose(): void {
    for (const d of this._disposables) {
      d.dispose();
    }
    this._disposables.length = 0;
    this._onDidChangeCodeLenses.dispose();
  }

  provideCodeLenses(
    document: vscode.TextDocument,
    _token: vscode.CancellationToken
  ): vscode.CodeLens[] {
    if (!isCodeLensEnabled()) {
      return [];
    }

    const entities = this.parseEntities(document);
    const lenses: vscode.CodeLens[] = [];

    for (const entity of entities) {
      const range = new vscode.Range(entity.line, 0, entity.line, 0);

      // 1. Reference count lens (resolved lazily)
      const refLens = new vscode.CodeLens(range);
      (refLens as vscode.CodeLens & { data: CodeLensData }).data = {
        kind: "references",
        entityId: entity.id,
        entityKind: entity.kind,
        uri: document.uri.toString(),
        line: entity.line,
      };
      lenses.push(refLens);

      // 2. Verify count lens (only if entity has verify statements)
      if (entity.verifyCount > 0) {
        const verifyLens = new vscode.CodeLens(range);
        const noun = entity.verifyCount === 1 ? "verify statement" : "verify statements";
        verifyLens.command = {
          title: `$(beaker) ${entity.verifyCount} ${noun}`,
          command: "",
        };
        lenses.push(verifyLens);
      }

      // 3. Show in Graph lens
      const graphLens = new vscode.CodeLens(range);
      graphLens.command = {
        title: "$(graph) Show in Graph",
        command: "specforge.focusInGraph",
        arguments: [entity.id],
      };
      lenses.push(graphLens);
    }

    return lenses;
  }

  async resolveCodeLens(
    codeLens: vscode.CodeLens,
    token: vscode.CancellationToken
  ): Promise<vscode.CodeLens> {
    const data = (codeLens as vscode.CodeLens & { data?: CodeLensData }).data;
    if (!data || data.kind !== "references") {
      // Already resolved (verify and graph lenses have commands set)
      return codeLens;
    }

    const count = await this.countReferences(data, token);

    if (token.isCancellationRequested) {
      return codeLens;
    }

    const noun = count === 1 ? "reference" : "references";
    const uri = vscode.Uri.parse(data.uri);
    const position = new vscode.Position(data.line, 0);

    codeLens.command = {
      title: `$(references) ${count} ${noun}`,
      command: count > 0 ? "editor.action.findReferences" : "",
      arguments:
        count > 0 ? [uri, position] : undefined,
    };

    return codeLens;
  }

  /**
   * Parse entity declarations and their verify counts from a document.
   */
  private parseEntities(document: vscode.TextDocument): EntityInfo[] {
    const entities: EntityInfo[] = [];
    const lineCount = document.lineCount;

    for (let i = 0; i < lineCount; i++) {
      const lineText = document.lineAt(i).text;
      const match = ENTITY_DECLARATION_RE.exec(lineText);
      if (!match) {
        continue;
      }

      const kind = match[1];
      const id = match[2];

      // Skip structural keywords
      if (NON_ENTITY_KEYWORDS.has(kind)) {
        continue;
      }

      // Count verify statements inside this block
      const verifyCount = this.countVerifyStatements(document, i);

      entities.push({ kind, id, line: i, verifyCount });
    }

    return entities;
  }

  /**
   * Count `verify` lines inside the brace-delimited body starting at the
   * entity declaration line.
   */
  private countVerifyStatements(
    document: vscode.TextDocument,
    declarationLine: number
  ): number {
    let count = 0;
    let depth = 0;
    let insideBlock = false;
    const lineCount = document.lineCount;

    for (let i = declarationLine; i < lineCount; i++) {
      const text = document.lineAt(i).text;

      for (const ch of text) {
        if (ch === "{") {
          depth++;
          insideBlock = true;
        } else if (ch === "}") {
          depth--;
          if (insideBlock && depth === 0) {
            // Reached closing brace of the entity block
            return count;
          }
        }
      }

      // Only count verify lines inside the block (skip the declaration line itself
      // unless it somehow contains a verify — unlikely but harmless)
      if (insideBlock && i > declarationLine && VERIFY_LINE_RE.test(text)) {
        count++;
      }
    }

    return count;
  }

  /**
   * Use the LSP client to count references to the entity at the given position.
   * Falls back to a simple text search if the LSP is unavailable.
   */
  private async countReferences(
    data: CodeLensData,
    token: vscode.CancellationToken
  ): Promise<number> {
    const client = getClient();

    if (client) {
      try {
        const uri = vscode.Uri.parse(data.uri);
        // Position on the entity ID — find it on the declaration line
        const document = await vscode.workspace.openTextDocument(uri);
        const lineText = document.lineAt(data.line).text;
        const idStart = lineText.indexOf(data.entityId);
        const position =
          idStart >= 0
            ? new vscode.Position(data.line, idStart)
            : new vscode.Position(data.line, 0);

        const locations = await client.sendRequest<
          | vscode.Location[]
          | null
        >(
          "textDocument/references",
          {
            textDocument: { uri: data.uri },
            position: { line: position.line, character: position.character },
            context: { includeDeclaration: false },
          },
          token
        );

        if (locations) {
          return locations.length;
        }
      } catch {
        // LSP request failed — fall back to text search
      }
    }

    // Fallback: simple workspace text search for the entity ID
    return this.countTextReferences(data.entityId, data.uri, token);
  }

  /**
   * Fallback reference counting via workspace text search.
   * Counts occurrences of the entity ID across all .spec files,
   * excluding the declaration itself.
   */
  private async countTextReferences(
    entityId: string,
    declarationUri: string,
    token: vscode.CancellationToken
  ): Promise<number> {
    if (token.isCancellationRequested) {
      return 0;
    }

    try {
      // Use word-boundary matching to avoid partial matches
      const locations = await vscode.commands.executeCommand<vscode.Location[]>(
        "vscode.executeReferenceProvider",
        vscode.Uri.parse(declarationUri),
        new vscode.Position(0, 0)
      );

      if (locations) {
        // Exclude self-declaration
        return locations.filter(
          (loc) =>
            !(
              loc.uri.toString() === declarationUri &&
              loc.range.start.line === 0
            )
        ).length;
      }
    } catch {
      // Reference provider not available
    }

    // Last resort: scan open .spec documents
    let count = 0;
    const re = new RegExp(`\\b${escapeRegex(entityId)}\\b`, "g");

    for (const doc of vscode.workspace.textDocuments) {
      if (doc.languageId !== "specforge") {
        continue;
      }

      const text = doc.getText();
      const matches = text.match(re);
      if (matches) {
        count += matches.length;
      }
    }

    // Subtract 1 for the declaration itself (the entity ID appears in its own block)
    return Math.max(0, count - 1);
  }
}

function escapeRegex(str: string): string {
  return str.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}
