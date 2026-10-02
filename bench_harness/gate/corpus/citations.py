"""G16 — every citation resolves, and the two spellings of each finding agree.

Takes one or more `docs/` directories. Three assertions, each over a different
kind of reference:

  * **Links.** Every relative markdown link resolves to a file on disk. A link
    into a sibling definition needs `../<def>/file.md`; a link into another
    crate needs `../../../`. Both are one path segment away from being wrong and
    neither fails anything at read time.

  * **Cited tests.** Every name in a `### Tests` table exists in the crate's own
    `tests/`. A findings document that names the test covering it is making a
    checkable claim; a renamed test turns it into a false one silently.

  * **Agreement.** Each Module Index findings row states a Subject and a Tier
    that the owning definition readme also states, in a different column order.
    They must be equal character for character. Two independently maintained
    spellings of one fact is the corpus' most reliable source of drift, and the
    column-order difference is exactly what stops a copy-paste from keeping them
    aligned by accident.
"""

import pathlib
import re
import sys

import corpus_lib as lib


LINK = re.compile( r'\[[^\]]*\]\(([^)#][^)#]*?)(?:#[^)]*)?\)' )
TEST_ROW = re.compile( r'^\| `?([a-z_][a-z0-9_]*)`? \|' )


def check( docs ):
  problems = []
  docs = pathlib.Path( docs )
  crate = docs.parent

  # --- links ----------------------------------------------------------------
  for p in sorted( docs.rglob( '*.md' ) ):
    rel = p.relative_to( docs ).as_posix()
    for ( i, line ) in lib.prose_lines( p ):
      for target in LINK.findall( line ):
        if target.startswith( ( 'http://', 'https://', 'mailto:' ) ):
          continue
        if not ( p.parent / target ).exists():
          problems.append( f'LINK   {rel}:{i}  {target}' )

  # --- cited tests ----------------------------------------------------------
  names = set()
  tests = crate / 'tests'
  if tests.is_dir():
    for t in tests.rglob( '*.rs' ):
      names |= set( re.findall( r'^\s*fn ([a-z_][a-z0-9_]*)', t.read_text(), re.M ) )

  # Fix(a_deeper_heading_is_not_a_boundary): mirrors corpus_lib.py's own
  # _section_rows fix. The close check used to be a bare
  # `line.startswith('###')`, so a deeper `####` heading nested under
  # `### Tests` satisfied it and wrongly ended the section instead of nesting
  # inside it. Root cause: the same one line over. A prefix compares character
  # runs, not heading depth. Pitfall: zero live trigger today (no `#### `
  # heading exists anywhere under docs/, confirmed by a repo-wide grep), since
  # every `### Tests` table in this corpus currently ends before any nested
  # heading. Dormant until an author nests one under a Tests table.
  #
  # Fix(citations_tests_open_is_a_prefix_not_a_heading): the open check
  # mirrored the fix above for the close boundary, but not the sibling fix
  # `_section_rows` also carries for the *open* boundary,
  # `a_longer_heading_is_not_the_same_heading`. `line.startswith( '### Tests' )`
  # would treat a differently-named heading like `### Tests Deferred` or
  # `### Testsuite Coverage` as opening this same section. Same bug class, one
  # line earlier. Root cause: identical to the fix above. A prefix compares
  # character runs, not "is this heading the one named heading," which is what
  # an open boundary means. Pitfall: zero live trigger today, measured
  # directly, not assumed. Every prose line in every doc instance across all 33
  # crates (158,104 lines, 923 files) for which `startswith( '### Tests' )` is
  # true is also exactly equal to `'### Tests'`, so no corpus content
  # distinguishes the two checks yet.
  for ( _, _, p ) in lib.instances( docs ):
    rel = p.relative_to( docs ).as_posix()
    inside = False
    for ( i, line ) in lib.prose_lines( p ):
      if line == '### Tests':
        inside = True
        continue
      if inside:
        hm = lib.HEADING.match( line )
        if hm and len( hm.group( 1 ) ) <= 3:
          inside = False
      if not inside:
        continue
      m = TEST_ROW.match( line )
      if not m:
        continue
      name = m.group( 1 )
      if name in ( 'test', 'Test' ):
        continue
      if name not in names:
        problems.append( f'TEST   {rel}:{i}  cites `{name}`, which no test in tests/ declares' )

  # --- agreement between the two findings tables ----------------------------
  declared = {}
  for d in sorted( x for x in docs.iterdir() if x.is_dir() and x.name != 'definition' ):
    for ( ln, fid, subj, tier ) in lib.declared_findings( d / 'readme.md' ):
      declared[ fid ] = ( f'{d.name}/readme.md', ln, subj, tier )

  for ( ln, fid, _, subj, tier, where ) in lib.master_findings( docs ):
    if fid not in declared:
      continue
    ( owner, oln, osubj, otier ) = declared[ fid ]
    if subj != osubj:
      problems.append(
        f'SUBJ   {fid}  index:{ln} "{subj}"  vs  {owner}:{oln} "{osubj}"'
      )
    if tier != otier:
      problems.append(
        f'TIER   {fid}  index:{ln} "{tier}"  vs  {owner}:{oln} "{otier}"'
      )
    if not where.startswith( owner.split( '/' )[ 0 ] + '/' ):
      problems.append( f'WHERE  {fid}  index:{ln} points at {where}, row lives in {owner}' )

  return problems


def main( argv ):
  bad = 0
  for docs in argv[ 1 : ]:
    bad += lib.report( lib.crate_name( docs ), check( docs ) )
  print( f'{bad} problem(s)' )
  return 1 if bad else 0


if __name__ == '__main__':
  sys.exit( main( sys.argv ) )
