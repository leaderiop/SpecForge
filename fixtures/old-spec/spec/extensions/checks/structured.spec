// @specforge/checks: the structured family

use "extensions/checks/features"

behavior checks_structured "Structured Family" {
  features [ck_value_checks]
  category command
  ensures {
    files_parse       "A json, yaml or toml value's file parses as its format, else E097 at the parser's line and column"
    fragments_resolve "A pointer or a Markdown anchor names something in its file, else E098 on the fragment"
    reads_bounded     "No structured check reads more than 1 MiB of a file; a larger file is W176 and is not checked"
  }
  contract """
    Each structured checker (json, yaml, toml, pointer, markdown_anchor)
    MUST read only what the host's probes answer: the file its value
    names, under the root its `root` argument or the value's `<name>/`
    prefix names, at most 1 MiB of it. A file that does not parse as its
    format, or is not UTF-8, MUST be E097 naming the parser's reason and
    the line and column. A `pointer` value is a path and an RFC 6901
    pointer in its fragment form (`a.json#/a/0`): a pointer that names no
    member of the file's tree MUST be E098 on the `#` part, naming the
    deepest part that exists; a malformed one is E103, and a file whose
    format neither its extension nor the `format` argument says is E097.
    YAML and TOML are read into the tree JSON is, so one pointer reads all
    three, YAML aliases are counted before they are copied, and nesting
    stops at 128 levels. A `markdown_anchor` value's `#anchor` MUST be a
    heading's GitHub slug, an explicit `{#id}` or an HTML `id` or `<a
    name>` of the file, else E098 on the `#` part listing up to eight of
    its headings; a `#` line in a code block names no anchor. A missing
    file is E016, and the `families.structured` switch gates E097 and E098.
  """
  verify unit "json, yaml and toml read their file whole, up to 1 MiB, under its root"
  verify unit "a file that parses passes, a byte order mark included, and one that does not is E097 at its line"
  verify unit "a missing file is E016, and one over the read or no regular file is W176"
  verify unit "a pointer that names nothing is E098 on its fragment, naming the deepest part that exists"
  verify unit "a pointer that is malformed is E103 on its token, and one whose format is unknown is E097"
  verify unit "a markdown anchor that is no heading or html id is E098 on its fragment, listing the headings"
  verify integration "a json file that does not parse is E097 at its line"
  verify integration "a missing pointer is E098 on the fragment"
  verify integration "a missing heading is E098 listing the headings"
  verify integration "a structured file over one MiB is W176"
}
