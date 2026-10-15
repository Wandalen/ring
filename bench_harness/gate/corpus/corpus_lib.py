"""Shared parsing for the corpus gates G14-G17.

One module rather than four, because all four gates answer questions about the
same three structures — the definition directories, the finding rows inside each
definition readme, and the Module Index that restates both — and four private
parsers of the same markdown is four places for the parsers to disagree. When
they disagree the gates do not fail; they report different counts for the same
tree, which is worse.

Every reader here is fence-aware. A findings table row and a line inside a ```sh
block are indistinguishable to a regex, and this corpus publishes recipes that
grep for the very row shapes these functions match — `command grep -n '^| F[0-9] |'`
appears inside a fenced block in `ring_registry/docs/pitfall/readme.md` and would
otherwise be parsed as a finding row of a definition that does not exist.
"""

import pathlib
import re


# A finding heading: `### RG41 — The Clone Is Forced, and Nothing Says So`.
# Two uppercase letters is the family's crate prefix; the number is not padded.
FINDING_HEADING = re.compile( r'^### ([A-Z]{2}\d+) — ' )

# A row of a definition readme's `### Findings Recorded Here` table:
# `| RG49 | `ring_registry` | n/a — doc gap | ... |`
DEFN_ROW = re.compile( r'^\| ([A-Z]{2}\d+) \| (.*?) \| (.*?) \| (.*) \|$' )

# A row of the Module Index's `## Findings` table. Column order differs from the
# definition readme's on purpose. Subject and Tier swap places, and a fifth
# `Where` column carries the link. Two orders for one fact is a real hazard, and
# G16 is the gate that holds them equal.
MASTER_ROW = re.compile(
  r'^\| ([A-Z]{2}\d+) \| (.*?) \| (.*?) \| (.*?) \| \[([a-z_]+/\d+)\]\(([^)]+)\) \|$'
)

# An instance filename: `001_two_branches_and_what_the_refusal_costs.md`.
INSTANCE_NAME = re.compile( r'^(\d\d\d)_.*\.md$' )

# Any ATX heading, captured so its depth (the run of `#`) can be compared
# against another heading's. See `_section_rows`. Public (no leading
# underscore): `citations.py`'s own `### Tests` section scan needs the same
# depth comparison and would otherwise duplicate this regex rather than the
# parsing loop around it, which has a shape of its own there (folded into a
# larger per-instance loop that also tracks `rel` and collects problems).
HEADING = re.compile( r'^(#+) ' )


def prose_lines( path ):
  """Yield ( lineno, text ) for every line outside a fenced code block.

  Finding(prose_lines_blockquoted_fence_invisible), not fixed here: the
  toggle below matches only a fence opened at column 0. A fence written
  `> ```sh` — inside a blockquote, as this corpus does five times today (two
  doctest quotations in `ring_atomic`, one shell recipe in
  `ring_flush/docs/decisions/readme.md`, and one each in `ring_mpsc`'s and
  `ring_tls`'s own `docs/readme.md`) — never starts with a backtick, so the
  toggle never fires and every line inside such a quote reads as prose
  instead of fenced content, the same hazard this module's own header names
  for a bare fence, just for a shape it didn't call out. Not exploited
  today: measured directly against all five real blockquoted fences in this
  corpus, none contains a line matching FINDING_HEADING, DEFN_ROW,
  MASTER_ROW, or a bare `#` at column 0.

  Widening the toggle to also match a blockquote/indent prefix
  (`^[ \\t>]*```') was tried and reverted — it breaks a case this corpus
  also has: `ring_flush/docs/integration/001_two_dependencies_....md` quotes,
  as illustrative body text at 4-space indent, an unclosed nested fence
  marker (`    ```toml` at line 379) *inside* an already-open real fence
  (bare, opened line 369, closed line 390). CommonMark treats indented text
  between a real fence's own open and close as literal content, never a
  second toggle point — but the widened regex can't tell "genuinely opens a
  fence" from "quotes what one looks like," so it toggled on the indented
  marker anyway, closing the outer fence early, then toggled twice more on
  two further indented markers a few lines later (`    ```bash ...`, `
  ```sh ...`), leaving the file's fence-parity flipped for everything after
  line 390 — 541 lines, including the real `### FL18 — ...` heading at line
  463, which `finding_owners()` then silently stopped finding. Measured, not
  assumed: a direct trace of both versions of the toggle over that file
  shows 38 toggles (even, ends outside) for the version below versus 41
  (odd, ends inside) for the widened one. A correct fix needs to track the
  opening delimiter's own indentation and match it on close, the way a real
  CommonMark parser does, rather than a bare boolean toggle — worth doing,
  not worth doing as a drive-by here.
  """
  inside = False
  for ( i, line ) in enumerate( pathlib.Path( path ).read_text().splitlines(), 1 ):
    if line.startswith( '```' ):
      inside = not inside
      continue
    if not inside:
      yield ( i, line )


def read_standard( path ):
  """Load `corpus_standard.txt` into { key: [values] }, comments stripped.

  Returned values stay lists even where one is expected, so a duplicated
  `min_findings` line is visible to the caller as a two-element list rather than
  silently resolving to whichever came last.
  """
  out = {}
  for line in pathlib.Path( path ).read_text().splitlines():
    line = line.strip()
    if not line or line.startswith( '#' ):
      continue
    ( key, _, value ) = line.partition( ' ' )
    out.setdefault( key, [] ).append( value.strip() )
  return out


def definition_dirs( docs ):
  """Every subdirectory of a crate's `docs/`, sorted. Includes `definition/`."""
  return sorted( p.name for p in pathlib.Path( docs ).iterdir() if p.is_dir() )


def instances( docs ):
  """Every doc instance under `docs/`, as ( definition, id, path ), sorted.

  `item/` is the one type directory that nests one level further: alongside its
  own direct instances (a whole-crate item census, say), it groups per-item
  instances by kind (`item/struct/001_...md`, `item/enum/002_...md`). Every
  other type directory is flat by convention — confirmed by grep, not assumed —
  so `item/`'s subdirectories are walked in addition to its direct children,
  never instead of them, and no other type directory is walked any deeper than
  before.
  """
  out = []
  for d in pathlib.Path( docs ).iterdir():
    if not d.is_dir():
      continue
    for p in sorted( d.iterdir() ):
      if p.is_dir():
        if d.name != 'item':
          continue
        for q in sorted( p.iterdir() ):
          m = INSTANCE_NAME.match( q.name )
          if m:
            out.append( ( d.name, m.group( 1 ), q ) )
        continue
      m = INSTANCE_NAME.match( p.name )
      if m:
        out.append( ( d.name, m.group( 1 ), p ) )
  return sorted( out )


def finding_owners( docs ):
  """Map finding id -> the instance path whose `### ID — ` heading declares it.

  Headings, not table rows, are the authority for where a finding lives: a row
  can be copied into any table, and several deliberately are. Only one file
  carries the finding's own body.
  """
  owner = {}
  for ( _, _, p ) in instances( docs ):
    for ( _, line ) in prose_lines( p ):
      m = FINDING_HEADING.match( line )
      if m:
        owner[ m.group( 1 ) ] = p
  return owner


def _section_rows( path, heading, pattern ):
  """Rows matching `pattern` between `heading` and the next heading at its
  level or shallower — a deeper heading nests inside the section rather than
  closing it.

  Fix(a_deeper_heading_is_not_a_boundary): the close check used to be
  `line.startswith( heading.split( ' ' )[ 0 ] )`, i.e. a bare `#`-run with no
  trailing space — `'####'.startswith('###')` is true in Python, so a `####`
  heading closed a `###`-level section and a `###` heading closed a
  `## Findings` section, both wrongly, since a deeper heading nests inside
  the shallower one it follows rather than ending it. Root cause: the
  intended check was "next heading of its level," but a bare prefix compares
  character runs, not heading depth, and every one of this corpus' `###`
  narrative subsections (`### Severity`, `### Regenerate`, ...) that follow a
  `## Findings` table satisfies that wrong prefix test. Pitfall: every
  `## Findings` table currently ends before its first `###` subsection, so
  today's corpus loses no rows either way — the bug is live only for a
  findings table an author ever splits under a nested heading, or interrupts
  with one before the table ends, and nothing would flag the loss since a
  short table still parses as a clean, merely smaller one. `disposition.py`'s
  own `sections()` already gets this right one file over, via an exact
  `line.startswith( '### ' )` — with the trailing space — since finding
  sections are always exactly three deep; this generalises that same exact-
  depth comparison to the two callers below, whose target heading depth
  varies between two and three.

  Fix(a_longer_heading_is_not_the_same_heading): the open check used to be
  `line.startswith( heading )` — a prefix test on the *opening* boundary,
  mirroring the same mistake the fix above already removed from the
  *closing* boundary. `'## Findings Recorded, Not Fixed'.startswith(
  '## Findings' )` is true, so that heading was read as another instance of
  the `'## Findings'` section opening rather than the different, later
  heading it is — and because the branch `continue`s, it skips the very
  closing-boundary check two lines below that would otherwise have correctly
  ended the section right there (the near-miss is itself `##`-depth, equal
  to the target, so `lvl <= depth` would have fired). The section then runs
  uninterrupted past the near-miss, into content it was never meant to
  include. Latent, not exploited: two real instances exist today — `##
  Findings Recorded, Not Fixed` in `ring_align` and `ring_cursor`'s own
  `docs/definition/readme.md` — and in both, the wrongly-swallowed trailing
  content uses single-letter row-id prefixes (`F1`, `O1`, ...) or a
  differently-shaped table, neither of which satisfies `MASTER_ROW`'s
  two-letter `[A-Z]{2}\\d+` id group, so live `master_findings()` output for
  both crates is unaffected (confirmed by direct execution: 52/52 rows for
  `ring_align`, 53/53 for `ring_cursor`, both exactly right). Fix: the target
  `heading` is always one specific named heading, not a class of headings —
  unlike `disposition.py`'s deliberate `'### '` class-prefix, an exact
  line match is what "this is the section's own opening" actually means.
  """
  if not pathlib.Path( path ).exists():
    return []
  out = []
  inside = False
  depth = len( HEADING.match( heading ).group( 1 ) )
  for ( i, line ) in prose_lines( path ):
    if line == heading:
      inside = True
      continue
    if inside:
      m = HEADING.match( line )
      if m and len( m.group( 1 ) ) <= depth:
        inside = False
    if inside:
      m = pattern.match( line )
      if m:
        out.append( ( i, m ) )
  return out


def declared_findings( defn_readme ):
  """Rows of one definition readme's `### Findings Recorded Here` table.

  Returns [ ( lineno, id, subject, tier ) ].
  """
  rows = _section_rows( defn_readme, '### Findings Recorded Here', DEFN_ROW )
  return [ ( i, m.group( 1 ), m.group( 2 ), m.group( 3 ) ) for ( i, m ) in rows ]


def master_findings( docs ):
  """Rows of the Module Index's `## Findings` table.

  Returns [ ( lineno, id, finding, subject, tier, where ) ].
  """
  index = pathlib.Path( docs ) / 'definition' / 'readme.md'
  rows = _section_rows( index, '## Findings', MASTER_ROW )
  return [
    ( i, m.group( 1 ), m.group( 2 ), m.group( 3 ), m.group( 4 ), m.group( 5 ) )
    for ( i, m ) in rows
  ]


def crate_name( docs ):
  """`ring_registry` from `.../ring/ring_registry/docs`."""
  return pathlib.Path( docs ).parent.name


def report( crate, problems ):
  """Print one crate's problems under its name; return how many there were.

  Callers accumulate the return value and exit non-zero on any total above zero.
  Printing here rather than at the call site keeps the per-crate heading adjacent
  to the lines it heads even when several gates run in one pass.

  **An entry beginning with whitespace is evidence for the entry above it, not a
  problem of its own, and is printed without being counted.** G15 attaches a
  quoted/actual diff beneath each stale recipe, and the first baseline run
  counted those lines: it reported 749 problems where 627 existed, the surplus
  being exactly the 122 lines of diff detail. A miscount in this direction is
  the harder one to notice — the number is alarming rather than reassuring, and
  every individual line printed under it is real.
  """
  n = sum( 1 for x in problems if not x[ :1 ].isspace() )
  if problems:
    print( f'  {crate}: {n} problem(s)' )
    for x in problems:
      print( f'    {x}' )
  return n
