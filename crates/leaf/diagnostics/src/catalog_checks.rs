//! The diagnostic codes of `@specforge/checks` (ADR 0338): a table `catalog.rs` merges into `CATALOG`, sorted by code.

catalog! {
    E016: Error checks,
        "Referenced file does not exist",
        "A `@specforge/checks` files check (`file`, `dir`, `glob`, `size`, `digest`, `encoding`, `line_range`) names a path that is not there under the root it reads: the project root, or the root its `root` argument or the value's `<name>/` prefix names. `file` also reports a directory and `dir` a file, and `glob` matching fewer than its `min` entries. It sits on the value. A path that leaves its root is E094 and one that exists under another case is W174, both reported by the host before anything is read. Fix the path, or create the missing file; a similarly named file in the path's own directory inside its root is suggested when one is found.";
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
    E103: Error checks,
        "The value is not well formed",
        "A format, a check digit, or the name a files or git check reads is malformed by the standard its checker names (`@specforge/checks`, ADR 0338): the message names the standard, or the digit it expected, and the finding points at the part that is wrong when it can. Correct the value. No family switch gates it, since several families report it; turning a family's switch off stops its checkers.";
    E104: Error checks,
        "The URL's content type is not allowed",
        "A `@specforge/checks:content_type` value answered a `Content-Type` outside the check's `allow`, or none. Point the link at the right resource, or add the type to `allow`.";
    E106: Error checks,
        "A commit, ref or file at a revision is not found",
        "A `@specforge/checks` git check (`commit`, `ref`, `file_at`) names something this repository does not have. Fix the name, or fetch what it names: a shallow clone may lack it.";
    E112: Error checks,
        "The URL's fragment is not in the page",
        "A `@specforge/checks:url_anchor` value's `#fragment` names no `id` attribute, and no `name` of an `<a>`, in the page as served, read to its first 1 MiB with no script run. A page cut off at that cap with no match is not decided. Fix the fragment or the page.";
    W172: Warning checks machine,
        "The URL or name did not answer",
        "The DNS lookup, the connection, the TLS handshake or the wait for an answer failed, or a `@specforge/checks:dns` name does not resolve. It says nothing about the project, only what this machine's network saw, so retry later or check the network. A `machine` diagnostic: `--strict` never promotes it, and the gate's warning baseline never counts it.";
    W173: Warning checks machine,
        "The URL redirected where it was not followed",
        "A redirect left the hosts this run is granted, downgraded https to http or went past 5 hops, or a `@specforge/checks:redirect` value ended on a host outside `hosts`. Point the link at the final URL, or widen `hosts` if the destination is intended. A `machine` diagnostic: `--strict` never promotes it, and the gate's warning baseline never counts it.";
    W176: Warning checks,
        "A file is not what its check asks",
        "A `@specforge/checks` files check found a file over its `size`, in another `encoding`, a `line_range` past the file's end, or more matches of a `glob` than its `max`. Fix the file, or the check's arguments.";
    W178: Warning checks,
        "The URL's certificate expires soon",
        "A `@specforge/checks:tls` value's certificate has a `not_after` within the check's `days` (14 by default). Renew it, or lower `days`.";
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
