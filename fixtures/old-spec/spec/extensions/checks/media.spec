// @specforge/checks: the media family

use "extensions/checks/features"

behavior checks_media "Media Family" {
  features [ck_value_checks]
  category command
  ensures {
    formats_known  "An image, svg, pdf or media value's file is one, of a format its check allows, else E099"
    active_refused "An svg value's file carries no script, event attribute or javascript: URL, else E113 at its line"
    limits_held    "A file within max_bytes, max_pixels and max_pages, when the check names them, else W177"
    reads_bounded  "A media check reads a file's head, an svg to 1 MiB, a pdf's head, tail and at most two rounds of spans"
  }
  contract """
    Each media checker (image, svg, pdf, media) MUST read only what the
    host's probes answer: the file its value names, under the root its
    `root` argument or the value's `<name>/` prefix names, through the
    `files` scope, or an https URL through the `media-remote` scope,
    which reaches only the hosts the values name. `image` and `media`
    read the first 64 KiB: an image is known by its signature (png, jpeg,
    gif, webp, avif, heic, bmp, ico, tiff) or an svg root, and a media
    file by its container (mp4, mov, m4a, webm, mkv, avi, wav, mp3, ogg,
    flac, aac); one that is no such file, or outside the check's
    `formats`, MUST be E099. `svg` reads the file whole, at most 1 MiB
    (a larger one is W176, not checked): XML that is not well formed, or
    whose root is not `<svg>`, MUST be E099, and a `<script>` element, an
    `on...` attribute, a `javascript:` URL in an `href` or in what an
    animation sets, or an entity not expanded where it could hide one,
    MUST be E113 naming the first and its line and column. `pdf` reads
    the first 1 KiB and the last 64 KiB, follows the cross-reference to
    the page tree, and when `max_pages` is asked reads the spans it still
    needs in at most two more rounds; a page count it cannot read leaves
    `max_pages` unchecked (W176). A file over `max_bytes`, an image over
    `max_pixels` or a pdf over `max_pages` MUST be W177; no limit is set
    unless named. A URL that answers 404 or 410 is E099; one the network
    did not serve, or a server that ignored the range past 1 MiB, is
    W172, W173, W181 or W182, never E099. A file is untrusted: no entity
    is expanded, nesting, counts and inflated bytes are capped, and no
    input panics. The `families.media` switch gates E099, E113 and W177.
  """
  verify unit "a media checker reads a path under its root and an https URL by ranges, and refuses other schemes"
  verify unit "an image of a format not allowed is E099, and one over its pixels or bytes is W177"
  verify unit "an svg with active content is E113 at its line, and one not well formed or not svg is E099"
  verify unit "a media file is known by its container, and one of another container or none is E099"
  verify unit "a pdf reads its head and tail, asks for the spans its page tree needs, then counts its pages against max_pages"
  verify unit "a remote file the network did not serve is a machine warning, and one that is gone is E099"
  verify integration "the repository's own brand images and logos pass image and svg"
  verify integration "an svg with an onload attribute is E113 at its line"
  verify integration "a png over max_pixels is W177"
  verify integration "a pdf over max_pages is W177"
  verify integration "a remote pdf is read by its tail through media-remote"
  verify integration "an unapproved remote image is W170 and I206, not a finding"
}
