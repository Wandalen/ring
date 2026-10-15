"""G14 — corpus shape: the counts, and the three places each is stated.

Takes one or more `docs/` directories. Exits 0 only when every one of them holds
the declared definitions, clears the instance and finding floors, and states the
same three numbers in the tree, in each definition readme's own findings table,
and in the Module Index.

The third assertion is the one that does the work. Counting files is a check a
fresh `mkdir` satisfies; requiring the count to agree with two independently
written restatements of itself is a check that fails the moment a finding is
added to a document and not to the index that claims to enumerate it, which is
the actual failure mode this corpus has.
"""

import pathlib
import sys

import corpus_lib as lib


def check( docs, standard ):
  problems = []
  docs = pathlib.Path( docs )

  want_defs = set( standard.get( 'definition', [] ) )
  min_defs = int( standard[ 'min_definitions' ][ 0 ] )
  min_inst = int( standard[ 'min_instances' ][ 0 ] )
  min_find = int( standard[ 'min_findings' ][ 0 ] )

  present = set( lib.definition_dirs( docs ) )
  missing = sorted( want_defs - present )
  if missing:
    problems.append( f'DEFS   missing {len( missing )}: {" ".join( missing )}' )
  extra = sorted( present - want_defs - { 'definition' } )
  if extra:
    problems.append( f'DEFS   undeclared {len( extra )}: {" ".join( extra )} — rule it into the standard or fold it' )
  if 'definition' not in present:
    problems.append( 'DEFS   no definition/ — the Module Index has nowhere to live' )
  if len( present & want_defs ) < min_defs:
    problems.append( f'DEFS   {len( present & want_defs )}/{min_defs}' )

  inst = lib.instances( docs )
  if len( inst ) < min_inst:
    problems.append( f'INST   {len( inst )}/{min_inst}' )

  # Every declared definition needs at least two instances of its own. A crate
  # can clear the family floor of 26 with thirteen instances in one directory
  # and none in the other twelve, which is the same count and not the same
  # corpus.
  per_def = {}
  for ( d, _, _ ) in inst:
    per_def[ d ] = per_def.get( d, 0 ) + 1
  thin = sorted( d for d in want_defs & present if per_def.get( d, 0 ) < 2 )
  if thin:
    problems.append(
      f'INST   {len( thin )} definition(s) under 2 instances: '
      + ' '.join( f'{d}={per_def.get( d, 0 )}' for d in thin )
    )

  owners = lib.finding_owners( docs )
  if len( owners ) < min_find:
    problems.append( f'FIND   {len( owners )}/{min_find}' )

  # The same count, from the definition readmes.
  declared = {}
  for d in sorted( want_defs & present ):
    for ( ln, fid, subj, tier ) in lib.declared_findings( docs / d / 'readme.md' ):
      declared[ fid ] = ( d, ln, subj, tier )
  only_heading = sorted( set( owners ) - set( declared ) )
  only_declared = sorted( set( declared ) - set( owners ) )
  if only_heading:
    problems.append(
      f'AGREE  {len( only_heading )} finding(s) have a heading and no definition-readme row: '
      + ' '.join( only_heading[ :8 ] ) + ( ' …' if len( only_heading ) > 8 else '' )
    )
  if only_declared:
    problems.append(
      f'AGREE  {len( only_declared )} definition-readme row(s) name no finding heading: '
      + ' '.join( only_declared[ :8 ] ) + ( ' …' if len( only_declared ) > 8 else '' )
    )

  # And again, from the Module Index.
  master = { r[ 1 ] for r in lib.master_findings( docs ) }
  if master:
    only_master = sorted( master - set( owners ) )
    unindexed = sorted( set( owners ) - master )
    if only_master:
      problems.append(
        f'INDEX  {len( only_master )} Module Index row(s) name no finding heading: '
        + ' '.join( only_master[ :8 ] ) + ( ' …' if len( only_master ) > 8 else '' )
      )
    if unindexed:
      problems.append(
        f'INDEX  {len( unindexed )} finding(s) absent from the Module Index: '
        + ' '.join( unindexed[ :8 ] ) + ( ' …' if len( unindexed ) > 8 else '' )
      )
  else:
    problems.append( 'INDEX  Module Index has no ## Findings table — 0 of the corpus is indexed' )

  return problems


def main( argv ):
  standard = lib.read_standard( argv[ 1 ] )
  bad = 0
  for docs in argv[ 2 : ]:
    bad += lib.report( lib.crate_name( docs ), check( docs, standard ) )
  print( f'{bad} problem(s)' )
  return 1 if bad else 0


if __name__ == '__main__':
  sys.exit( main( sys.argv ) )
