"""One row per crate: what it has, what the standard asks, and the shortfall.

This is the observable half of the plan's demo. G14-G17 answer REACHED or NOT
REACHED and print the problems behind the verdict, which is what a gate is for
and is not something a person can read progress off. The scoreboard is the same
tree counted the same way, arranged so that the distance left to travel is
visible in one screen.

It takes no verdict and is not a gate. Every number here is also asserted by
G14, which is the point: a scoreboard that could disagree with the gate would be
a second source of truth, and this one is a rendering of the first.
"""

import pathlib
import sys

import corpus_lib as lib


def main( argv ):
  standard = lib.read_standard( argv[ 1 ] )
  want = set( standard.get( 'definition', [] ) )
  ( md, mi, mf ) = (
    int( standard[ 'min_definitions' ][ 0 ] ),
    int( standard[ 'min_instances' ][ 0 ] ),
    int( standard[ 'min_findings' ][ 0 ] ),
  )

  rows = []
  for docs in argv[ 2 : ]:
    docs = pathlib.Path( docs )
    present = set( lib.definition_dirs( docs ) ) & want
    inst = len( lib.instances( docs ) )
    find = len( lib.finding_owners( docs ) )
    rows.append( ( lib.crate_name( docs ), len( present ), inst, find ) )

  # Finding(at_standard_checks_three_of_g14s_seven_categories): both this sort
  # key and `at` below compare only the three headline counts (def/inst/find)
  # against their floors. shape.check() — the actual G14 verdict this
  # docstring claims never to disagree with — also fails a crate for an
  # `extra` undeclared definition dir, a `missing` declared one, a `thin`
  # definition under two instances, or an AGREE/INDEX mismatch between a
  # finding's heading, its definition-readme row and its Module Index row —
  # none of which this file reads at all. A crate could clear def/inst/find
  # and still carry any of those, showing "at standard" here while G14
  # reports it NOT REACHED. No live trigger: checked directly, not assumed —
  # ran shape.check() against every one of the 33 real ring crates and
  # compared its REACHED/NOT-REACHED verdict to this file's own `at`
  # computation for each; zero disagreements today. That is a property of
  # the current corpus being clean on every category this file does not
  # check, not a guarantee this code provides — the two computations are
  # independent, and the docstring's "a rendering of the first" claim holds
  # only as long as that coincidence does. Closing the gap for good means
  # driving `at` from shape.check()'s own verdict (import and call it per
  # crate) rather than re-deriving three of its seven conditions here, which
  # is a heavier change than a drive-by.
  rows.sort( key = lambda r: ( r[ 1 ] >= md and r[ 2 ] >= mi and r[ 3 ] >= mf, r[ 3 ], r[ 2 ], r[ 0 ] ) )

  print( f'  {"crate":<18} {"defs":>9} {"inst":>9} {"findings":>11}   state' )
  print( f'  {"-" * 18} {"-" * 9} {"-" * 9} {"-" * 11}   {"-" * 28}' )

  done = 0
  ( sd, si, sf ) = ( 0, 0, 0 )
  for ( name, d, i, f ) in rows:
    sd += d
    si += i
    sf += f
    at = d >= md and i >= mi and f >= mf
    done += at
    gaps = []
    if d < md:
      gaps.append( f'+{md - d} def' )
    if i < mi:
      gaps.append( f'+{mi - i} inst' )
    if f < mf:
      gaps.append( f'+{mf - f} find' )
    state = 'at standard' if at else ', '.join( gaps )
    print( f'  {name:<18} {d:>4}/{md:<4} {i:>4}/{mi:<4} {f:>5}/{mf:<5}   {state}' )

  n = len( rows )
  print( f'  {"-" * 18} {"-" * 9} {"-" * 9} {"-" * 11}   {"-" * 28}' )
  print( f'  {"total":<18} {sd:>4}/{md * n:<4} {si:>4}/{mi * n:<4} {sf:>5}/{mf * n:<5}   {done}/{n} at standard' )
  return 0


if __name__ == '__main__':
  sys.exit( main( sys.argv ) )
