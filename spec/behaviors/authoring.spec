// Authoring behaviors — typed edits of a project's entities

use "invariants/core"
use "ports/inbound"
use "ports/outbound"

defaults behavior {
  features [typed_edits]
  ports    [CompilerApi, FileSystem]
}

behavior create_entity "Create an Entity" {
  invariants [typed_edit_canonical_form, typed_edit_atomicity]
  category   mutation
  contract   """
    A create edit names a kind, an id, a title and a file named from the project
    root, inside the spec root (wherever the spec root lies); every spelling of the file under the project root is that file (its source key; ADR 0195, ADR 0242). The block is written in the canonical form, after the last block of
    its kind in the file, or at the end of the file, in the file's line break; a file that does not exist
    is created, unless the later edits of the list leave it empty: a file that does not
    exist reads as empty, and a file the list leaves as it read is not written. The id MUST be an identifier of 2 to 60 characters that is not a
    reserved word (E013), and MUST NOT be taken, the kind MUST be declared, the title MUST NOT be blank (only whitespace; refused
    invalid_title before the project is read; ADR 0225), and the file MUST be a project source.
    Each field is written as a set of it writes it, so a field that holds what its file's default holds, as the graph stores both,
    is not written, and each verify obligation as an add-verify writes it, its --verify prefix read by the kind's verify kinds, so an equal one given twice is written
    once. The field flags of `specforge new` are the kind's fields in the project
    the command acts on. Its command line is one command read once with the kind's fields: an option may stand anywhere
    and one given twice is refused, --help with a kind is that kind's help, its help and its usage errors name the program as specforge new, and an unknown kind is refused as unknown_kind,
    and a line whose --path a field's value spells is refused; a field named like one of its options is no flag of it, and
    the kind's help says to set it with specforge set. A create takes whole fields; a `field.entry` path is refused.
    The skeleton of a new block of a kind is what a create of it writes with no values, with a slot where each value
    goes: the id, the title, each required field and the final cursor; a member's skeleton is its field written in its
    form (ADR 0191).
  """
  verify unit "new writes the block in the canonical form after the last block of its kind"
  verify unit "new creates a missing source file"
  verify unit "new refuses a blank title"
  verify unit "new's help names a date field's value as YYYY-MM-DD"
  verify unit "new refuses a file that is not a project source"
  verify unit "new reads every spelling of a file under the project root as that file"
  verify unit "two spellings of one new file in one list are one file"
  verify unit "new refuses an id that is taken"
  verify unit "new refuses a reserved id before the edited project is compiled"
  verify unit "new compares what it writes with the default, not what it was given"
  verify unit "new never writes a field that restates its default"
  verify unit "new refuses a field.entry path"
  verify unit "new writes each field as set writes it"
  verify unit "new writes an equal verify obligation once"
  verify unit "a create's skeleton filled in is the block new writes"
  verify unit "every kind's skeleton formats to itself"
  verify unit "a member's skeleton is written in its field's form"
  verify unit "a file new creates is not written when the later edits leave it empty"
  verify integration "new reads its field flags from the project its --path names, wherever --path stands"
  verify integration "new --in reads a spelling of an existing file as that file"
  verify integration "new refuses a file named from the spec root, with no suggestion"
  verify integration "new refuses a field given twice that takes one value"
  verify integration "new refuses an unknown kind as unknown_kind"
  verify unit "a kind's field named like one of new's options is no flag of new, and the kind's help says to set it with set"
  verify integration "new --help begins with what new does"
  verify integration "new's help and its usage errors name the program as specforge new"
  verify integration "new refuses one of its options given twice, wherever each stands"
  verify integration "new's help of a kind shows each of its options as every command declares it"
  verify integration "new --help of a kind the project does not declare is refused as unknown_kind"
  verify integration "new --help after a field flag prints the kind's help"
  verify integration "new refuses a command line whose --path a field's value spells"
  verify unit "new writes into a spec root outside the project by the file check names"
  verify unit "a create that would shadow a target or make a short scope field ambiguous respells the tokens to keep their targets"
  verify unit "a skeleton writes a block list and a typed block empty, and a list of words with a slot"
}

behavior edit_entity_field "Edit an Entity Field" {
  invariants [typed_edit_canonical_form]
  category   mutation
  contract   """
    A set edit replaces a field's value, written in the form its declaration
    decides, a text that holds a line break, or whose one-line form passes the
    project's max_width, triple-quoted with its lines one indent of the
    project's format configuration past its key, and inserts an absent field
    at its rank (one-line values, then
    block fields, then triple-quoted text, in the kind's declared order), a new last field or entry above the comment lines
    directly above the closing brace of its block or block field, which no unset takes. A value holding a comment inside it (a list's item
    comment, a block field's member comment) is kept as written when the set reads the same as the graph stores both, and a set
    to another value is refused not_editable, naming its field, its entity and its file: the comment lives in no value an edit
    writes. A comment trailing the value stays on its row. An
    unset edit leaves the field holding none: it removes the field and the comment lines directly above it, as the
    parse reads them (a `//` line inside a string is none), or, over a file default, writes `[]` for a list and `{}` for a block field; a field of one value with a default cannot
    hold none and is refused. A second unset changes nothing. A
    dotted `field.entry` path edits one entry of a block field the kind
    declares; its field is refused as any field is, a field that is not a declared block field, or a path whose
    parts are not each one identifier, is refused not_editable, and so is a path into a block field the entity
    writes as a value. A block field the block inherits from its
    file's defaults is copied into an own field before an entry of it is set or unset, and an own field that then
    holds what the default holds, as the graph stores both, is removed; a set of an entry of a block field the block writes as none, or not at all,
    writes the entry inside it, and annotations given to a set are written on the value it places, a new entry's
    included; a block field left with no entry over a default is written `{}`. An unset of an
    entry, or of a block field, that neither the block nor its file's defaults hold changes nothing. A retitle to a blank title (only whitespace) is refused
    invalid_title before anything is planned, as a create's is. A retitle edit
    replaces the header title where the parse read it, or writes one after
    the name when the header has none; a union's string variant is never its
    title; a spec block's title is its name, and a retitle of it is
    refused not_editable. A field the kind does not declare is refused. A spec
    block's fields are declared by no kind: a set, unset, link or unlink of one is refused not_editable, naming the
    file to edit, as its retitle is. A field that is the other
    side of a relation another kind owns is refused naming the owning field (ADR 0157).
    A set, unset, link or unlink of a field the block writes twice first removes its later entries, which the
    graph does not hold (E081, ADR 0215). A union (`kind name = a | b`) holds only its variants: a set,
    unset, link, unlink, add-verify or remove-verify of it is refused not_editable before its field is
    read, whatever the field; a retitle and a delete take it. Whether a block is a union is the parse's
    answer, never a field's name (ADR 0219).
  """
  verify unit "set writes the value in the form its field declares"
  verify unit "a multi-line value is laid out with the project's format configuration"
  verify unit "set of an absent field inserts it at its rank"
  verify unit "unset removes the field and the comment above it"
  verify unit "unset leaves a field holding none, once"
  verify unit "set to a default's value leaves the block inheriting it, in any form"
  verify unit "set refuses a field the kind does not declare"
  verify unit "set refuses the other side of a relation naming its owning field"
  verify unit "set of a nested entry edits one line of the block field"
  verify unit "every verb that changes a field written twice leaves the block writing it once, holding what it says"
  verify unit "every verb reads a field.entry path the same way"
  verify unit "an entry of a block field written as a value is refused"
  verify unit "every verb leaves a field holding what it says, own or inherited"
  verify unit "set of an entry into a block field written as none writes it inside"
  verify unit "set of a new entry writes its annotations"
  verify unit "retitle replaces the header title"
  verify unit "retitle of a spec block is refused: its title is its name"
  verify unit "a set or unset of a spec block's field is refused not_editable: no kind declares its fields"
  verify unit "retitle of an untitled union writes the title after its name, never over a variant"
  verify unit "every member edit of a union is refused as one not_editable, before its field is read"
  verify unit "retitle and delete take a union"
  verify unit "retitle refuses a blank title before anything is planned, as create does"
  verify unit "unset keeps a // line inside the string above the field"
  verify unit "set of an absent field never writes inside the string above the first field"
  verify unit "a typed edit of a CRLF block holding a multi-line quoted text is planned, its text kept"
  verify unit "set of a value holding a comment to what it holds writes nothing"
  verify unit "set of a value holding a comment to another value is refused not_editable, naming its entity and file"
  verify unit "set keeps the comment trailing the value, annotations given or not"
  verify unit "a new last field or entry goes above the comment lines before its closing brace, and its unset leaves them"
  verify unit "setting a scope field moves the entity and respells every reference to it, and a capture is refused"
  verify unit "a plan's edits read the project as the edits before them leave it"
  verify unit "a typed edit finds its block by its declared path, so an E002 duplicate never shifts the target"
  verify unit "a typed edit sets a block member, appends and removes a block-list item, and refuses the other ways to change one"
  verify unit "set reads a JSON object or an array of objects as a typed block or a block list, and any other text as one value"
}

behavior edit_entity_link "Edit an Entity Reference List" {
  invariants [typed_edit_canonical_form]
  category   mutation
  contract   """
    A link edit adds a target to a list field and keeps the items as written;
    an unlink edit removes it, and removes the field when the last item goes.
    A link to a target already there changes nothing, whatever comment the list holds; a comment trailing the list stays on its
    row, and a link or unlink that changes a list holding a comment inside it is refused as a set of it is. A list written as one value is its one item. A field a block inherits
    from its file's defaults is copied into an own field before it changes, as
    the earlier edits of the list left the file, and an own field that holds what the default holds, as the graph stores both, is removed. A link or unlink
    takes a whole list field: a `field.entry` path names an entry of a block field, which holds one text, and is
    refused not_editable, naming set and unset. An unlink of a single reference
    the block inherits empties a field of one value over a default, which is
    refused as unset refuses it.
  """
  verify unit "link on the other side of a relation names the owning field of the target's kind"
  verify unit "a reference list's items are split at commas however they are given"
  verify unit "link appends the target and keeps the items as written"
  verify unit "unlink removes a single reference written as a number"
  verify unit "unlink of the last item removes the field"
  verify unit "link of an inherited field copies the default"
  verify unit "an own field equal to the default is removed"
  verify unit "link of a target already there writes nothing"
  verify unit "a link after new starts from the file's default"
  verify unit "a link after unset starts from none"
  verify unit "link and unlink read a list written as one value as its one item"
  verify unit "link and unlink refuse a field.entry path as an entry of a block field"
  verify unit "unlink of an inherited single reference is refused as unset is"
  verify unit "a link and its unlink leave a block as it was"
  verify unit "unlink of the last item keeps a // line inside the string above the list"
  verify unit "link and unlink keep the comment trailing the list"
  verify unit "a link to a target already there writes nothing, whatever comment the list holds"
  verify unit "a typed edit writes the shortest spelling that resolves from its holder, and a scope field as a full path"
  verify unit "a plan's edits read the project as the edits before them leave it"
}

behavior edit_entity_obligation "Edit an Entity Obligation" {
  invariants [typed_edit_canonical_form]
  category   mutation
  contract   """
    An add-verify edit inserts a verify line after the last verify statement,
    or above the comment lines directly above the closing brace, which are the
    block's: a remove-verify of it leaves them. A remove-verify edit removes the statement
    with that kind and text. Where each statement is written is read from the
    block's parse: an entry named verify inside a block field is a field, never
    a statement (ADR 0187, amended). An equal line is not added twice. The
    CLI's --verify kind:text names its kind only when the part before the first
    colon is one of the verify kinds the entity's kind accepts (declared, or an
    enhancement's), else the whole text is the obligation's text; it names the
    obligation the entity already writes in either reading of the prefix
    (ADR 0237). An MCP edit names its kind and is never split.
  """
  verify unit "a verify line is added after the last one"
  verify unit "a verify line is removed by kind and text"
  verify unit "remove-verify removes the statement, never a block field's entry named verify"
  verify unit "add-verify writes after the last statement, never inside a block field"
  verify unit "add-verify on a union is refused as not_editable, a union has no verify statements"
  verify unit "add-verify on a braced block writes the statement whatever its members are named"
  verify unit "a prefix is an obligation's kind only when it is one of the verify kinds its entity's kind accepts"
  verify unit "an obligation the entity writes in either reading of a prefix is the one its text names"
  verify unit "--verify reads a prefix as a kind only when it is one of the entity's kind's verify kinds"
  verify unit "--verify names the obligation the entity writes in either reading of its prefix"
  verify unit "specforge link --verify and specforge.edit add_verify write one obligation from one text"
  verify unit "remove-verify keeps a // line inside the string above the statement"
  verify unit "an added verify line goes above the comment lines before the closing brace, and its removal leaves them"
}

behavior delete_entity "Delete an Entity" {
  invariants [typed_edit_atomicity]
  category   mutation
  contract   """
    A delete edit removes the entity's block and its doc comment, as the parse
    reads it: the // lines directly above the block with no blank line between,
    and one blank line beside it; the file keeps its final newline.
    The format header is never a doc comment and stays. Deleting an entity that is still referenced leaves the project
    with an error, so it is refused with E080 unless the edit asks to remove
    every reference to the entity in the same change. The references are every
    token that names the entity, as the graph's one finder reads the texts the
    earlier edits left (ADR 0163). A token written where no typed edit writes,
    in a defaults block, in a field its kind does not declare or as a type name
    a derived field reads, refuses the delete as not_editable, naming where it
    is written.
    A use binding that imports the entity is removed, its statement with it
    when it was the last, and with the statement its doc comment, as a deleted
    block's goes; the format header stays (ADR 0233, ADR 0274).
    A referrer's list holding a comment inside it refuses the delete as not_editable, naming the referrer and its file.
  """
  verify unit "delete removes the block and the comment above it"
  verify unit "delete keeps the format header above the block"
  verify unit "delete of a referenced entity is E080"
  verify unit "delete with unlink_references removes every reference"
  verify unit "delete with unlink_references removes a scheme ref ID from a string list"
  verify unit "delete with unlink_references removes a reference list written as one value"
  verify unit "delete with unlink_references refuses a reference no typed edit writes"
  verify unit "delete with unlink_references refuses a type name a derived field reads"
  verify unit "delete with unlink_references removes a use binding that imports it"
  verify unit "delete with unlink_references keeps a // line inside a string above a reference"
  verify unit "delete with unlink_references through a list holding a comment is refused naming the referrer and its file"
  verify unit "a delete keeps the file's final newline"
  verify unit "delete with unlink_references removes the comment above a use statement it removes, never the format header"
}

behavior validate_typed_edit "Validate a Typed Edit" {
  invariants [typed_edit_canonical_form, typed_edit_atomicity]
  category   validation
  contract   """
    Edits are planned in order over the texts the project was compiled from.
    The edited project is compiled from what the project holds: the text of
    each source it read, the E025 of each it could not read, and the planned
    texts, reading no source again, so its verdict never depends on a file
    the project could not read; an edit that leaves an error-severity
    diagnostic the project did not have, with every check run (after an
    editor update that skipped the checks they are run for the comparison),
    is refused with E080 and writes nothing (an error is the same error after
    an edit moves it, even when its message names a position). Each edit declares what it may
    change in the graph: set and unset their field (a field.entry path's
    field), link and unlink their field, retitle the title, add-verify and
    remove-verify the obligations, create and delete their entity, a delete
    with unlink_references each referrer's field; an edge is declared by its
    source's declared field or by a created or deleted end. An edit whose
    edited graph changes anything else is refused as undeclared_change and
    writes nothing; a block an edit leaves holding text the parser cannot read whole is refused edit_unreadable, also a
    defect of the edit, and writes nothing; a block that holds such text before the edit is refused block_unreadable; E080 is judged first. Warnings and infos it adds are written and reported. A preview
    refuses exactly what the write would. The files are written as one project
    change: a file changed since the compile refuses the whole plan, and a
    failed write puts every file already written back. The report names the net
    change: the entities whose blocks the list, taken together, creates, changes or deletes, and the
    files whose text it changes; an entity a later edit puts back as it was is not named, and a
    block that only moves within its file is not either.
  """
  verify unit "a graph change no edit declares is beyond the declaration"
  verify unit "every verb's graph change is what it declares"
  verify unit "a block holding a padded value takes a typed edit"
  verify unit "an edit that adds an error is E080 and writes nothing"
  verify unit "a judged rules finding is no error a write adds"
  verify unit "a value holding a control character is refused not_editable and writes nothing"
  verify unit "an error whose message names a position is the same error after an edit moves it"
  verify unit "an edit that adds a warning is written and reports it"
  verify unit "a preview refuses what the write would"
  verify unit "every block an edit touches is format-clean"
  verify unit "a failed write puts every file back"
  verify unit "an edit planned while the checks are skipped is compared with the checked report"
  verify unit "the report names what the edits change taken together"
  verify unit "a create links the names the project already writes for its ID"
  verify unit "a block an edit leaves unreadable is refused edit_unreadable; one unreadable before it, block_unreadable"
}

behavior collect_harness_ledger "Keep the Harness Ledger" {
  invariants [typed_edit_atomicity]
  category   mutation
  contract   """
    Every host write keeps `specforge-harness` (ADR 0287): a planned change carries its ledger rewrite, applied
    under `.specforge/write.lock` (every write of a project that declares a harness field takes it) to the ledger as read, so two writers on disjoint files both land their lines.
    A rename or scope move writes a move record and re-keys its lines. `specforge harness gc` rewrites the ledger
    canonically, and the first ledger write adds `/specforge-harness merge=union` to `.gitattributes`.
  """
  verify unit "harness gc rewrites the ledger canonically and --dry-run writes nothing"
  verify unit "harness gc leaves a ledger alone when no loaded extension declares a harness field"
  verify unit "every host write of a project with a harness field takes the write lock, and one without creates no lock file"
  verify unit "the first ledger write adds the union attribute"
}

behavior apply_edit_plan "Apply an Edit Plan" {
  invariants [typed_edit_atomicity]
  category   mutation
  contract   """
    A command or a collector that declares `writes` may answer an edit plan (ADR 0289): typed operations the host
    checks, plans with the typed edit engine, judges and applies all or nothing, the only writer. A project whose
    extensions declare no writing command, no writing collector, no human-only value and no guard pass reads and
    writes as it did before: a read-only command's input carries no revisions, its tool is announced read-only with
    the schema its arguments derive, its help has no `--dry-run`, and a typed edit writes the same bytes.
  """
  verify unit "a read-only command's input carries no revisions"
  verify unit "a read-only command's tool keeps its annotations and schema"
  verify unit "a read-only command's help and output are unchanged"
  verify unit "a typed edit writes the same bytes and report"
  verify unit "an edit plan and a writing command's input read back as they were written"
  verify unit "a writing declaration reaches the wire and its plan carries the call's revisions"
  verify unit "a writing command receives the revision of every source and its plan is applied"
  verify unit "a plan is all or nothing, the ledger included"
  verify unit "two plans on disjoint files both land with their ledger lines"
  verify unit "a plan may not write a computed value"
  verify unit "a harness value is set only by a plan of its extension or of a peer"
  verify unit "a plan from a command that does not declare writes, or that failed, is not applied"
  verify unit "a plan renames in order and re-keys the ledger once"
  verify unit "a malformed plan is invalid_plan and names the operation"
  verify unit "a writing command previews its plan with --dry-run and writes it otherwise on the command line"
  verify unit "a writing command's plan and the file it writes agree on the command line and over MCP"
  verify unit "a cite naming more than one thing is not a cite"
  verify unit "an edit naming a revision for an entity the compile does not hold is refused"
  verify unit "a writing collector's plan is previewed with dry_run over MCP"
  verify unit "set_verdict is refused without a judged rule"
  verify unit "set_verdict checks the rule and its basis"
  verify unit "set_verdict is refused on a human-only kind and for a computed value"
  verify unit "set_verdict compares its inputs with the disk under the lock"
  verify unit "set_verdict writes its verdict and its ledger line"
  verify unit "set_verdict refuses a value its field refuses"
  verify unit "set_verdict refuses a finding whose member name a spec block cannot write"
  verify unit "format does not overwrite a concurrent write"
  verify unit "format locks a project with a writing extension and not one that merely has a .specforge directory"
}

behavior refuse_stale_revision "Refuse a Stale Revision" {
  invariants [typed_edit_atomicity]
  category   validation
  contract   """
    An operation of an edit plan names the revision of its file: `sha256:<hex>` of the bytes the host compiled and
    sent, or `absent` for a file it creates (ADR 0289). A plan whose file changed since is refused `stale_revision`
    with the file, the revision planned on and the current one, and writes nothing; a plan that names no revision
    (a core typed edit) is planned on the current compile, as before.
  """
  verify unit "a typed edit naming no revision is refused file_changed when its file changed since the compile"
  verify unit "a plan naming a stale revision is refused and writes nothing"
  verify unit "two plans whose union adds an error refuse the later"
  verify unit "a core edit names the revision it was planned on and is refused when it is stale"
}

behavior roll_back_interrupted_write "Roll Back an Interrupted Write" {
  invariants [typed_edit_atomicity]
  category   mutation
  contract   """
    A write of several files is journaled under the write lock before its first file is replaced (ADR 0289). The next
    writer that finds the journal restores each file that holds the bytes the interrupted write was putting there,
    leaves a file that holds what it held before, and notes both; a project check reports a journal left behind as
    W167 and writes nothing.
  """
  verify unit "an interrupted write is rolled back by the next writer"
  verify unit "an interrupted write is W167 until the next writer rolls it back"
  verify unit "a session reports a journal that appears and drops one that goes"
  verify unit "a pending journal another writer holds is no error this write adds"
  verify unit "rollback restores exactly the files at their after digest"
  verify unit "a typed edit of a project with a writing command and no harness field takes the write lock"
  verify unit "a journal marked done is removed by the next writer and rolls nothing back"
  verify unit "a journal naming a path outside the project root or a malformed digest is refused and restores nothing"
  verify unit "rollback restores a removed file and removes a created host file, and refuses a path no write records"
  verify unit "rollback leaves a removed file alone when something else was written at its name since"
  verify unit "rollback refuses a journal that names a file no write of the project records"
  verify unit "the next install rolls back an install killed between two writes before it plans"
  verify unit "a journal never restores a host config file, which runs code"
  verify unit "a journal never restores a host config file, which runs code, through an install"
  verify unit "the journal keeps no text of a host config file"
}

behavior refuse_human_only_write "Refuse a Human-Only Write" {
  invariants [typed_edit_atomicity]
  category   validation
  contract   """
    A kind, a field or a member an extension declares human-only is written by a person: a plan of a command or a
    collector, an MCP edit and a command line without a terminal are refused `human_only` and write nothing, and the
    command line at a terminal writes it (ADR 0289).
  """
  verify unit "a human-only value is refused in an agents write"
  verify unit "the command line writes a human-only value from a terminal and refuses it without one"
  verify unit "a human-only or computed member is refused in the value of a set, append or create"
  verify unit "a delete that unlinks references and a rename do not change a human-only entity"
  verify unit "an agents set, unset or link of an existing human-only entity is refused"
  verify unit "an agents delete of an entity holding a persons value is refused"
}
