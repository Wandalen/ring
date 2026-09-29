"""Static dead-anchor detection for multi-clause match recipes.

A recipe built from several independently-anchored match clauses — one awk
program running more than one `/pattern/{ ... }` rule against the same file,
most commonly — can go PARTIALLY dead: one clause's anchor pattern is edited
out of the target it reads, that clause stops contributing anything, and the
remaining clauses keep emitting output. The block still runs, still exits 0,
and prints exactly as before to anyone who does not already know which lines
came from which clause.

Nothing else in this corpus catches that. G15 (`recipes.py`) diffs a block's
own output against a quoted `Live output:` fence, but an unquoted block — the
common case for a definition readme's `### Regenerate` section, per house
style — has no fence to diff against, so G15 is structurally exempt from ever
seeing this shape (`_check_readme` holds an unquoted block to exit-0 only).
A separate vacuous-label scan that checks `echo '-- label --'` announcements
against a recorded fence cannot see it either: a dead awk clause announces
nothing, so there is no label mismatch to find, and line count is not a
signal — the block prints a plausible number of lines whether one clause of
three is dead or all three are alive.

This is a *static* check for exactly that gap: for every independently
anchored clause a recipe's command contains, assert the clause's own anchor
pattern still occurs somewhere in the target file it names — no execution, no
recorded output needed, so it sees an unquoted block exactly as well as a
quoted one.

Recognizes one clause shape today: an awk program's repeated
`/pattern/{ action }` rules, each carrying its own anchor — the shape this
check was built to catch, and the only one exercised by a test (see
`anchors_test.py`, built from `ring_slot`'s own git history). Two sibling
shapes named alongside it — a command's several `-e 'pattern'` flags, and a
single pattern's top-level `|` alternation treated as independent branches —
have no specimen in this corpus yet to build a real test from, so they are
left unhandled rather than guessed at: extend `_AWK_CLAUSE`/`awk_clauses`
once one exists.
"""

import pathlib
import re
import sys

import corpus_lib as lib
import recipes


# One awk invocation within a (possibly multi-command) recipe body: the
# program in single quotes, then the target file as the word immediately
# following. Shell single-quotes admit no escaping of their own, so the
# program body is exactly "everything up to the next quote character" — every
# awk recipe in this corpus follows that convention (no embedded `'`). No `$`
# anchor after the target: `ring_handle/docs/lifecycle/002_...md` pipes the
# same invocation's output to `head -4`, and requiring the target to run to
# end of line missed the invocation entirely rather than merely mis-sizing it.
_AWK_INVOCATION = re.compile( r"^[ \t]*awk\s+'([^']*)'\s*(\S+)", re.MULTILINE )

# A shell line continuation — backslash immediately before a newline — joined
# into the space it stands in for, matching how bash itself reads it before
# `_AWK_INVOCATION` ever sees the text. Without this, `ring_handle`'s awk
# invocation split across two physical lines was read only as far as the
# trailing backslash, which `(\S+)` then captured as the "target" in place of
# the file on the next line.
_LINE_CONTINUATION = re.compile( r'\\\n[ \t]*' )

# One awk pattern-action clause inside a program: `/regex/{`. The trailing `{`
# is what marks an anchored rule rather than a bare NR-range condition that
# reuses an earlier clause's anchor and carries none of its own — e.g.
# `n1 && NR <= n1 + 7 { print }`, which this pattern correctly does not match.
# The regex body is `(?:\\.|[^/\\])*` rather than `[^/]*` so an escaped slash
# (`\/`, how this corpus spells a literal `/` inside an awk-quoted pattern —
# `///` for a Rust doc-comment marker is the recurring case) does not end the
# clause early.
_AWK_CLAUSE = re.compile( r'/((?:\\.|[^/\\])*)/[ \t]*\{' )

# The recipe's own declared root, per this repo's `rulebook.md` § Documentation
# : Executable Recipes ("state where it runs from"): an absolute `cd` as the
# first line of nearly every block here. Read the *last* one in the command
# text, matching shell execution order, for the rare body that changes
# directory twice.
#
# Finding(cd_line_never_matches_the_corpus_own_git_rev_parse_idiom): this
# pattern requires the cd argument to start with a literal `/`, so it can
# only ever match a hardcoded absolute path — never `cd "$(git rev-parse
# --show-toplevel)"`, the actual idiom "an absolute cd as the first line of
# nearly every block" turns out to mean in practice: the argument starts
# with `"`, not `/`. Measured directly, not assumed: every one of the 182
# awk-clause-bearing commands across the whole corpus (ring/, module/,
# substrate/) uses exactly that idiom, and _CD_LINE matches precisely zero
# of them — not "rare," never. `_command_root` therefore always falls
# through to its caller's `default_root` for every real recipe today, and
# the "read the *last* one... for the rare body that changes directory
# twice" mechanism this comment describes has not fired once in the corpus
# that exists. No live trigger for `check()` specifically: its own
# `default_root` is `_repo_root(docs)`, a `.git`-ancestor walk that resolves
# to the same repository root `$(git rev-parse --show-toplevel)` would have
# produced at runtime, so the fallback is not merely safe here, it is
# independently correct. The gap that stays real is the one
# `dead_clauses`'s own docstring already names for a *different* caller: a
# fixture lifted from git history whose command text carries a genuine
# stale literal absolute path (not the git-rev-parse form) would still be
# preferred by this function over that caller's own fixture root, exactly
# as documented there — `anchors_test.py` does not exercise that path
# either way, since its fixture strips the `cd` line rather than supplying
# one for `_command_root` to choose between.
_CD_LINE = re.compile( r'^[ \t]*cd[ \t]+(/\S+)[ \t]*$', re.MULTILINE )


def awk_clauses( command ):
  """Yield ( pattern, target ) for every anchored clause of every awk
  invocation in `command`.

  Pure text extraction — no filesystem access, so this half is testable with
  nothing more than the command string itself.
  """
  command = _LINE_CONTINUATION.sub( ' ', command )
  out = []
  for m in _AWK_INVOCATION.finditer( command ):
    ( program, target ) = ( m.group( 1 ), m.group( 2 ) )
    for cm in _AWK_CLAUSE.finditer( program ):
      out.append( ( cm.group( 1 ), target ) )
  return out


def _command_root( command, default_root ):
  """The directory a relative target path in `command` resolves against."""
  command = _LINE_CONTINUATION.sub( ' ', command )
  matches = _CD_LINE.findall( command )
  return pathlib.Path( matches[ -1 ] ) if matches else default_root


def _static_target( target ):
  """The literal path `target` names, or None when it is a shell template this
  static check has no value to resolve without actually running the shell —
  e.g. `"$d/Cargo.toml"` inside `ring_core`'s `for d in ring/ring_*/; do ...`
  loop, one recipe checking every crate's `Cargo.toml` in turn rather than one
  fixed file, or `ring_debug`'s `task/unverified/*.md` glob, which the shell
  (not this check) expands against whatever task files currently exist.
  Flagging either as a dead anchor would be wrong in the opposite direction
  from the class this module exists to catch: not a live block misread as
  fine, but a fine block misread as broken.
  """
  if len( target ) >= 2 and target[ 0 ] == '"' and target[ -1 ] == '"':
    target = target[ 1 : -1 ]
  return None if any( c in target for c in '$`*?[' ) else target


def dead_clauses( command, default_root ):
  """Every clause in `command` whose anchor no longer occurs in its target.

  Returns [ ( pattern, target, reason ) ]. `default_root` resolves a relative
  target when `command` carries no `cd` of its own (see `_command_root`) — a
  caller checking a fixture lifted from history passes the fixture's own root
  directly, rather than relying on a historical `cd` line that would resolve
  against this machine's *current* tree instead of the fixture.
  """
  root = _command_root( command, default_root )
  problems = []
  cache = {}
  for ( pattern, raw_target ) in awk_clauses( command ):
    target = _static_target( raw_target )
    if target is None:
      continue
    if target not in cache:
      path = pathlib.Path( root ) / target
      cache[ target ] = path.read_text() if path.exists() else None
    text = cache[ target ]
    if text is None:
      problems.append( ( pattern, target, 'target file not found' ) )
      continue
    try:
      live = re.search( pattern, text, re.MULTILINE ) is not None
    except re.error as e:
      problems.append( ( pattern, target, f'anchor does not compile as a regex ({e})' ) )
      continue
    if not live:
      problems.append( ( pattern, target, 'anchor no longer found in target' ) )
  return problems


def _repo_root( start ):
  """Nearest ancestor of `start` (inclusive) carrying `.git` — this repo's root.

  Fallback for a recipe that omits the `cd` line `_command_root` otherwise
  reads; every recipe seen in this corpus carries one, so this path is
  defensive rather than load-bearing.
  """
  p = pathlib.Path( start ).resolve()
  for candidate in ( p, *p.parents ):
    if ( candidate / '.git' ).is_dir():
      return candidate
  return None


def _readme_commands( readme, rel ):
  """Yield ( rel, lineno, command ) for every runnable recipe one readme carries."""
  if not readme.exists():
    return
  for ( ln, cmd, _quoted ) in recipes.blocks( readme ):
    if cmd is not None:
      yield ( rel, ln, cmd )


def _commands( docs ):
  """Yield ( rel, lineno, command ) for every recipe in scope.

  Same scope `recipes.check` grades — instances, every definition readme
  (`item/`'s kind subdirectories included), and the crate-level readme —
  reused via `recipes.blocks` rather than re-parsed, so the two checkers can
  never disagree about where a fence starts and ends.
  """
  docs = pathlib.Path( docs )

  for ( _, _, p ) in lib.instances( docs ):
    rel = p.relative_to( docs ).as_posix()
    for ( ln, cmd, _quoted ) in recipes.blocks( p ):
      if cmd is not None:
        yield ( rel, ln, cmd )

  for d in sorted( x for x in docs.iterdir() if x.is_dir() ):
    yield from _readme_commands( d / 'readme.md', d.name )
    if d.name == 'item':
      for kind in sorted( x for x in d.iterdir() if x.is_dir() ):
        yield from _readme_commands( kind / 'readme.md', f'{d.name}/{kind.name}' )

  yield from _readme_commands( docs / 'readme.md', '.' )


def check( docs ):
  docs = pathlib.Path( docs )
  root = _repo_root( docs )
  problems = []
  for ( rel, ln, cmd ) in _commands( docs ):
    for ( pattern, target, reason ) in dead_clauses( cmd, root ):
      problems.append( f'DEADANCHOR {rel}:{ln}  {reason} — {target}: {pattern}' )
  return problems


def main( argv ):
  bad = 0
  for docs in argv[ 1 : ]:
    bad += lib.report( lib.crate_name( docs ), check( docs ) )
  print( f'{bad} problem(s)' )
  return 1 if bad else 0


if __name__ == '__main__':
  sys.exit( main( sys.argv ) )
