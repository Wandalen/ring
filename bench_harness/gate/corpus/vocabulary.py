"""G17 — the tier vocabulary is closed, and the finding ids are a clean set.

Takes one or more `docs/` directories. Two assertions:

  * **Closed vocabulary.** Every tier string in every findings table — in the
    definition readmes and in the Module Index alike — is one of the thirteen
    declared in `corpus_standard.txt`. The bolding is part of the string and
    part of the check: a reachable tier is bolded, a record tier is not, and a
    hybrid like `n/a — latent hazard` claims both at once. Four are live in
    `ring_publish`'s Module Index right now, which makes this gate's first real
    hit an existing defect rather than a seeded one.

  * **Clean id set.** Ids within a crate are unique and contiguous from 1. A gap
    means a finding was deleted and its successors not renumbered — every
    cross-reference to the numbers after the gap now points one place off — and
    a duplicate means two findings answer to one name, so a citation to it
    resolves to whichever the reader happens to open.

Scanning both tables matters. The definition readmes and the Module Index are
independently maintained, and the four hybrids prove they diverge: the family's
632 definition-readme rows are clean, and the malformed strings are all in an
index that restates them.
"""

import pathlib
import re
import sys

import corpus_lib as lib


def check( docs, tiers ):
  problems = []
  docs = pathlib.Path( docs )
  seen = {}

  for d in sorted( x for x in docs.iterdir() if x.is_dir() and x.name != 'definition' ):
    for ( ln, fid, _, tier ) in lib.declared_findings( d / 'readme.md' ):
      if tier not in tiers:
        problems.append( f'TIER   {d.name}/readme.md:{ln}  {fid}  "{tier}" is not a declared tier' )
      seen.setdefault( fid, [] ).append( f'{d.name}/readme.md:{ln}' )

  for ( ln, fid, _, _, tier, _ ) in lib.master_findings( docs ):
    if tier not in tiers:
      problems.append( f'TIER   definition/readme.md:{ln}  {fid}  "{tier}" is not a declared tier' )

  dup = sorted( f for ( f, w ) in seen.items() if len( w ) > 1 )
  for fid in dup:
    problems.append( f'DUP    {fid} declared in {len( seen[ fid ] )} places: {" ".join( seen[ fid ] )}' )

  nums = sorted( int( re.sub( r'^[A-Z]{2}', '', f ) ) for f in seen )
  if nums:
    gaps = [ n for n in range( 1, nums[ -1 ] + 1 ) if n not in set( nums ) ]
    if gaps:
      shown = ' '.join( str( n ) for n in gaps[ :12 ] ) + ( ' …' if len( gaps ) > 12 else '' )
      problems.append( f'GAP    {len( gaps )} id(s) missing below {nums[ -1 ]}: {shown}' )

  prefixes = { re.sub( r'\d+$', '', f ) for f in seen }
  if len( prefixes ) > 1:
    problems.append( f'PREFIX {len( prefixes )} id prefixes in one crate: {" ".join( sorted( prefixes ) )}' )

  return problems


def main( argv ):
  standard = lib.read_standard( argv[ 1 ] )
  tiers = set( standard.get( 'tier', [] ) )
  if not tiers:
    print( 'the standard declares no tiers — nothing to check against' )
    return 1
  bad = 0
  for docs in argv[ 2 : ]:
    bad += lib.report( lib.crate_name( docs ), check( docs, tiers ) )
  print( f'{bad} problem(s)' )
  return 1 if bad else 0


if __name__ == '__main__':
  sys.exit( main( sys.argv ) )
