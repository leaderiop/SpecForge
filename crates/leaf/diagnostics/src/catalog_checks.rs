//! The diagnostic codes of `@specforge/checks` (ADR 0338): a table `catalog.rs` merges into `CATALOG`, sorted by code.

catalog! {
    E016: Error checks,
        "Referenced file does not exist",
        "A `@specforge/checks` files, structured or media check (`file`, `dir`, `glob`, `size`, `digest`, `encoding`, `line_range`, `json`, `yaml`, `toml`, `pointer`, `markdown_anchor`, `image`, `svg`, `pdf`, `media`) names a path that is not there under the root it reads: the project root, or the root its `root` argument or the value's `<name>/` prefix names. `file` also reports a directory and `dir` a file, and `glob` matching fewer than its `min` entries. It sits on the value. A path that leaves its root is E094 and one that exists under another case is W174, both reported by the host before anything is read. Fix the path, or create the missing file; a similarly named file in the path's own directory inside its root is suggested when one is found.";
    E091: Error checks,
        "The URL is not valid",
        "A field whose check is `@specforge/checks:url` (or another of the links family) holds a text that is no absolute URL, or one that carries a user name or password. It is never fetched. Write the whole URL with its scheme, and keep credentials out of it: a value is committed, and an approved secret is the way to send one.";
    E092: Error checks,
        "The URL's scheme is not allowed",
        "The URL's scheme is outside the check's `schemes` (by default only `https`). `javascript:`, `data:` and `file:` are never allowed, whatever `schemes` says. It is never fetched. Change the URL, or add its scheme to the check's `schemes`.";
    E093: Error checks,
        "The URL answered that the page is gone",
        "After at most 5 redirects, the URL answered 404 or 410: a definite answer that the page does not exist. Fix the link or remove it. A field whose check says `status: \"warn\"` reports W183 instead.";
    E096: Error checks,
        "A file differs from its pinned digest",
        "A `@specforge/checks:digest` value reads `path@sha256:<64 hex digits>`, and the file's SHA-256 is another. If the change is intended, write the new digest; if not, restore the file. A missing file is E016, and a malformed pin is E103.";
    E097: Error checks,
        "The file does not parse",
        "A `@specforge/checks` structured check (`json`, `yaml`, `toml`, or a `pointer`'s file) read a file that does not parse as its format, or is not UTF-8: the message names the parser's reason and the line and column. A `pointer` whose file's format its extension does not say, and whose `format` argument names none, is E097 too. Fix the file, or name its `format`. A file over the 1 MiB a structured check reads is W176 and is not checked.";
    E098: Error checks,
        "The pointer or anchor names nothing in its file",
        "A `@specforge/checks:pointer` value's JSON pointer (`path#/a/0`, RFC 6901 in its fragment form) names no member of the file's tree, or a `markdown_anchor` value's `#anchor` is no heading slug, `{#id}` or HTML `id` of the Markdown file, as GitHub renders them. The finding sits on the `#` part: a pointer's message names the deepest part that exists, and an anchor's help lists the file's headings. Fix the fragment, or the file.";
    E099: Error checks,
        "The image, SVG, PDF or media file is not one, or not of an allowed format",
        "A `@specforge/checks` media check read the file a value names, under its root or from an https URL, and it is not what the check reads: `image` found no image signature (or a whole file whose size does not read) or a format outside its `formats` (by default png, jpeg, webp, gif, svg, avif); `svg` found XML that is not well formed, or whose root is not `<svg>`; `pdf` found no `%PDF-` header, no `startxref` or a cross-reference that does not read; `media` found no container it knows (mp4, mov, m4a, webm, mkv, avi, wav, mp3, ogg, flac, aac) or one outside its `formats`. A URL that answers 404 or 410, or names another scheme than https, is E099 too. Fix the file or the value, or allow its format. A network that did not answer is W172, not E099.";
    E103: Error checks,
        "The value is not well formed",
        "A format, a check digit, the name a files, git or pointer check reads, the URL or `formats` of a media check, or a registries check's package, DOI or ORCID iD is malformed by the standard its checker names (`@specforge/checks`, ADR 0338): the message names the standard, or the digit it expected, and the finding points at the part that is wrong when it can. Correct the value. No family switch gates it, since several families report it; turning a family's switch off stops its checkers.";
    E104: Error checks,
        "The URL's content type is not allowed",
        "A `@specforge/checks:content_type` value answered a `Content-Type` outside the check's `allow`, or none. Point the link at the right resource, or add the type to `allow`.";
    E105: Error checks,
        "The package, release, DOI or ORCID iD is not registered",
        "A `@specforge/checks` registries check asked the identifier's own resolver and got a definite no: `package`, `advisory` or `license_of` found no such package on crates.io, npm or PyPI (a 404), or no release its requirement admits once yanked releases are left out (the message names the newest release that is not yanked, and the finding sits on the requirement); `doi` found a DOI doi.org does not register (`responseCode` 100, or a 404); `orcid` found an iD ORCID has no record of (404 or 410). Fix the value. A registry that is down, limits the rate or wants credentials is W181 or W182, and one that did not answer W172, never E105.";
    E106: Error checks,
        "A commit, ref or file at a revision is not found",
        "A `@specforge/checks` git check (`commit`, `ref`, `file_at`) names something this repository does not have. Fix the name. A miss git cannot vouch for is not reported: with no repository, in a shallow clone, or for a tag in a clone that fetched none, the check stays unchecked (I207).";
    E112: Error checks,
        "The URL's fragment is not in the page",
        "A `@specforge/checks:url_anchor` value's `#fragment` names no `id` attribute, and no `name` of an `<a>`, in the page as served, read to its first 1 MiB with no script run. A page cut off at that cap with no match is not decided. Fix the fragment or the page.";
    E113: Error checks,
        "The SVG carries active content",
        "A `@specforge/checks:svg` value's file holds a `<script>` element, an event attribute (`onload`, `onclick` and every other `on…`), a `javascript:` URL in an `href` or in what an animation sets, or an entity the check does not expand where one could hide either. The message names the first one and its line and column. An SVG shown through an `<img>` runs no script, but one opened, inlined or embedded does: remove it.";
    W172: Warning checks machine,
        "The URL or name did not answer",
        "The DNS lookup, the connection, the TLS handshake or the wait for an answer failed, a `@specforge/checks:dns` name does not resolve, or a server answered a media check's request for part of a file with all of it, past the 1 MiB that is read, or a registry's answer was over the 16 MiB read or did not read as its format. It says nothing about the project, only what this machine's network saw, so retry later or check the network. A `machine` diagnostic: `--strict` never promotes it, and the gate's warning baseline never counts it.";
    W173: Warning checks machine,
        "The URL redirected where it was not followed",
        "A redirect left the hosts this run is granted, downgraded https to http or went past 5 hops, or a `@specforge/checks:redirect` value ended on a host outside `hosts`. Point the link at the final URL, or widen `hosts` if the destination is intended. A `machine` diagnostic: `--strict` never promotes it, and the gate's warning baseline never counts it.";
    W176: Warning checks,
        "A file is not what its check asks",
        "A `@specforge/checks` files check found a file over its `size`, in another `encoding`, a `line_range` past the file's end, or more matches of a `glob` than its `max`; a structured or `svg` check found a file over the 1 MiB it reads, or one that is no regular file; or a `pdf` check asked for `max_pages` could not read the page count (its page tree in an encrypted object stream, or past what three rounds of reads reach), so the limit is not checked. Fix the file, or the check's arguments.";
    W177: Warning checks,
        "The image, SVG, PDF or media file is over its limit",
        "A `@specforge/checks` media check's file is longer than its `max_bytes`, an image has more pixels than its `max_pixels`, or a PDF has more pages than its `max_pages`. No limit is set unless the check names it. Shrink the file, or raise the limit.";
    W178: Warning checks,
        "The URL's certificate expires soon",
        "A `@specforge/checks:tls` value's certificate has a `not_after` within the check's `days` (14 by default). Renew it, or lower `days`.";
    W179: Warning checks,
        "The package release has an advisory, or a licence that is not allowed",
        "A `@specforge/checks:advisory` value's package resolves to a release OSV lists an advisory for (withdrawn ones are not reported): the message names its id, its aliases and the first version a range of it names as fixed. Or a `license_of` value's release states a licence its `allow` does not admit (an SPDX expression holds when the licences `allow` names satisfy it), or states none as SPDX. Upgrade or replace the package, or widen `allow` if the licence is accepted.";
    W180: Warning checks,
        "A path is not tracked by git",
        "A `@specforge/checks:tracked` value names a path that is not in the commit at HEAD: it is ignored, untracked or only staged. Commit it, or stop naming it.";
    W181: Warning checks machine,
        "The URL answered with another status",
        "After at most 5 redirects, the URL answered 429, a 5xx or another status that is neither success nor gone. Whether the page exists is not decided, so retry later. A `machine` diagnostic: `--strict` never promotes it, and the gate's warning baseline never counts it.";
    W182: Warning checks machine,
        "The URL needs credentials",
        "The URL answered 401 or 403: the page needs credentials this machine did not send, so whether it exists is not decided. Approve a secret for the extension if it should be read, or drop the check. A `machine` diagnostic: `--strict` never promotes it, and the gate's warning baseline never counts it.";
    W183: Warning checks,
        "The URL answered that the page is gone (lowered)",
        "After at most 5 redirects, the URL answered 404 or 410, on a field whose check says `status: \"warn\"`, so the project asks for a warning rather than E093. Fix the link or remove it.";
}
