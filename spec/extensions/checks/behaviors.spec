// @specforge/checks behaviors

behavior ck_family_switches "Family Switches" {
  features [ck_value_checks]
  category command
  contract """
    @specforge/checks MUST declare one bool setting per family, on by
    default. While a family's setting is false its checkers MUST NOT be
    called, send no probe and add no scope to what the run asks approval
    for, and the setting MUST gate the codes only that family reports.
  """
  verify unit "the extension declares the scopes files, links and git, and five family switches on by default"
  verify unit "each switch gates only codes its family alone reports, and E103 under no switch"
  verify unit "every code a checks switch gates is catalogued with owner @specforge/checks"
  verify unit "a family whose switch is off reports nothing"
}

behavior ck_formats_family "Formats Family" {
  features [ck_value_checks]
  category command
  contract """
    Each formats checker MUST decide a value from its text alone, by the
    standard it names, and report E103 on the part of the value's span the
    kit points at. Each list item MUST be judged alone, and an empty value
    MUST be left to the kernel's `required`.
  """
  verify unit "a format that refuses a value reports E103 on the part of the value's span the kit points at"
}

behavior ck_digits_family "Digits Family" {
  features [ck_value_checks]
  category command
  contract """
    Each digits checker MUST decide a value from its text alone, MUST name
    the check digit it expected, and MUST need no scope and no approval.
  """
  verify unit "a digit checker needs no scope and asks no approval"
}

behavior ck_files_family "Files Family" {
  features [ck_value_checks]
  category command
  ensures {
    missing_files_diagnosed "Every path a files check names that is not there is E016 on its value"
    existing_files_pass     "A path that is there, and is what its checker asks for, reports nothing"
    paths_contained         "A path that leaves its root is E094, reported by the host, and is not read"
  }
  contract """
    Each files checker (file, dir, glob, size, digest, encoding,
    line_range) MUST read only what the host's probes answer, under the
    root its `root` argument names (the project root when it names none)
    or the root a `<name>/` prefix names, and MUST report a missing file
    as E016 on the value's own span, with a sibling only the host found
    inside the root. A check attached to a field reads that field on the
    kinds it is attached to only. A single value's text, quoted or a bare
    word, is one path; each item of a list is its own. `glob` reports
    fewer than `min` matches as E016 and more than `max` as W176; `size`
    and `encoding` report W176; `digest` reports a file whose SHA-256 is
    another than its pin as E096 and a malformed pin as E103; `line_range`
    reports a range past the end as W176, and a file cut off at its read
    cap whose range is not covered is not decided. A path that leaves its
    root is E094 and one that exists under another case W174, reported by
    the host before anything is read.
  """
  verify unit "a missing file in a list reports E016 on that item's span, not the entity's"
  verify unit "a field of the same name on another kind is not checked"
  verify unit "a single path on a string field is checked like a one-item list"
  verify integration "a compile checks a bare word as a path"
  verify unit "a path is read from the root its check names and suggests a close sibling there"
  verify unit "a check whose root is the project reads its relative paths from the directory of specforge.json"
  verify unit "a sibling is suggested only from inside the root"
  verify unit "a file whose digest differs from its pin is E096"
  verify unit "a line range past the end of the file is W176"
}

behavior ck_links_family "Links Family" {
  features [ck_value_checks]
  category command
  ensures {
    malformed_urls_unfetched "A URL the text alone refuses is E091 or E092 and is never requested"
    definite_answers_errors  "Only a definite answer (404 or 410) is an error; every other failure is a warning"
  }
  contract """
    Each links checker (url, url_anchor, content_type, tls, redirect,
    dns) MUST read only what the host's probes answer, under the links
    scope. A malformed or relative URL, or one that carries credentials,
    MUST be E091, and a scheme outside `schemes` (default `https`;
    `javascript`, `data` and `file` never) E092 on the scheme, both
    without a request. After at most 5 redirects, 404 or 410 is E093, or
    W183 on a field whose check says `status: "warn"`; 401 and 403 are
    W182; 429, 5xx and any other non-success W181; a failure to reach
    the host W172; and a redirect not followed W173. A HEAD answered 405
    or 501 is asked once more as a GET. `url_anchor` reads at most 1 MiB
    and reports a fragment no `id` or `name` matches as E112 on the
    fragment; a page cut off at that cap is not decided. `content_type`
    reports a type outside `allow`, or none, as E104; `tls` a certificate
    ending within `days` (default 14) as W178; `redirect` a final host
    outside `hosts` as W173; `dns` a host that does not resolve as W172
    and a malformed host name as E103.
  """
  verify unit "a URL that answers 404 is E093, and W183 on a field whose check lowers it"
  verify unit "without approval only the links scope is asked for"
  verify unit "a links family switched off sends no request"
  verify unit "the remote's answers are machine-decided warnings, the project's own are not"
  verify unit "a HEAD answered 405 or 501 is asked once more as a GET"
}

behavior ck_git_family "Git Family" {
  features [ck_value_checks]
  category command
  ensures {
    names_parsed_first "A name the kit refuses is E103 and never reaches git"
    missing_names_e106 "A commit, ref or file at a revision the repository lacks is E106 on its value"
  }
  contract """
    Each git checker (commit, ref, file_at, tracked) MUST read only what
    the Git port answers, under the git scope. `commit` takes an object
    name of at least `min_len` (default 7) hex digits; `ref` a ref name,
    looked up under refs/tags, refs/heads or both as its `kind` says;
    `file_at` a `<revision>:<path>` whose path is from the repository's
    top and holds no `.` or `..`. A malformed value is E103, a name the
    repository lacks E106, and a `tracked` path the commit at HEAD does
    not hold W180. A repository that cannot be asked decides nothing.
  """
  verify unit "a commit, ref or file at a revision the repository lacks is E106"
  verify unit "a path the commit at HEAD does not hold is W180"
}
