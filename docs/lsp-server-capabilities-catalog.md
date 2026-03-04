# LSP Server Capabilities Catalog

**Complete reference for LSP 3.17 server capabilities**

Based on `lsp-types` v0.97.0 and the LSP 3.17 specification.

## Overview

The `ServerCapabilities` structure contains 34 fields that servers can use to advertise their features to clients during the `initialize` handshake. This document catalogs every capability with its purpose, request/response types, and dynamic registration support.

---

## Core Capabilities

### 1. position_encoding

**Type:** `Option<PositionEncodingKind>`

**What it does:** Negotiates the character encoding used for position calculations between client and server. Supports UTF-8, UTF-16, or UTF-32.

**Request/Response:** Set during initialization only (not a request/response capability)

**Dynamic registration:** No (initialization-time only)

**Introduced:** LSP 3.17.0

---

### 2. text_document_sync

**Type:** `Option<TextDocumentSyncCapability>`

**What it does:** Defines how text documents are synchronized between client and server. Determines whether the server receives full document content, incremental changes, or no sync.

**Related messages:**
- `textDocument/didOpen` (notification)
- `textDocument/didChange` (notification)
- `textDocument/didClose` (notification)
- `textDocument/didSave` (notification)
- `textDocument/willSave` (notification)
- `textDocument/willSaveWaitUntil` (request → `TextEdit[]`)

**Sync modes:**
- `None`: No synchronization
- `Full`: Full document content sent on every change
- `Incremental`: Only changed portions sent

**Dynamic registration:** No (initialization-time only)

**Introduced:** LSP Core (1.0)

---

### 3. notebook_document_sync

**Type:** `Option<OneOf<NotebookDocumentSyncOptions, NotebookDocumentSyncRegistrationOptions>>`

**What it does:** Enables synchronization of notebook documents (e.g., Jupyter notebooks) between client and server.

**Related messages:**
- `notebookDocument/didOpen`
- `notebookDocument/didChange`
- `notebookDocument/didSave`
- `notebookDocument/didClose`

**Dynamic registration:** Yes

**Introduced:** LSP 3.17.0

---

## Language Features

### 4. completion_provider

**Type:** `Option<CompletionOptions>`

**What it does:** Provides code completion suggestions at a given cursor position.

**Request:** `textDocument/completion`
- **Params:** `CompletionParams` (document, position, context)
- **Response:** `CompletionItem[]` or `CompletionList`

**Follow-up request:** `completionItem/resolve`
- **Purpose:** Lazily compute expensive completion properties (documentation, detail)
- **Params:** `CompletionItem`
- **Response:** `CompletionItem` (with additional data)

**CompletionOptions fields:**
- `triggerCharacters`: Characters that trigger completion (e.g., `.`, `::`)
- `allCommitCharacters`: Characters that commit a selection
- `resolveProvider`: Boolean indicating support for `completionItem/resolve`
- `completionItem`: Settings for label details and commit characters
- `workDoneProgress`: Progress reporting support

**Dynamic registration:** Yes

**Introduced:** LSP Core (1.0)

---

### 5. hover_provider

**Type:** `Option<HoverProviderCapability>`

**What it does:** Provides hover information (documentation, type info) for symbols under the cursor.

**Request:** `textDocument/hover`
- **Params:** `HoverParams` (document, position)
- **Response:** `Hover` (markdown content, range)

**Dynamic registration:** Yes

**Introduced:** LSP Core (1.0)

---

### 6. signature_help_provider

**Type:** `Option<SignatureHelpOptions>`

**What it does:** Shows function/method signature hints while typing arguments.

**Request:** `textDocument/signatureHelp`
- **Params:** `SignatureHelpParams` (document, position, context)
- **Response:** `SignatureHelp` (signatures, active signature, active parameter)

**SignatureHelpOptions fields:**
- `triggerCharacters`: Characters that trigger help (e.g., `(`, `,`)
- `retriggerCharacters`: Characters that retrigger when already shown
- `workDoneProgress`: Progress reporting support

**Dynamic registration:** Yes

**Introduced:** LSP Core (1.0)

---

### 7. declaration_provider

**Type:** `Option<DeclarationCapability>`

**What it does:** Navigates to the declaration of a symbol (e.g., forward declaration in C++).

**Request:** `textDocument/declaration`
- **Params:** `DeclarationParams` (document, position)
- **Response:** `Location`, `Location[]`, or `LocationLink[]`

**Dynamic registration:** Yes

**Introduced:** LSP 3.14.0

---

### 8. definition_provider

**Type:** `Option<OneOf<bool, DefinitionOptions>>`

**What it does:** Navigates to the definition of a symbol (where it's implemented/defined).

**Request:** `textDocument/definition`
- **Params:** `DefinitionParams` (document, position)
- **Response:** `Location`, `Location[]`, or `LocationLink[]`

**Dynamic registration:** Yes

**Introduced:** LSP Core (1.0)

**Note:** This is one of the most commonly implemented features.

---

### 9. type_definition_provider

**Type:** `Option<TypeDefinitionProviderCapability>`

**What it does:** Navigates to the type definition of a symbol (e.g., from variable to type).

**Request:** `textDocument/typeDefinition`
- **Params:** `TypeDefinitionParams` (document, position)
- **Response:** `Location`, `Location[]`, or `LocationLink[]`

**Dynamic registration:** Yes

**Introduced:** LSP 3.6.0

---

### 10. implementation_provider

**Type:** `Option<ImplementationProviderCapability>`

**What it does:** Navigates to implementations of an interface or abstract class.

**Request:** `textDocument/implementation`
- **Params:** `ImplementationParams` (document, position)
- **Response:** `Location`, `Location[]`, or `LocationLink[]`

**Dynamic registration:** Yes

**Introduced:** LSP 3.6.0

---

### 11. references_provider

**Type:** `Option<OneOf<bool, ReferencesOptions>>`

**What it does:** Finds all references to a symbol throughout the workspace.

**Request:** `textDocument/references`
- **Params:** `ReferenceParams` (document, position, include declaration flag)
- **Response:** `Location[]`

**Dynamic registration:** Yes

**Introduced:** LSP Core (1.0)

---

### 12. document_highlight_provider

**Type:** `Option<OneOf<bool, DocumentHighlightOptions>>`

**What it does:** Highlights all occurrences of the symbol under the cursor within the current document.

**Request:** `textDocument/documentHighlight`
- **Params:** `DocumentHighlightParams` (document, position)
- **Response:** `DocumentHighlight[]` (range, kind: read/write/text)

**Dynamic registration:** Yes

**Introduced:** LSP Core (1.0)

**Note:** Used for "highlight matching symbols" feature in editors.

---

### 13. document_symbol_provider

**Type:** `Option<OneOf<bool, DocumentSymbolOptions>>`

**What it does:** Lists all symbols (classes, functions, variables) in a document for outline/navigation.

**Request:** `textDocument/documentSymbol`
- **Params:** `DocumentSymbolParams` (document)
- **Response:** `DocumentSymbol[]` or `SymbolInformation[]`

**DocumentSymbol structure:** Hierarchical (nested children)
**SymbolInformation structure:** Flat list

**Dynamic registration:** Yes

**Introduced:** LSP Core (1.0)

**Note:** Powers the "Outline" view in editors.

---

### 14. workspace_symbol_provider

**Type:** `Option<OneOf<bool, WorkspaceSymbolOptions>>`

**What it does:** Searches for symbols across the entire workspace (e.g., "Go to Symbol in Workspace").

**Request:** `workspace/symbol`
- **Params:** `WorkspaceSymbolParams` (query string)
- **Response:** `SymbolInformation[]` or `WorkspaceSymbol[]`

**Follow-up request:** `workspaceSymbol/resolve`
- **Purpose:** Lazily compute expensive symbol properties
- **Params:** `WorkspaceSymbol`
- **Response:** `WorkspaceSymbol` (with additional data)

**Dynamic registration:** Yes

**Introduced:** LSP Core (1.0)

---

### 15. code_action_provider

**Type:** `Option<CodeActionProviderCapability>`

**What it does:** Provides quick fixes, refactorings, and other code actions for diagnostics or selections.

**Request:** `textDocument/codeAction`
- **Params:** `CodeActionParams` (document, range, diagnostics context)
- **Response:** `(Command | CodeAction)[]`

**Follow-up request:** `codeAction/resolve`
- **Purpose:** Lazily compute expensive action properties (workspace edits)
- **Params:** `CodeAction`
- **Response:** `CodeAction` (with workspace edit)

**CodeActionOptions fields:**
- `codeActionKinds`: Supported kinds (quickfix, refactor, source, etc.)
- `resolveProvider`: Boolean indicating support for `codeAction/resolve`
- `workDoneProgress`: Progress reporting support

**Common action kinds:**
- `quickfix`: Fix problems
- `refactor`: Rename, extract, etc.
- `source`: Organize imports, format, etc.

**Dynamic registration:** Yes

**Introduced:** LSP Core (1.0)

---

### 16. code_lens_provider

**Type:** `Option<CodeLensOptions>`

**What it does:** Displays inline commands/information above code (e.g., "5 references", "Run test").

**Request:** `textDocument/codeLens`
- **Params:** `CodeLensParams` (document)
- **Response:** `CodeLens[]` (range, command, data)

**Follow-up request:** `codeLens/resolve`
- **Purpose:** Lazily compute expensive code lens properties
- **Params:** `CodeLens`
- **Response:** `CodeLens` (with command)

**CodeLensOptions fields:**
- `resolveProvider`: Boolean indicating support for `codeLens/resolve`
- `workDoneProgress`: Progress reporting support

**Dynamic registration:** Yes

**Introduced:** LSP Core (1.0)

---

### 17. document_formatting_provider

**Type:** `Option<OneOf<bool, DocumentFormattingOptions>>`

**What it does:** Formats an entire document according to style rules.

**Request:** `textDocument/formatting`
- **Params:** `DocumentFormattingParams` (document, formatting options)
- **Response:** `TextEdit[]`

**Dynamic registration:** Yes

**Introduced:** LSP Core (1.0)

---

### 18. document_range_formatting_provider

**Type:** `Option<OneOf<bool, DocumentRangeFormattingOptions>>`

**What it does:** Formats a selected range within a document.

**Request:** `textDocument/rangeFormatting`
- **Params:** `DocumentRangeFormattingParams` (document, range, formatting options)
- **Response:** `TextEdit[]`

**Dynamic registration:** Yes

**Introduced:** LSP Core (1.0)

---

### 19. document_on_type_formatting_provider

**Type:** `Option<DocumentOnTypeFormattingOptions>`

**What it does:** Formats code as you type (e.g., auto-indent after `{` or newline).

**Request:** `textDocument/onTypeFormatting`
- **Params:** `DocumentOnTypeFormattingParams` (document, position, character, formatting options)
- **Response:** `TextEdit[]`

**DocumentOnTypeFormattingOptions fields:**
- `firstTriggerCharacter`: Required trigger character (e.g., `}`, `;`)
- `moreTriggerCharacter`: Optional additional triggers

**Dynamic registration:** Yes

**Introduced:** LSP Core (1.0)

---

### 20. rename_provider

**Type:** `Option<OneOf<bool, RenameOptions>>`

**What it does:** Renames symbols across the workspace with consistency checks.

**Request:** `textDocument/rename`
- **Params:** `RenameParams` (document, position, new name)
- **Response:** `WorkspaceEdit`

**Prepare request:** `textDocument/prepareRename`
- **Purpose:** Validate rename and provide placeholder
- **Params:** `PrepareRenameParams` (document, position)
- **Response:** `Range` or `{ range, placeholder }`

**RenameOptions fields:**
- `prepareProvider`: Boolean indicating support for `prepareRename`
- `workDoneProgress`: Progress reporting support

**Dynamic registration:** Yes

**Introduced:** LSP Core (1.0)

---

### 21. document_link_provider

**Type:** `Option<DocumentLinkOptions>`

**What it does:** Detects clickable links in documents (URLs, file paths, imports).

**Request:** `textDocument/documentLink`
- **Params:** `DocumentLinkParams` (document)
- **Response:** `DocumentLink[]` (range, target URL, tooltip, data)

**Follow-up request:** `documentLink/resolve`
- **Purpose:** Lazily compute expensive link properties
- **Params:** `DocumentLink`
- **Response:** `DocumentLink` (with target)

**DocumentLinkOptions fields:**
- `resolveProvider`: Boolean indicating support for `documentLink/resolve`
- `workDoneProgress`: Progress reporting support

**Dynamic registration:** Yes

**Introduced:** LSP Core (1.0)

---

### 22. color_provider

**Type:** `Option<ColorProviderCapability>`

**What it does:** Detects color literals and provides color picker UI.

**Request:** `textDocument/documentColor`
- **Params:** `DocumentColorParams` (document)
- **Response:** `ColorInformation[]` (range, color RGBA)

**Presentation request:** `textDocument/colorPresentation`
- **Purpose:** Convert color to text representation
- **Params:** `ColorPresentationParams` (document, color, range)
- **Response:** `ColorPresentation[]` (label, text edit, additional edits)

**Dynamic registration:** Yes

**Introduced:** LSP 3.6.0

---

### 23. folding_range_provider

**Type:** `Option<FoldingRangeProviderCapability>`

**What it does:** Provides code folding regions (functions, blocks, imports, comments).

**Request:** `textDocument/foldingRange`
- **Params:** `FoldingRangeParams` (document)
- **Response:** `FoldingRange[]` (start line, end line, kind)

**Folding kinds:**
- `comment`: Comment blocks
- `imports`: Import/include sections
- `region`: User-defined regions

**Dynamic registration:** Yes

**Introduced:** LSP 3.10.0

---

### 24. selection_range_provider

**Type:** `Option<SelectionRangeProviderCapability>`

**What it does:** Provides semantic selection expansion (e.g., select word → expression → statement → block).

**Request:** `textDocument/selectionRange`
- **Params:** `SelectionRangeParams` (document, positions)
- **Response:** `SelectionRange[]` (range, parent)

**Dynamic registration:** Yes

**Introduced:** LSP 3.15.0

**Note:** Powers "Expand Selection" / "Shrink Selection" commands.

---

### 25. linked_editing_range_provider

**Type:** `Option<LinkedEditingRangeServerCapabilities>`

**What it does:** Enables simultaneous editing of related ranges (e.g., HTML open/close tags).

**Request:** `textDocument/linkedEditingRange`
- **Params:** `LinkedEditingRangeParams` (document, position)
- **Response:** `LinkedEditingRanges` (ranges, word pattern)

**Dynamic registration:** Yes

**Introduced:** LSP 3.16.0

---

### 26. call_hierarchy_provider

**Type:** `Option<CallHierarchyServerCapability>`

**What it does:** Navigates call hierarchies (callers/callees of functions).

**Prepare request:** `textDocument/prepareCallHierarchy`
- **Params:** `CallHierarchyPrepareParams` (document, position)
- **Response:** `CallHierarchyItem[]`

**Incoming calls:** `callHierarchy/incomingCalls`
- **Params:** `CallHierarchyIncomingCallsParams` (item)
- **Response:** `CallHierarchyIncomingCall[]`

**Outgoing calls:** `callHierarchy/outgoingCalls`
- **Params:** `CallHierarchyOutgoingCallsParams` (item)
- **Response:** `CallHierarchyOutgoingCall[]`

**Dynamic registration:** Yes

**Introduced:** LSP 3.16.0

---

### 27. type_hierarchy_provider

**Type:** Implied by field name (not in the 34-field list, but exists in specs)

**What it does:** Navigates type hierarchies (supertypes/subtypes).

**Prepare request:** `textDocument/prepareTypeHierarchy`
- **Params:** `TypeHierarchyPrepareParams` (document, position)
- **Response:** `TypeHierarchyItem[]`

**Supertypes:** `typeHierarchy/supertypes`
- **Params:** `TypeHierarchySupertypesParams` (item)
- **Response:** `TypeHierarchyItem[]`

**Subtypes:** `typeHierarchy/subtypes`
- **Params:** `TypeHierarchySubtypesParams` (item)
- **Response:** `TypeHierarchyItem[]`

**Dynamic registration:** Yes

**Introduced:** LSP 3.17.0

---

### 28. semantic_tokens_provider

**Type:** `Option<SemanticTokensServerCapabilities>`

**What it does:** Provides fine-grained semantic syntax highlighting (better than regex-based TextMate grammars).

**Full document request:** `textDocument/semanticTokens/full`
- **Params:** `SemanticTokensParams` (document)
- **Response:** `SemanticTokens` (encoded token array)

**Delta request:** `textDocument/semanticTokens/full/delta`
- **Purpose:** Only send changes since last request (efficient)
- **Params:** `SemanticTokensDeltaParams` (document, previous result ID)
- **Response:** `SemanticTokensDelta` (edits to token array)

**Range request:** `textDocument/semanticTokens/range`
- **Params:** `SemanticTokensRangeParams` (document, range)
- **Response:** `SemanticTokens`

**SemanticTokensServerCapabilities fields:**
- `legend`: Token types and modifiers vocabulary
- `range`: Boolean/object indicating range support
- `full`: Boolean/object indicating full document support
  - `delta`: Boolean indicating delta support
- `workDoneProgress`: Progress reporting support

**Token encoding:** Compact integer array format (delta-encoded positions)

**Dynamic registration:** Yes

**Introduced:** LSP 3.16.0

---

### 29. inline_value_provider

**Type:** `Option<OneOf<bool, InlineValueServerCapabilities>>`

**What it does:** Displays inline variable values during debugging (like VS Code's "inline values" in debug mode).

**Request:** `textDocument/inlineValue`
- **Params:** `InlineValueParams` (document, range, context)
- **Response:** `InlineValue[]`

**Dynamic registration:** Yes

**Introduced:** LSP 3.17.0

---

### 30. inlay_hint_provider

**Type:** `Option<OneOf<bool, InlayHintServerCapabilities>>`

**What it does:** Displays inline hints (type annotations, parameter names) without modifying code.

**Request:** `textDocument/inlayHint`
- **Params:** `InlayHintParams` (document, range)
- **Response:** `InlayHint[]` (position, label, kind, tooltip, padding)

**Resolve request:** `inlayHint/resolve`
- **Purpose:** Lazily compute expensive hint properties
- **Params:** `InlayHint`
- **Response:** `InlayHint` (with tooltip/edits)

**Hint kinds:**
- `type`: Type annotations (e.g., `: string`)
- `parameter`: Parameter names (e.g., `value: 42`)

**InlayHintServerCapabilities fields:**
- `resolveProvider`: Boolean indicating support for `inlayHint/resolve`
- `workDoneProgress`: Progress reporting support

**Dynamic registration:** Yes

**Introduced:** LSP 3.17.0

**Note:** Rust Analyzer popularized this feature.

---

### 31. diagnostic_provider

**Type:** `Option<DiagnosticServerCapabilities>`

**What it does:** Provides **pull diagnostics** (client requests diagnostics on-demand, unlike traditional push model).

**Document diagnostics:** `textDocument/diagnostic`
- **Params:** `DocumentDiagnosticParams` (document, previous result ID)
- **Response:** `DocumentDiagnosticReport` (full or unchanged)

**Workspace diagnostics:** `workspace/diagnostic`
- **Params:** `WorkspaceDiagnosticParams` (previous result IDs)
- **Response:** `WorkspaceDiagnosticReport`

**DiagnosticServerCapabilities fields:**
- `identifier`: Optional server identifier for diagnostics
- `interFileDependencies`: Boolean (do diagnostics depend on other files?)
- `workspaceDiagnostics`: Boolean (supports workspace diagnostics?)
- `workDoneProgress`: Progress reporting support

**Pull vs Push:**
- **Push:** Server sends `textDocument/publishDiagnostics` notifications (traditional)
- **Pull:** Client requests diagnostics with `textDocument/diagnostic` (new in 3.17)
- Both models can coexist

**Dynamic registration:** Yes

**Introduced:** LSP 3.17.0

---

### 32. moniker_provider

**Type:** `Option<OneOf<bool, MonikerServerCapabilities>>`

**What it does:** Provides unique identifiers (monikers) for symbols across projects/repositories (for LSIF graph generation).

**Request:** `textDocument/moniker`
- **Params:** `MonikerParams` (document, position)
- **Response:** `Moniker[]` (scheme, identifier, unique, kind)

**Use case:** Code intelligence indexing, cross-repo navigation

**Dynamic registration:** Yes

**Introduced:** LSP 3.16.0

---

### 33. execute_command_provider

**Type:** `Option<ExecuteCommandOptions>`

**What it does:** Executes server-defined commands (e.g., "Reload project", "Run code action").

**Request:** `workspace/executeCommand`
- **Params:** `ExecuteCommandParams` (command, arguments)
- **Response:** `any` (command-specific result)

**ExecuteCommandOptions fields:**
- `commands`: String array of supported command identifiers
- `workDoneProgress`: Progress reporting support

**Dynamic registration:** Yes

**Introduced:** LSP Core (1.0)

**Note:** Commands are usually triggered by code actions or code lenses.

---

### 34. experimental

**Type:** `Option<Value>`

**What it does:** Allows servers to advertise experimental/non-standard capabilities.

**Format:** Arbitrary JSON value (server-defined)

**Dynamic registration:** Server-defined

**Introduced:** LSP Core (1.0)

---

## Workspace Capabilities

The `workspace` field (#26) contains nested capabilities:

### workspace.workspace_folders

**Type:** `Option<WorkspaceFoldersServerCapabilities>`

**What it does:** Indicates the server supports multi-root workspaces.

**Related notifications:**
- `workspace/didChangeWorkspaceFolders` (notification)

**WorkspaceFoldersServerCapabilities fields:**
- `supported`: Boolean indicating support
- `changeNotifications`: Boolean/string indicating notification support

---

### workspace.file_operations

**Type:** `Option<WorkspaceFileOperationsServerCapabilities>`

**What it does:** Registers interest in file operation events (create, rename, delete).

**Fields (all `Option<FileOperationRegistrationOptions>`):**

1. **did_create** - `workspace/didCreateFiles` notification
2. **will_create** - `workspace/willCreateFiles` request (can veto)
3. **did_rename** - `workspace/didRenameFiles` notification
4. **will_rename** - `workspace/willRenameFiles` request (can veto)
5. **did_delete** - `workspace/didDeleteFiles` notification
6. **will_delete** - `workspace/willDeleteFiles` request (can veto)

**FileOperationRegistrationOptions fields:**
- `filters`: Glob patterns for files to watch

**Use cases:**
- Update imports when files are renamed
- Warn about breaking changes before deletion
- Generate boilerplate when files are created

**Introduced:** LSP 3.16.0

---

## Additional Workspace Messages

### workspace/didChangeConfiguration

**What it does:** Notifies the server when client configuration changes.

**Notification:** `workspace/didChangeConfiguration`
- **Params:** `DidChangeConfigurationParams` (settings)

**Pull request:** `workspace/configuration`
- **Purpose:** Server requests specific config from client
- **Params:** `ConfigurationParams` (items)
- **Response:** `any[]`

**No explicit capability:** Implicitly supported by all servers

---

### workspace/didChangeWatchedFiles

**What it does:** Notifies the server when watched files change on disk.

**Notification:** `workspace/didChangeWatchedFiles`
- **Params:** `DidChangeWatchedFilesParams` (changes)

**Server registration:** Server can dynamically register file watchers

**Change types:** Created, Changed, Deleted

---

### workspace/applyEdit

**What it does:** Server requests the client to apply a workspace edit.

**Request:** `workspace/applyEdit` (initiated by server)
- **Params:** `ApplyWorkspaceEditParams` (edit, label)
- **Response:** `ApplyWorkspaceEditResult` (applied, failure reason)

**No explicit capability:** Implicitly supported by clients that accept edits

---

## Window Messages (Server-initiated)

### window/showMessage

**What it does:** Server displays a notification message to the user.

**Notification:** `window/showMessage`
- **Params:** `ShowMessageParams` (type, message)
- **Message types:** Error, Warning, Info, Log

---

### window/showMessageRequest

**What it does:** Server displays a message with action buttons (e.g., "Yes/No" dialog).

**Request:** `window/showMessageRequest`
- **Params:** `ShowMessageRequestParams` (type, message, actions)
- **Response:** `MessageActionItem` or `null`

---

### window/logMessage

**What it does:** Server logs a message to the client's output channel.

**Notification:** `window/logMessage`
- **Params:** `LogMessageParams` (type, message)

---

### window/showDocument

**What it does:** Server requests the client to open/reveal a document.

**Request:** `window/showDocument`
- **Params:** `ShowDocumentParams` (URI, external, take focus, selection)
- **Response:** `ShowDocumentResult` (success)

**Introduced:** LSP 3.16.0

---

### window/workDoneProgress

**What it does:** Server reports long-running operation progress.

**Request:** `window/workDoneProgress/create`
- **Params:** `WorkDoneProgressCreateParams` (token)
- **Response:** `void`

**Notifications:**
- `$/progress` (begin, report, end)

---

## Additional Messages

### $/cancelRequest

**What it does:** Client cancels an in-flight request.

**Notification:** `$/cancelRequest`
- **Params:** `CancelParams` (request ID)

**No capability:** Always available

---

### $/setTrace

**What it does:** Client sets the trace/logging level.

**Notification:** `$/setTrace`
- **Params:** `SetTraceParams` (value: off/messages/verbose)

**No capability:** Always available

---

## Dynamic Registration Summary

**Supports dynamic registration (27):**
- All language features (completion, hover, definition, etc.)
- All formatting providers
- Semantic tokens
- Inlay hints
- Diagnostics (pull)
- Code actions/lenses
- Rename
- References
- Call/type hierarchy
- Notebook sync

**Does NOT support dynamic registration (7):**
- position_encoding (initialization only)
- text_document_sync (initialization only)
- workspace capabilities (declared at init)
- experimental
- window/showMessage*

---

## Request/Response Type Reference

### Common Parameter Types

- `TextDocumentPositionParams`: document URI + position
- `TextDocumentIdentifier`: document URI
- `Position`: line (0-based) + character (0-based)
- `Range`: start position + end position
- `WorkDoneProgressParams`: progress token
- `PartialResultParams`: partial result token

### Common Response Types

- `Location`: URI + range
- `LocationLink`: origin range + target URI + target range + target selection range
- `TextEdit`: range + new text
- `WorkspaceEdit`: changes across multiple documents
- `Command`: title + command identifier + arguments
- `Diagnostic`: range + severity + code + message + source

---

## Capability Priorities for Implementation

### Essential (Core LSP)
1. `text_document_sync` - Mandatory for any LSP server
2. `hover_provider` - Documentation on hover
3. `definition_provider` - Go to definition
4. `completion_provider` - Code completion
5. `document_symbol_provider` - Outline view

### High Value
6. `references_provider` - Find all references
7. `rename_provider` - Refactoring
8. `code_action_provider` - Quick fixes
9. `document_formatting_provider` - Format document
10. `diagnostic_provider` - Error checking (push or pull)

### Medium Value
11. `document_highlight_provider` - Highlight matching symbols
12. `signature_help_provider` - Function signatures
13. `workspace_symbol_provider` - Global symbol search
14. `folding_range_provider` - Code folding
15. `semantic_tokens_provider` - Rich syntax highlighting

### Advanced Features
16. `inlay_hint_provider` - Type/parameter hints (very popular in Rust)
17. `code_lens_provider` - Inline annotations
18. `call_hierarchy_provider` - Navigate call graphs
19. `implementation_provider` - Find implementations
20. `type_definition_provider` - Go to type

### Specialized
21. `color_provider` - CSS/color literals
22. `document_link_provider` - Clickable links
23. `selection_range_provider` - Smart selection
24. `linked_editing_range_provider` - Linked editing (HTML tags)
25. `moniker_provider` - LSIF indexing
26. `inline_value_provider` - Debugging support

---

## SpecForge LSP Recommendations

For the SpecForge language server, prioritize:

### Phase 1 (MVP)
1. ✅ `text_document_sync` - Document synchronization
2. ✅ `hover_provider` - Show entity descriptions
3. ✅ `definition_provider` - Jump to referenced entities
4. ✅ `document_symbol_provider` - Outline of entities
5. ✅ Diagnostics via `textDocument/publishDiagnostics` (push model)

### Phase 2 (Productivity)
6. `completion_provider` - Entity references, field names, keywords
7. `references_provider` - Find entity usages
8. `rename_provider` - Rename entities with consistency
9. `document_formatting_provider` - Format .spec files
10. `code_action_provider` - Quick fixes for validation errors

### Phase 3 (Advanced)
11. `workspace_symbol_provider` - Search entities across project
12. `semantic_tokens_provider` - Rich syntax highlighting
13. `code_lens_provider` - Show test coverage, reference counts
14. `inlay_hint_provider` - Show inferred titles/types
15. `folding_range_provider` - Fold entity blocks

### Phase 4 (Specialized)
16. `document_link_provider` - Clickable external refs (Figma, GitHub, etc.)
17. `call_hierarchy_provider` - Entity dependency chains
18. `diagnostic_provider` - Pull diagnostics for large workspaces
19. `execute_command_provider` - "Run tests", "Generate code" commands

---

## References

- **LSP Specification:** https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/
- **lsp-types crate:** https://docs.rs/lsp-types/latest/lsp_types/struct.ServerCapabilities.html
- **tower-lsp crate:** https://docs.rs/tower-lsp/latest/tower_lsp/
- **LSP Meta Model:** https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/metaModel/metaModel.html

---

**Document version:** 2026-03-04
**LSP version:** 3.17
**lsp-types version:** 0.97.0
