#!/usr/bin/env python3
"""G21 — no recipe quotes a source file by line number.

A corpus recipe that says `sed -n '210,222p' ring/ring_core/src/lib.rs` is
correct exactly until somebody edits line 209. After that the recipe still runs,
still exits zero, and still prints Rust — just the wrong Rust. G15 notices
because the quoted block no longer matches, so nothing silently rots today; what
it costs instead is that **every source correction invalidates every recipe
addressing a line below it, across every crate.**

That cost was measured rather than assumed. One five-line change to
`ring_core/src/lib.rs` — carrying a validated `Capacity` in a variant so an
infallible accessor would stop re-running a fallible validation — left **61
recipes stale across 18 crates**, and not one of the 61 was a real disagreement
about the source. Every one was a line number that had moved.

S7 asks for 458 such corrections. At that coupling the stage is a treadmill: the
corrections are cheap and the repairs after them are not, and a corpus that
punishes correcting the defects it finds will be left uncorrected.

So the rule is a bright line: **a recipe addresses source by content, never by
line number.** `awk '/pub fn capacity/,/^  }$/'` survives every edit above it and
fails loudly rather than quietly when the thing it names is gone — which is the
behaviour a citation should have. Note which half of `awk` that is: the tool is
not the question, and `awk 'NR >= 619 && NR <= 627'` is banned beside
`sed -n '619,627p'` for being the same address wearing a different hat.

**This gate's fourth defect, and it is the first three repeated a fourth time.**
`grep -n` was exempted here on the ground that it "finds its line by content and
only the printed number moves, so it goes stale without ever becoming wrong."
Every clause of that is true and none of it is the rule. The harm this gate was
written for is stated eighteen lines above, in cost: *every source correction
invalidates every recipe addressing a line below it, across every crate* — and
the 61 measured there were, in that same sentence, "not one a real disagreement
about the source. Every one was a line number that had moved." That is precisely
what `grep -n` produces. The exemption answered a correctness question the rule
had never asked, and so carved out the exact failure the rule exists to price.

It was measured, on 2026-09-04: edits to `ring_align`, `ring_config` and
`ring_cursor` left **192 recipes stale across 26 crates**, of which **122
differed from their published output in nothing but a line number** — the
identical failure as the founding 61, at three times the size, entirely inside
the carve-out. So the number is what is banned, not the tool that prints it.

`grep -n` is flagged only when a number actually reaches the page: the recipe
requests numbering from a file *and* the `Live output` beneath it carries the
result. A `grep -n` whose number is consumed mid-pipeline and never published
moves nothing when the source moves, and is left alone. That gate is narrower
than the one on `sed`/`awk`, which are flagged whether or not output is quoted,
and deliberately so: a `sed -n '210,222p'` slides onto different code and keeps
exiting zero, so it is dangerous unpublished. A number nobody printed is not.

Usage: python3 addressing.py <docs>...
"""

import pathlib
import re
import shlex
import sys

import corpus_lib
import recipes


# `sed -n '210,222p' ring/ring_core/src/lib.rs` — an absolute line address
# into a named file. The quotes are part of the corpus' own idiom and are
# required here so a `sed -n "$range"p` built at runtime is not matched blind.
#
# **A target is required, and it must look like a file.** `... | sed -n '1,4p'`
# pages a pipeline — it is `head -4` spelled differently, addressing a stream
# this recipe produced a line earlier, not a source file whose numbering some
# future edit will shift. Seven of those exist in the family and flagging them
# was this gate's own first defect: the rule is about citing source by line, and
# a pager cites nothing. The separator is `[ \t]` rather than `\s` for the same
# reason — `\s` crosses a newline and swallows the next command's first word as
# though it were a filename.
#
# **This gate's third defect, and it is the second one repeated.** The first
# version of this pattern ended at `p'` — a closing quote immediately after the
# command letter — so it saw `sed -n '210,222p' f` and nothing else. A sed
# program holds any number of commands separated by `;`, and
# `sed -n '161,166p;177,182p;199,204p' f` has a semicolon where the pattern
# demanded a quote, so it matched none of them. The family carried **148 such
# addresses across 15 crates** while this gate reported REACHED for all 33 and
# the `awk` half below was doing every bit of the work. That is the identical
# failure the `awk` comment describes — a rule stated in prose and implemented
# as one syntax — occurring inside the very tool written to stop it. So the
# program is parsed now rather than pattern-matched.
SED_CALL = re.compile( r"sed -n +('[^']*'|\"[^\"]*\")[ \t]+(\S+)" )

# What an address is made of, once the parts that cannot hold a line number are
# folded away. A `/regex/` and a `$(( ... ))` each collapse to one opaque
# character, leaving only literal digits behind — and the placeholder is `%`/`&`
# rather than a letter because an address ends where its command letter begins,
# so a letter placeholder truncates it. That truncation was not hypothetical:
# `R` for regex turned `1,/^### FC13 /p` into the address `1,`, which reads as a
# bare literal and flagged a recipe that runs from the top of a file to a match.
REGEX_PART = re.compile( r'\\?/(?:\\.|[^/\\])*/' )
SHELL_PART = re.compile( r'\$\(\([^)]*\)\)|\$\([^)]*\)|\$\{[^}]*\}|\$\w+' )
COMMAND_SEP = re.compile( r'[;\n]' )
ADDRESS_HEAD = re.compile( r'^[^a-zA-Z]*' )
FROM_TOP = re.compile( r'1[ \t]*,[ \t]*%!?' )


def sed_addresses( program ):
  """Every address in one sed program that names a line by decimal literal.

  Three shapes are content-anchored and are not addresses in the sense this rule
  bans. `/pub fn capacity/,/^  }$/p` names content and moves with it.
  `${n},$(( n + 18 ))p` computes both bounds from a line found by matching a
  moment earlier, so its digits are an offset rather than an address.
  `1,/^### FC13 /p` starts at the top of the file — a position no edit below it
  can move — and stops at a match.
  """
  body = program[ 1 : -1 ]
  body = SHELL_PART.sub( '&', body )
  body = REGEX_PART.sub( '%', body )
  out = []
  for command in COMMAND_SEP.split( body ):
    address = ADDRESS_HEAD.match( command.strip() ).group( 0 ).rstrip()
    if not any( char.isdigit() for char in address ):
      continue
    if FROM_TOP.fullmatch( address ):
      continue
    out.append( address )
  return out

# `awk 'NR >= 619 && NR <= 627 { ... }' ring/ring_flush/src/lib.rs` — the same
# absolute address in the other spelling this corpus writes it in.
#
# **This is the gate's second defect, and it is the more instructive one.** The
# first version knew only `sed`, reported the family clean, and was believed. The
# 28 `awk NR` addresses it could not see surfaced a day later out of G15 instead:
# four recipes had slid onto different code and were still exiting zero. A gate
# that names its rule in prose — *address the content, never the line* — and then
# implements one syntax for it does not enforce the rule, it enforces the syntax,
# and the difference is invisible from a clean report.
#
# `NR` must be compared against a literal for the address to be absolute.
# `awk '/must_use/{ m = NR } m && NR > m && NR <= m + 3'` is content-anchored:
# every bound is relative to a line found by matching, so the whole window moves
# with the thing it names. That is the form the rule wants, not the form it bans.
# awk invocations are found by tokenising rather than by matching `awk '`
# directly, because the program is not always the first argument. A converted
# recipe reads `awk -v n1="$( ... )" 'NR >= n1 && NR <= 930' T`, and a pattern
# anchored on the quote right after `awk` skips the whole call — so the `930`
# still sitting in it became invisible to this gate at the moment it was
# converted. A checker made blind by the fix for what it checks is worse than no
# checker, since the clean report is what gets believed.
#
# **The digits must be a whole literal, not the tail of a name.** Converting the
# 148 `sed` addresses above produced the form this rule asks for —
# `/^pub fn set/{ n1 = NR } n1 && NR == n1 + 3 { print }`, every bound computed
# from a line found by matching — and this pattern flagged 69 of them. The
# unanchored `\d+` was matching the `1` inside the variable name `n1`, with
# `= NR` right after it, so the very shape the rule prescribes read as the shape
# it bans. A gate that rejects its own remedy pushes the corpus back toward what
# it was written to remove, which is worse than a gate that misses: a false
# REACHED is believed once, a false NOT REACHED is argued with every time.
AWK_OPT_WITH_VALUE = ( '-v', '-f', '-F' )
NR_LITERAL = re.compile( r'\bNR\b[ \t]*[<>!=]=?[ \t]*\d+\b|\b\d+[ \t]*[<>!=]=?[ \t]*\bNR\b' )

# `grep -n pat ring/ring_cursor/src/lib.rs` — the third spelling, and the one
# the family actually writes most. Options taking a separate value are stepped
# over so their argument is not mistaken for the pattern or a target; a cluster
# is only a numbering request if the `n` is in a single-dash cluster, since
# `--include=*.rs` carries an `n` that means nothing of the sort. That is not a
# hypothetical: a first census of this construct used `-[a-zA-Z]*n` and counted
# `--include` as a hit, inflating the family's exposure by three quarters.
GREP_OPT_WITH_VALUE = ( '-e', '-f', '-m', '-A', '-B', '-C', '-d', '--include',
                        '--exclude', '--exclude-dir', '--color' )
PIPE_TOKENS = ( '|', ';', '&&', '||', '>', '>>' )

# What counts as a file: a path, or a variable holding one. A bare word is a
# command that followed the pager, not its target. `*` and `?` belong in the
# segment class because `ring_*/src/*.rs` is how this corpus greps a family —
# without them the commonest multi-crate recipe in the family reads as having
# no target at all.
FILE_TARGET = re.compile( r'^(\$|["\']?[.~/]|[\w.*?-]+/)' )


def program_args( toks ):
  """Indices of the `sed`/`awk` program arguments — text, never paths.

  `s/:  */:/` opens with a word and a slash, which is exactly what a relative
  path looks like, so a scan for path-shaped tokens picks sed programs up and
  reports them as the file a recipe cites. The distinction is positional, not
  textual: the first non-flag token after `sed` or `awk` is its program. It
  cannot be recovered by making the path pattern stricter, because there is no
  spelling that separates `s/a/b/` from a real two-segment relative path.
  """
  out = set()
  for ( i, tok ) in enumerate( toks ):
    if tok not in ( 'sed', 'awk' ):
      continue
    j = i + 1
    while j < len( toks ) and toks[ j ].startswith( '-' ) and toks[ j ] != '-':
      j += 2 if toks[ j ] in ( '-e', '-f', '-v' ) else 1
    if j < len( toks ):
      out.add( j )
  return out

# A published number: `241:`, `241-`, or `module/x.rs:241:`. Both separators
# count, because `grep -n` uses `:` for a match and `-` for a context line.
#
# **The fourth occurrence of this gate's founding defect, and this one was
# mine.** The separator was `:` alone, on the stated ground that "`grep -n -A6`
# marks context lines with `-` rather than `:`, but the matched line always
# carries the colon, so requiring it costs no detection." The premise is true
# and the conclusion does not follow: the matched line carries the colon only
# while it is still in the output. `ring_bench/docs/item/001` runs
# `grep -nB1 '^pub enum RunError' … | grep derive`, whose second stage keeps the
# `-B1` context and discards the match — so every line reaching the page is
# `638-#[ derive( … ) ]`, a volatile number with a hyphen after it, and the rule
# waved all five through. A rule about the number, implemented as one of the two
# syntaxes it appears in. That is the same mistake as the `grep -n` exemption
# eighteen lines above, and the `sed`-semicolon and pager cases before it.
#
# The lookahead is what the colon was doing usefully: `2026-09-04` at the head
# of a line is a date, not line 2026, and only the date has two more
# hyphen-separated pairs behind it.
#
# `[ \t]*` is the same mistake caught an eighth time, and the cheapest instance
# of it yet: a rule about *the number*, implemented as *the number at column
# zero*. Recipes routinely close with `sed 's|^|    |'` to set their output off
# from the surrounding prose, and that purely cosmetic indent moved every
# address they publish out of the anchor's reach. Twenty-six recipes were
# invisible for no reason but four spaces — `grep_calls` had already found the
# `-n` and the file it read, and only this test stood between them and a
# report. Each of the twenty-six was then checked the hard way, by taking the
# published number and the text beside it and asking the cited file whether that
# text is genuinely on that line: twenty confirmed outright, and the six the
# check could not resolve were truncated by a `cut -c` or rewrapped by a `fold`
# in the recipe itself, not absent.
NUMBERED_LINE = re.compile( r'^[ \t]*(?:[^\s:]+:)?\d+[:-](?!\d{2}-\d{2})', re.M )

# `CursorPair::capacity (line 324) ... ok` — rustdoc names every doctest by the
# line its fence opens on, so a recipe that runs `cargo test --doc` and quotes
# the result publishes one absolute source citation per doctest, and they all
# shift together the moment anyone inserts a line above them.
#
# **This is what the rule is actually about, arriving without a tool.** The
# three detectors above each find a *command that addresses by line* — `sed -n`,
# `awk NR`, `grep -n`. Not one of them is present here: the recipe asks for a
# test run and prints what came back. Written as "find the addressing command",
# the rule had no way to see it; written as "find the published line address",
# it is the same violation in a different coat.
#
# That framing error is this gate's founding defect, and this is the sixth time
# it has surfaced — after the pager case, the `sed`-semicolon case, the `grep -n`
# exemption, and `NUMBERED_LINE`'s own colon-only separator. Every one of them
# was a rule about *the number* implemented as *one syntax the number is written
# in*. `ring_cursor/docs/decisions/002` publishes fourteen of these at once,
# under prose whose claim is "fourteen doctests, one per public item" — a
# sentence about the count, evidenced by a block that is mostly line numbers.
#
# The recipe's own author already knew: the same pipeline carries
# `sed -E 's/; finished in .*/; finished/'` to strip the wall-clock time. The
# volatility was recognised and stripped, once, in the place it was noticed.
DOCTEST_LINE = re.compile( r'\(line \d+\)' )
# `test ring/ring_mpsc/src/lib.rs - (line 121) - compile fail ... ok`. Whether
# the path survives to the page depends on the recipe: ring_cursor's `sed` cuts
# it off, so `-p <crate>` is the fallback, and a recipe naming neither is not
# reported rather than reported against a guess.
DOCTEST_PATH = re.compile( r'^test[ \t]+(\S+)[ \t]+-[ \t]', re.M )
CARGO_PACKAGE = re.compile( r'(?:^|\s)-p[ \t]+([\w-]+)' )


def docs_files( docs ):
  """Every markdown file under a crate's `docs/` — instances and readmes both.

  Definition readmes carry regenerate blocks too, and the Module Index carries
  the crate's largest one. Checking only instances would leave the biggest
  recipes in the tree ungraded, which is how `recipes.py` came to never scan
  `docs/readme.md`.

  Fix(docs_files_reported_a_missing_docs_tree_as_clean): `rglob` on a directory
  that does not exist returns an empty iterator rather than raising, so a stale
  path reached `check()` as zero files, `main()` printed `0 problem(s)`, and the
  gate exited 0. Every sibling checker reaches `iterdir` instead — via
  `corpus_lib.definitions` or `recipes`' own walk — and raises `FileNotFoundError`
  there, which `common.sh`'s `checker_crashed()` already catches as an abort
  rather than a verdict. This was the only one of the six that answered a path it
  could not read with a pass. Root cause: `rglob`'s tolerance of a missing root,
  which reads as convenience and behaves as a vacuity hole. Pitfall: the live gate
  cannot currently reach it — `corpus_docs()` tests `[ -d "$d/docs" ]` per crate
  and fails loudly on any miss — so this guards direct invocation and any future
  caller that skips that check. The tree has been relocated four times; a stale
  path list is the ordinary case here, not the exotic one.
  """
  root = pathlib.Path( docs )
  if not root.is_dir():
    raise FileNotFoundError( f'no such docs directory: {docs}' )
  return sorted( root.rglob( '*.md' ) )


def awk_calls( text, depth = 0 ):
  """( program, target ) for every awk invocation in a recipe.

  Tokenised with `shlex` so that options preceding the program are stepped over
  rather than tripped on. A token that itself contains `awk` is a command
  substitution — `"$( ... awk ... )"` collapses to one token — and is descended
  into, bounded so a pathological nesting cannot spin.
  """
  try:
    toks = shlex.split( text, comments = True )
  except ValueError:
    return []
  out = []
  i = 0
  while i < len( toks ):
    tok = toks[ i ]
    if tok == 'awk':
      j = i + 1
      while j < len( toks ) and toks[ j ].startswith( '-' ) and toks[ j ] != '-':
        j += 2 if toks[ j ] in AWK_OPT_WITH_VALUE else 1
      if j < len( toks ):
        out.append( ( toks[ j ], toks[ j + 1 ] if j + 1 < len( toks ) else '' ) )
      i = j
    elif 'awk' in tok and depth < 3:
      out.extend( awk_calls( tok, depth + 1 ) )
    i += 1
  return out


def grep_calls( text ):
  """( flag, file ) for every grep invocation in a recipe that numbers lines.

  The file reported is the numbering grep's own target where it has one, and
  otherwise the recipe's first file argument — because `grep -vE x f | grep -nE y`
  numbers the *filtered stream*, and a filtered stream's numbering is no more
  stable than the file's. Its counter moves when any earlier line changes,
  including a comment line the first grep discards, so the published number
  shifts for edits that are not even visible in the output. Requiring the `-n`
  to sit on the grep holding the filename would wave 24 such recipes through —
  the same shape of hole as the `grep -n` exemption this rule replaced, and
  found the same way: by G15 failing on what this gate had already passed.
  """
  try:
    toks = shlex.split( text, comments = True )
  except ValueError:
    return []
  # Every file the recipe reads, for the fallback below. The argument of a `cd`
  # is not one of them: every recipe in this corpus opens by cd-ing to the
  # repository root, so taking it would name that path as the cited file in
  # every single message.
  programs = program_args( toks )
  files = [ t for ( k, t ) in enumerate( toks )
            if not t.startswith( '-' ) and t not in PIPE_TOKENS
            and FILE_TARGET.match( t ) and k not in programs
            and ( k == 0 or toks[ k - 1 ] != 'cd' ) ]
  out = []
  i = 0
  while i < len( toks ):
    if toks[ i ] != 'grep':
      i += 1
      continue
    j = i + 1
    numbering = None
    while j < len( toks ) and toks[ j ].startswith( '-' ) and toks[ j ] != '-':
      tok = toks[ j ]
      if not tok.startswith( '--' ) and 'n' in tok[ 1 : ]:
        numbering = tok
      j += 2 if tok in GREP_OPT_WITH_VALUE else 1
    j += 1                                        # the pattern
    # Stop at the next `grep` as well as at a pipe. `shlex` flattens newlines,
    # so a recipe running two greps on two lines has no separator between them
    # at all — scanning only to the next pipe swallows the second call whole.
    # That cost 28 detections when this loop was first written without it, all
    # of them in the corpus' most ordinary shape: one grep per line.
    own = None
    while j < len( toks ) and toks[ j ] not in PIPE_TOKENS and toks[ j ] != 'grep':
      if own is None and not toks[ j ].startswith( '-' ) and FILE_TARGET.match( toks[ j ] ):
        own = toks[ j ]
      j += 1
    if numbering and ( own or files ):
      out.append( ( numbering, own or files[ 0 ] ) )
    i = j
  return out


# `module/` was this family's only crate root until the ring crates moved to
# `ring/`, and a doctest citation names its package, never its path — so the
# root has to be found rather than assumed. Assumed, the gate still fires (a
# `<word>/...` target matches `FILE_TARGET` whichever root it names) but prints
# a file that no longer exists — a stale citation, minted by the gate written
# to grade them.
#
# The four roots and the depth walk below are `common.sh`'s `crate_dir()`
# written in Python, deliberately: one resolver disagreeing with the other about
# where a crate lives is how a gate grades a tree nobody occupies.
CRATE_ROOTS = ( 'module', 'ring', 'substrate', 'spike' )
REPO_ROOT = pathlib.Path( __file__ ).resolve().parents[ 4 ]


def _group_dirs( parent ):
  """Real, sorted subdirectories of `parent` — scratch and build output excluded.

  Sorted because resolution order is a gate's output: two same-named crates under
  different groups would otherwise resolve by filesystem order, the defect
  `ws_verify`'s `test_sources` already had to fix once. `-*` is skipped per
  `rulebook.md`'s storage-tier convention (gitignored scratch at any depth), and
  `target` because a build directory reaches a real crate's nesting depth by
  accident rather than by being one.
  """
  if not parent.is_dir():
    return []
  return sorted(
    d for d in parent.iterdir()
    if d.is_dir() and d.name != 'target' and not d.name.startswith( '-' )
  )


def crate_src_lib( package ):
  """`<root>/…/<package>/src/lib.rs` for whichever crate root actually holds it.

  Searched flat (`ring/<package>`), one group deeper (`substrate/<family>/<package>`)
  and two (`module/division/<NNN_name>/<package>`) — every depth a crate is reachable
  at in this tree, matching `common.sh`'s `crate_dir()` and `g10_pinned_math.sh`'s
  own scan globs.

  Fix(addressing_crate_roots_missed_substrate_and_spike): the roots were
  `( 'module', 'ring' )` and the nested walk looked under `<root>/substrate/` and
  `<root>/division/`. Root cause: the substrate tree was promoted from
  `module/substrate/<NNN_name>/` to a repository root of its own, losing exactly one
  level, so `module/substrate/` stopped existing and every one of its crates — plus
  all of `spike/`, never a root here at all — resolved to None. Pitfall: None is not
  a loud failure here. `addresses()` drops the doctest finding on `if target:`, so an
  unresolvable package reports clean rather than reporting an error, and the gate
  stays green over the crates it can no longer see. `g10_pinned_math.sh` carries the
  same relocation's fix and its own comment records 742 sources going unread; this
  resolver was the copy that did not get it.

  Finding(crate_src_lib_matches_directory_name_not_package_name): `package` is
  matched against the *directory* name, so a crate whose manifest renames it away
  from its directory resolves to None, the same silent shape the Fix above closed.
  Left as-is rather than fixed: `common.sh`'s `crate_dir()` matches on the
  directory name too, and one resolver learning package names while the other
  does not is the disagreement the comment above this function exists to
  prevent. `addressing_test.py` pins the divergence set — empty for every crate
  currently in this repo — so it fails rather than silently widens if a future
  crate is added under a renamed manifest.
  Fix(crate_src_lib_resolved_the_empty_package_name): an empty `package` returned
  the first *group* directory that happens to hold a `src/lib.rs` of its own —
  `module/frame_spine//src/lib.rs`, note the doubled slash. Root cause: `pathlib`
  collapses `dir / ''` back to `dir`, so the membership test asked whether the
  group itself is a crate, while the returned string was still built by an f-string
  that spliced the empty name back in. Pitfall: unreachable from `addresses()` today
  — `CARGO_PACKAGE`'s `[\\w-]+` cannot match an empty name — so this is a guard on
  the function's own contract, not a live path. It is still worth holding: the
  failure is a fabricated citation printed into a G21 problem line, which is the
  exact defect this gate exists to grade.
  """
  if not package:
    return None
  for root in CRATE_ROOTS:
    root_dir = REPO_ROOT / root
    if ( root_dir / package / 'src' / 'lib.rs' ).is_file():
      return f'{root}/{package}/src/lib.rs'
    for group in _group_dirs( root_dir ):
      if ( group / package / 'src' / 'lib.rs' ).is_file():
        return f'{root}/{group.name}/{package}/src/lib.rs'
      for workstream in _group_dirs( group ):
        if ( workstream / package / 'src' / 'lib.rs' ).is_file():
          return f'{root}/{group.name}/{workstream.name}/{package}/src/lib.rs'
  return None


def addresses( recipe, quoted ):
  """Every absolute line address in one recipe, as ( shown, target ) pairs.

  All four spellings are collected here rather than in `check` so that a fifth
  one — and there were a second, a third and a fourth, each found only because
  G15 caught what this gate had already passed — is added in one place and
  cannot be added to the `sed` path while the others silently keep their own
  idea of the rule.

  The first three ask *which command addressed by line*; the fourth asks *what
  line address reached the page*. The second question is the one the rule was
  always about, and it is the only one that catches a citation no command in the
  recipe produced — see `DOCTEST_LINE`.

  **`recipe` is None for a probe recording, and that was this gate's fifth
  defect.** `recipes.blocks()` yields two shapes and says so in its own
  docstring: a `sh`-tagged fence, whose body is the command, and a probe
  recording, whose command was deleted on purpose because re-running it is not
  the point — `command` is None there. This function was written against the
  one-shape version, so the first `findall` raised `TypeError` on the first
  None it met. It met one on crate 13 of 34 and the gate still reported
  `across 33 crate(s)`; see `common.sh`'s `checker_crashed` for why that claim
  outlived the abort that made it false.

  The guard wraps the first three scans rather than the whole function, on
  purpose. The fourth question is answerable from `quoted` alone, and a probe
  recording is nothing but quoted output — so it is precisely the shape most
  able to publish a bare line address with no surviving command to blame it on.
  Skipping those blocks wholesale would have narrowed the rule silently, which
  is the failure the four comments above this one already record.
  """
  out = []
  if recipe is not None:
    for ( program, target ) in SED_CALL.findall( recipe ):
      if sed_addresses( program ):
        out.append( ( f'sed -n {program} {target}', target ) )
    for ( program, target ) in awk_calls( recipe ):
      m = NR_LITERAL.search( program )
      if m and target:
        out.append( ( f"awk '…{m.group( 0 )}…' {target}", target ) )
    if quoted and NUMBERED_LINE.search( quoted ):
      for ( flag, target ) in grep_calls( recipe ):
        out.append( ( f'grep {flag} … {target}', target ) )
  if quoted:
    cited = DOCTEST_LINE.findall( quoted )
    if cited:
      m = DOCTEST_PATH.search( quoted )
      if m:
        target = m.group( 1 )
      else:
        m = CARGO_PACKAGE.search( recipe ) if recipe is not None else None
        target = crate_src_lib( m.group( 1 ) ) if m else None
      if target:
        many = f' ×{len( cited )}' if len( cited ) > 1 else ''
        out.append( ( f'doctest {cited[ 0 ]}{many} of {target}', target ) )
  return out


def check( docs ):
  """Every line-addressed extraction in one crate. Returns a list of problems."""
  root = pathlib.Path( docs )
  problems = []
  for path in docs_files( docs ):
    # `recipes.blocks` rather than a regex over the text: fence parsing here has
    # to be line-anchored, because this corpus publishes recipes that grep for
    # fence markers and a non-anchored `.*?` closes the block on a pattern
    # inside it. Reusing G15's parser also means both gates agree on what a
    # recipe is and on which output belongs to it, which is the whole basis for
    # the `grep -n` rule below.
    for ( _, recipe, quoted ) in recipes.blocks( path ):
      for ( shown, target ) in addresses( recipe, quoted ):
        if not FILE_TARGET.match( target ):
          continue
        rel = path.relative_to( root )
        problems.append(
          f'LINEADDR {rel}: `{shown}` — address the content, not the line'
        )
  return problems


def main():
  if len( sys.argv ) < 2:
    print( 'usage: addressing.py <docs>...' )
    return 2
  total = 0
  for docs in sys.argv[ 1 : ]:
    total += corpus_lib.report( corpus_lib.crate_name( docs ), check( docs ) )
  print( f'G21 addressing: {total} problem(s)' )
  return 1 if total else 0


if __name__ == '__main__':
  sys.exit( main() )
