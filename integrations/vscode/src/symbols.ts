import * as vscode from "vscode";

/**
 * VS Code's own rendering of a `SymbolKind`: its codicon and its theme colour. A kind's `lsp_icon`
 * is the only fact an extension declares about how an entity looks; the language server reports it
 * as the SymbolKind of the kind's symbols (`Object` when none, ADR 0268 D4), and every view of the
 * client (tree, graph, hover) renders that one SymbolKind.
 */
const K = vscode.SymbolKind;
const symbols: Record<vscode.SymbolKind, [icon: string, color: string]> = {
  [K.File]: ["symbol-file", "symbolIcon.fileForeground"],
  [K.Module]: ["symbol-module", "symbolIcon.moduleForeground"],
  [K.Namespace]: ["symbol-namespace", "symbolIcon.namespaceForeground"],
  [K.Package]: ["symbol-package", "symbolIcon.packageForeground"],
  [K.Class]: ["symbol-class", "symbolIcon.classForeground"],
  [K.Method]: ["symbol-method", "symbolIcon.methodForeground"],
  [K.Property]: ["symbol-property", "symbolIcon.propertyForeground"],
  [K.Field]: ["symbol-field", "symbolIcon.fieldForeground"],
  [K.Constructor]: ["symbol-constructor", "symbolIcon.constructorForeground"],
  [K.Enum]: ["symbol-enum", "symbolIcon.enumeratorForeground"],
  [K.Interface]: ["symbol-interface", "symbolIcon.interfaceForeground"],
  [K.Function]: ["symbol-function", "symbolIcon.functionForeground"],
  [K.Variable]: ["symbol-variable", "symbolIcon.variableForeground"],
  [K.Constant]: ["symbol-constant", "symbolIcon.constantForeground"],
  [K.String]: ["symbol-string", "symbolIcon.stringForeground"],
  [K.Number]: ["symbol-number", "symbolIcon.numberForeground"],
  [K.Boolean]: ["symbol-boolean", "symbolIcon.booleanForeground"],
  [K.Array]: ["symbol-array", "symbolIcon.arrayForeground"],
  [K.Object]: ["symbol-object", "symbolIcon.objectForeground"],
  [K.Key]: ["symbol-key", "symbolIcon.keyForeground"],
  [K.Null]: ["symbol-null", "symbolIcon.nullForeground"],
  [K.EnumMember]: ["symbol-enum-member", "symbolIcon.enumeratorMemberForeground"],
  [K.Struct]: ["symbol-struct", "symbolIcon.structForeground"],
  [K.Event]: ["symbol-event", "symbolIcon.eventForeground"],
  [K.Operator]: ["symbol-operator", "symbolIcon.operatorForeground"],
  [K.TypeParameter]: ["symbol-type-parameter", "symbolIcon.typeParameterForeground"],
};

function of(kind: vscode.SymbolKind | undefined): [string, string] {
  return symbols[kind ?? K.Object] ?? symbols[K.Object];
}

/** The icon of an entity whose kind declared `kind` (`Object` when it declared none). */
export function symbolIcon(kind: vscode.SymbolKind | undefined): vscode.ThemeIcon {
  const [icon, color] = of(kind);
  return new vscode.ThemeIcon(icon, new vscode.ThemeColor(color));
}

/** The codicon text of `kind`, for markdown (`$(symbol-method)`). */
export function symbolCodicon(kind: vscode.SymbolKind | undefined): string {
  return `$(${of(kind)[0]})`;
}

/** The CSS colour of `kind` in a webview: the theme's colour variable (`var(--vscode-symbolIcon-…)`). */
export function symbolColor(kind: vscode.SymbolKind | undefined): string {
  return `var(--vscode-${of(kind)[1].replace(".", "-")})`;
}
