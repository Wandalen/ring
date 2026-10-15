"""Tests for `anchors.py`'s dead-clause detection.

The fixture is not invented: it is `ring_slot`'s own git history, at the
revision where the `docs/pitfall/readme.md` awk recipe's first clause anchored
on a doc-comment line a prior edit to `src/lib.rs` had already reworded away,
while the recipe's other two clauses still matched and the block still printed
eight lines. That revision (`5b281d8f`, before the recipe was repaired to
track the reworded line) is exactly the PROVEN SPECIMEN a partially-dead
multi-clause recipe needs and nothing else in this corpus's control fixtures
covers, so it is read straight out of `git show` rather than copied into a
static fixture file that could drift from what git actually recorded.
"""

import pathlib
import subprocess
import tempfile
import unittest

import anchors


# The revision immediately before the readme's awk recipe was repaired to
# track a doc-comment reworded in an earlier commit. See the module docstring.
_PRE_REPAIR_REV = '5b281d8f'
_LIB_PATH = 'module/ring_slot/src/lib.rs'
_README_PATH = 'module/ring_slot/docs/pitfall/readme.md'


def _git_show( root, rev, path ):
  """The exact content of `path` at `rev`, straight from this repo's own git
  history — `git show` is a read-only inspection of an already-committed
  object, never a write to the working tree.
  """
  r = subprocess.run(
    [ 'git', 'show', f'{rev}:{path}' ],
    cwd = root, capture_output = True, text = True, check = True,
  )
  return r.stdout


class DeadClauseDetectionTest( unittest.TestCase ):
  """Encodes the VERIFY requirement: run the anchor-existence check against
  the pre-repair revision of `ring_slot/docs/pitfall/readme.md` and confirm it
  flags exactly the one dead clause, while the block's own recorded output is
  non-empty — so the finding is never confused with a fully-vacuous block.
  """

  @classmethod
  def setUpClass( cls ):
    cls.repo_root = anchors._repo_root( pathlib.Path( __file__ ) )
    assert cls.repo_root is not None, 'could not locate the repo root from this test file'
    readme = _git_show( cls.repo_root, _PRE_REPAIR_REV, _README_PATH )
    cls.lib_rs = _git_show( cls.repo_root, _PRE_REPAIR_REV, _LIB_PATH )
    # The one awk line, lifted unmodified from the historical readme. The test
    # drops the surrounding `cd` and grep lines deliberately. The historical
    # `cd` names this machine's *live* tree, which no longer has anything under
    # `module/ring_slot` (the family moved to `ring/` since this revision).
    # Carried along, it would misreport a moved target as a dead anchor instead
    # of exercising the one clause this test is about.
    lines = [ l for l in readme.splitlines() if l.startswith( "awk '" ) ]
    assert len( lines ) == 1, f'expected exactly one awk line, found {len( lines )}'
    cls.awk_line = lines[ 0 ]

  def setUp( self ):
    tmp = tempfile.TemporaryDirectory( prefix = '-anchors_test_' )
    self.addCleanup( tmp.cleanup )
    self.root = pathlib.Path( tmp.name )
    target = self.root / _LIB_PATH
    target.parent.mkdir( parents = True )
    target.write_text( self.lib_rs )

  def test_awk_clauses_finds_all_three( self ):
    clauses = anchors.awk_clauses( self.awk_line )
    self.assertEqual( len( clauses ), 3 )

  def test_dead_clauses_flags_exactly_the_deleted_anchor( self ):
    problems = anchors.dead_clauses( self.awk_line, self.root )
    self.assertEqual( len( problems ), 1 )
    ( pattern, target, reason ) = problems[ 0 ]
    self.assertIn( 'dropping whatever it held', pattern )
    self.assertEqual( target, _LIB_PATH )
    self.assertEqual( reason, 'anchor no longer found in target' )

  def test_surviving_clauses_are_not_flagged( self ):
    problems = anchors.dead_clauses( self.awk_line, self.root )
    flagged_patterns = ' '.join( p for ( p, _t, _r ) in problems )
    self.assertNotIn( 'is_none', flagged_patterns )
    self.assertNotIn( 'is_empty', flagged_patterns )

  def test_recorded_output_is_non_empty( self ):
    """The block itself is not the fully-vacuous case (CLASS 16) — the two
    surviving clauses still print, which is exactly what makes the partial
    death invisible to a line-count check.
    """
    r = subprocess.run(
      [ 'bash', '-c', self.awk_line ], cwd = self.root,
      capture_output = True, text = True, check = True,
    )
    self.assertEqual( len( r.stdout.splitlines() ), 8 )
    self.assertTrue( r.stdout.strip() )

  def test_repaired_live_recipe_has_no_dead_clauses( self ):
    """The current, already-repaired `ring_slot` recipe must report clean —
    the false-positive check this detector needs just as much as the
    true-positive one above (a detector that flags everything is exactly as
    useless as one that flags nothing).
    """
    docs = self.repo_root / 'ring' / 'ring_slot' / 'docs'
    self.assertEqual( anchors.check( docs ), [] )


if __name__ == '__main__':
  unittest.main()
