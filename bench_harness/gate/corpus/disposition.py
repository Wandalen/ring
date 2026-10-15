#!/usr/bin/env python3
"""G20 — every finding on a reachable tier is acted on or declined, with a reason.

The other four corpus gates grade the corpus as a document: that it has the
right shape, that its recipes still reproduce, that its citations resolve, that
its tier strings are declared. None of them asks the question this one asks —
whether anything was *done* about what the corpus found. A family can be
perfectly consistent, fully cross-referenced and entirely reproducible while
every hazard it names is still there.

A finding on a reachable tier is a claim that something can still be acted on.
This gate requires each one to carry exactly one line, inside its own section:

    **Disposition:** applied — <what changed>. Now prints: `<literal>`
    **Disposition:** declined — <reason naming something concrete>

**The `Now prints:` clause is the whole design.** A disposition column that
takes the author's word is a Proxy Trap: it measures that someone typed
`applied`, never that a correction landed. So `applied` must quote a literal
string out of a `Live output:` block *in the same file* — and G15 independently
holds every such block equal to what its recipe prints today. The recipe reads
the source. So the chain is: source -> recipe -> quoted block -> disposition,
with G15 holding the middle link and this gate holding the last one. A
disposition claiming a correction the source does not carry fails here or there.

`declined` cannot be bound that way — a decision not to act leaves no trace to
measure — so the bar it carries instead is specificity: the reason must name
something, in backticks or as a path. "Not worth it" names nothing and is
exactly the shape a decline takes when it is a shrug rather than a ruling.

Usage: python3 disposition.py <corpus_standard.txt> <docs>...
"""

import pathlib
import re
import sys

import corpus_lib


# The disposition line, as it appears inside a finding's own section.
DISPOSITION = re.compile( r'^\*\*Disposition:\*\* (applied|declined) — (.+)$' )

# The evidence clause an `applied` disposition must end with. The literal is
# backticked so that a token carrying spaces or punctuation stays one token.
NOW_PRINTS = re.compile( r'Now prints: `([^`]+)`\s*$' )

# A `Live output:` block, quoted verbatim beneath the recipe that produced it.
LIVE_OUTPUT = re.compile( r'^Live output:\n\n^```\n(.*?)^```\n', re.S | re.M )

# Something concrete a declined reason can name: a backticked token, or a path
# with a directory separator in it.
CONCRETE = re.compile( r'`[^`]+`|\b[\w.]+/[\w./*-]+' )

# How much reason a decline has to carry before it is a ruling rather than a
# shrug. Short enough that a genuinely one-line reason fits; long enough that
# "no" and "later" do not.
MIN_REASON = 40


def reachable_tiers( standard ):
  """The bolded tiers from the declared set — the ones that can still be acted on.

  Bolding is the corpus' own distinction and is load-bearing in its findings
  tables, so it is read here rather than restated as a second list that could
  drift from the standard the other gates share.
  """
  return { t for t in standard.get( 'tier', [] ) if t.startswith( '**' ) }


def sections( path ):
  """Map finding id -> the text of its own `### ID — ` section.

  A section runs to the next heading at its own depth or shallower, or to end
  of file — a deeper heading nests inside instead of closing it, the same rule
  `corpus_lib._section_rows` states for the same reason. Fenced blocks are
  kept in the body — an `applied` disposition's evidence is quoted output,
  which only exists inside a fence — but a `###` *inside* one is not a
  heading. This corpus publishes recipes that print markdown, so the
  distinction is live and not hypothetical; `corpus_lib.finding_owners` draws
  it the same way, and two readers of one file disagreeing about where a
  section ends is worse than either being wrong.

  Fix(disposition_sections_close_is_a_prefix_not_a_depth): the close check
  used to be `line.startswith( '### ' )` — an exact-depth-3 test that missed
  a *shallower* heading (`##` or `#`) closing the section too. Root cause:
  same bug class this module's sibling checks already carry a named fix for
  (`corpus_lib.py`'s `a_deeper_heading_is_not_a_boundary`,
  `citations.py`'s `citations_tests_open_is_a_prefix_not_a_heading`) — a
  prefix test answers "is this exactly a level-3 heading," not "has a new,
  same-or-higher-level section begun." Live shape: `docs/algorithm/*.md`
  routinely follows its last finding with `## Termination` and
  `## Cross-References` — whole-document sections, not part of any one
  finding — before the closing `### Sources` / `### Tests`; the old check
  let both bleed into that last finding's own disposition-checked body.
  Pitfall: zero effect on any verdict today — measured, not assumed: for
  every finding in every finding-owning file across all 33 crates (872
  files), the `**Disposition:**` hit-count `check()` derives from the
  section body is identical whether the close boundary is this depth-aware
  version or the old exact-prefix one, because none of the absorbed
  `##`-level content anywhere in this corpus happens to contain a
  `**Disposition:**`-shaped line today.
  """
  text = pathlib.Path( path ).read_text()
  lines = text.splitlines( keepends = True )
  out = {}
  current = None
  buf = []
  fenced = False
  for line in lines:
    fence = line.startswith( '```' )
    if fenced or fence:
      if current:
        buf.append( line )
      if fence:
        fenced = not fenced
      continue
    m = corpus_lib.FINDING_HEADING.match( line )
    if m:
      if current:
        out[ current ] = ''.join( buf )
      current = m.group( 1 )
      buf = []
      continue
    hm = corpus_lib.HEADING.match( line )
    if hm and len( hm.group( 1 ) ) <= 3 and current:
      out[ current ] = ''.join( buf )
      current = None
      buf = []
      continue
    if current:
      buf.append( line )
  if current:
    out[ current ] = ''.join( buf )
  return out


def paragraphs( body ):
  """Blank-line-separated paragraphs, each joined back into one line.

  A disposition is read as a paragraph rather than as a line because every other
  document in this corpus wraps its prose at roughly eighty columns, and a rule
  that a disposition must fit on one physical line would be a rule that it must
  be written unlike everything around it. The `Now prints:` clause then anchors
  at the end of the joined paragraph, so it stays the last thing said.
  """
  out = []
  buf = []
  for line in body.splitlines():
    if line.strip():
      buf.append( line.strip() )
    elif buf:
      out.append( ' '.join( buf ) )
      buf = []
  if buf:
    out.append( ' '.join( buf ) )
  return out


def check( standard, docs ):
  """Every reachable-tier finding in one crate. Returns a list of problems."""
  reachable = reachable_tiers( standard )
  owners = corpus_lib.finding_owners( docs )
  problems = []
  printed = {}

  for ( _, fid, _, _, tier, _ ) in corpus_lib.master_findings( docs ):
    if tier not in reachable:
      continue
    owner = owners.get( fid )
    if owner is None:
      problems.append( f'DISP {fid}: on {tier} and no instance declares it' )
      continue

    if owner not in printed:
      text = owner.read_text()
      printed[ owner ] = '\n'.join( LIVE_OUTPUT.findall( text ) )
    body = sections( owner ).get( fid, '' )
    rel = f'{owner.parent.name}/{owner.name}'

    hits = [ DISPOSITION.match( par ) for par in paragraphs( body ) ]
    hits = [ h for h in hits if h ]
    if not hits:
      problems.append( f'DISP {fid} ({rel}): {tier}, no **Disposition:** line' )
      continue
    if len( hits ) > 1:
      problems.append( f'FORM {fid} ({rel}): {len( hits )} disposition lines, want 1' )
      continue

    ( verdict, reason ) = ( hits[ 0 ].group( 1 ), hits[ 0 ].group( 2 ) )
    if verdict == 'applied':
      m = NOW_PRINTS.search( reason )
      if not m:
        problems.append(
          f'EVID {fid} ({rel}): applied without a `Now prints:` clause'
        )
      elif m.group( 1 ) not in printed[ owner ]:
        problems.append(
          f'EVID {fid} ({rel}): applied, and no Live output block prints '
          f'{m.group( 1 )!r}'
        )
    else:
      if len( reason ) < MIN_REASON:
        problems.append(
          f'WHY {fid} ({rel}): declined in {len( reason )} chars, want {MIN_REASON}'
        )
      elif not CONCRETE.search( reason ):
        problems.append( f'WHY {fid} ({rel}): declined and names nothing concrete' )

  return problems


def main():
  argv = sys.argv[ 1 : ]
  tiers = []
  while '--tier' in argv:
    i = argv.index( '--tier' )
    tiers.append( argv[ i + 1 ] )
    del argv[ i : i + 2 ]
  if len( argv ) < 2:
    print( 'usage: disposition.py <corpus_standard.txt> [--tier T]... <docs>...' )
    return 2

  standard = corpus_lib.read_standard( argv[ 0 ] )
  # `--tier` narrows the gate to one tier at a time so a stage correcting the
  # hazards can take a verdict of its own while the costs are still open. It can
  # only ever narrow: a tier named here that the standard does not declare is a
  # typo that would otherwise report a clean run over nothing.
  if tiers:
    declared = reachable_tiers( standard )
    unknown = [ t for t in tiers if t not in declared ]
    if unknown:
      print( f'G20 disposition: undeclared tier(s) { unknown }' )
      return 2
    standard = dict( standard, tier = tiers )

  total = 0
  for docs in argv[ 1 : ]:
    total += corpus_lib.report( corpus_lib.crate_name( docs ), check( standard, docs ) )
  scope = ', '.join( tiers ) if tiers else 'all reachable tiers'
  print( f'G20 disposition ({ scope }): { total } problem(s)' )
  return 1 if total else 0


if __name__ == '__main__':
  sys.exit( main() )
