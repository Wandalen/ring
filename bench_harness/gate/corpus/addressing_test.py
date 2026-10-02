"""Tests for `addressing.py`'s crate resolver.

The bug this exists to prevent already happened twice. `crate_src_lib` turns the
package name in a `cargo test --doc -p <pkg>` recipe into the source path a
doctest citation is reported against, and `addresses()` drops the finding
entirely when that returns None — so a resolver that has lost a crate root
reports the crates behind it clean instead of reporting an error. The first
instance took `g10_pinned_math.sh`'s scan globs and went unnoticed over 742
sources; the second took this resolver and went unnoticed over every crate under
`substrate/` and `spike/`, 169 of the tree's 250 libraries.

Both were one relocation away from a resolver that named its roots and depths as
literals. So the assertions here are deliberately not a list of paths: they
quantify over whatever the tree actually holds right now, and fail when the tree
grows a shape the resolver does not cover. A fixture of expected paths would have
passed for both bugs — it would have been updated in the same edit that moved the
crates, or not exercised the moved ones at all.

`unittest`, run directly (`python3 addressing_test.py`), matching
`anchors_test.py` — no gate is wired to either.
"""

import collections
import pathlib
import re
import tempfile
import unittest

import addressing


# The `[package] name = "…"` of a manifest, read from after the `[package]`
# header so a `[dependencies]` entry's own `name =` (a renamed dependency,
# which `g5_export_surface.sh` has its own blind spot about) cannot be mistaken
# for the crate's.
_PACKAGE_NAME = re.compile( r'^\s*name\s*=\s*"([^"]+)"', re.M )

# Crates whose manifest name differs from their directory name. `crate_src_lib`
# matches the directory, so any such crate would resolve to None. See its own
# Finding(crate_src_lib_matches_directory_name_not_package_name). Pinned as an
# exact set rather than skipped as a class, because an entry is a new instance
# of the same silent-None shape and must fail here, not be absorbed. Empty in
# this repo, since none of its 34 crates diverge, but kept as a set rather than
# removed. That way `test_name_differs_from_dir_set_has_not_grown` catches a
# future divergence instead of letting it silently pass the class check below.
_NAME_DIFFERS_FROM_DIR = set()


def _is_scratch( rel ):
  """Whether a repo-relative path lies in build output or gitignored scratch.

  `target` and `-*` are excluded for the same reasons `addressing._group_dirs`
  excludes them; this duplicates that rule on purpose, so the test's own idea of
  which directories are real is independent of the code under test.
  """
  return any( part == 'target' or part.startswith( '-' ) for part in rel.parts )


def _manifests():
  """Every real `[package]` manifest under the four crate roots, as
  ( package-name, repo-relative crate directory, depth-below-root ).
  """
  found = []
  for root in addressing.CRATE_ROOTS:
    for manifest in ( addressing.REPO_ROOT / root ).rglob( 'Cargo.toml' ):
      rel = manifest.relative_to( addressing.REPO_ROOT )
      if _is_scratch( rel ):
        continue
      text = manifest.read_text()
      if '[package]' not in text:
        continue
      m = _PACKAGE_NAME.search( text.split( '[package]', 1 )[ 1 ] )
      if m:
        # parts: root / …groups… / crate / Cargo.toml
        found.append( ( m.group( 1 ), rel.parent, len( rel.parts ) - 2 ) )
  return found


class CrateResolverTest( unittest.TestCase ):

  @classmethod
  def setUpClass( cls ):
    cls.manifests = _manifests()
    cls.libs = [
      ( name, crate, depth ) for ( name, crate, depth ) in cls.manifests
      if ( addressing.REPO_ROOT / crate / 'src' / 'lib.rs' ).is_file()
    ]

  def test_repo_root_is_the_repository_root( self ):
    """`REPO_ROOT` is computed by counting `parents[…]` from this file, so a
    directory inserted anywhere above `corpus/` silently points it at a parent
    of the repo — where every root is missing and every lookup is None.
    """
    self.assertTrue( ( addressing.REPO_ROOT / 'Cargo.toml' ).is_file() )
    for root in addressing.CRATE_ROOTS:
      self.assertTrue(
        ( addressing.REPO_ROOT / root ).is_dir(),
        f'crate root {root}/ does not exist under {addressing.REPO_ROOT}',
      )

  def test_every_crate_root_holds_at_least_one_resolvable_crate( self ):
    """Per-root, never aggregate — the distinction is the whole lesson.

    `g10_pinned_math.sh` asserted its scan was non-empty overall, so when
    `module/substrate/` lost a level and stopped matching, the roots that still
    matched kept the total non-zero and the gate kept reporting. Its comment
    records 742 sources going unread behind that. An aggregate count here would
    reproduce it exactly: `ring/` alone would carry this test green while every
    substrate crate resolved to None.
    """
    by_root = collections.Counter()
    for ( name, _crate, _depth ) in self.libs:
      if name in _NAME_DIFFERS_FROM_DIR:
        continue
      target = addressing.crate_src_lib( name )
      if target:
        by_root[ target.split( '/', 1 )[ 0 ] ] += 1
    for root in addressing.CRATE_ROOTS:
      self.assertGreater(
        by_root[ root ], 0,
        f'no crate resolved under {root}/, so every crate in that root is '
        f'invisible to G21 while the other roots keep the gate looking busy',
      )

  def test_every_library_crate_resolves_to_its_own_source( self ):
    """The universal quantifier, and the assertion that actually catches a
    relocation: every library in the tree, resolved by the name a `-p` flag
    would carry, must come back as its own `src/lib.rs`.
    """
    unresolved, wrong = [], []
    for ( name, crate, _depth ) in self.libs:
      if name in _NAME_DIFFERS_FROM_DIR:
        continue
      expected = f'{crate.as_posix()}/src/lib.rs'
      target = addressing.crate_src_lib( name )
      if target is None:
        unresolved.append( name )
      elif target != expected:
        wrong.append( ( name, expected, target ) )
    self.assertEqual( unresolved, [], f'{len( unresolved )} crate(s) resolved to None' )
    self.assertEqual( wrong, [], f'{len( wrong )} crate(s) resolved to another crate' )

  def test_resolution_covers_every_depth_the_tree_uses( self ):
    """`crate_src_lib` walks exactly three depths. This asserts the tree uses no
    fourth — the failure mode is silent (a deeper crate resolves to None), so it
    has to be caught by comparing against the tree rather than by a lookup.
    """
    depths = sorted( { depth for ( _n, _c, depth ) in self.manifests } )
    self.assertEqual(
      depths, [ 1, 2, 3 ],
      'the tree nests crates at a depth the resolver does not walk',
    )

  def test_returned_paths_are_repo_relative_and_exist( self ):
    """A citation is printed verbatim into a G21 problem line, so a path that
    does not resolve from the repo root is a stale citation minted by the gate
    written to grade them — the exact defect this resolver's own comment names.
    """
    for ( name, _crate, _depth ) in self.libs[ :: 20 ]:
      if name in _NAME_DIFFERS_FROM_DIR:
        continue
      target = addressing.crate_src_lib( name )
      self.assertFalse( target.startswith( '/' ), f'{name} resolved to an absolute path' )
      self.assertTrue( ( addressing.REPO_ROOT / target ).is_file() )

  def test_unknown_package_resolves_to_none( self ):
    """The false-positive control. A resolver that answered every name would
    pass every assertion above and report a citation against a path it invented.
    """
    self.assertIsNone( addressing.crate_src_lib( 'no_such_crate_anywhere' ) )
    self.assertIsNone( addressing.crate_src_lib( '' ) )

  def test_name_differs_from_dir_set_has_not_grown( self ):
    """Pins the known divergence — empty in this repo. Any entry excluded from
    the assertions above would silently inherit the exclusion instead of being
    noticed, so the set stays pinned rather than open-ended even at zero size.
    """
    live = {
      name for ( name, crate, _depth ) in self.manifests
      if name != crate.name
    }
    self.assertEqual( live, _NAME_DIFFERS_FROM_DIR )


class DoctestFindingTest( unittest.TestCase ):
  """The resolver's consequence, end to end: `addresses()` discards the finding
  when `crate_src_lib` returns None, which is why a broken resolver reports
  clean. These pin that a resolvable package produces a finding and that the
  discard is reached only when the package genuinely is not in the tree.
  """

  _QUOTED = 'running 1 test\ntest widget_store::read (line 306) ... ok\n'

  def test_package_route_produces_a_finding( self ):
    found = addressing.addresses( 'cargo test --doc -p widget_store', self._QUOTED )
    self.assertEqual( len( found ), 1 )
    ( shown, target ) = found[ 0 ]
    self.assertIn( '(line 306)', shown )
    self.assertEqual( target, 'widget/widget_store/src/lib.rs' )
    self.assertTrue( addressing.FILE_TARGET.match( target ) )

  def test_unresolvable_package_is_silently_dropped( self ):
    """Not an aspiration — a recorded property of the current design. The drop
    is `addresses()`'s `if target:`, and it is exactly what made the resolver
    bug invisible. Asserted so that the silence is a decision on record rather
    than a thing the next reader has to rediscover.
    """
    self.assertEqual(
      addressing.addresses( 'cargo test --doc -p no_such_crate_anywhere', self._QUOTED ),
      [],
    )

  def test_inline_path_wins_over_the_package_route( self ):
    """`cargo`'s own output names the file when it has one, and that spelling is
    preferred — the package route is the fallback, so a resolver defect can only
    reach citations cargo did not spell out itself.
    """
    quoted = 'test ring/ring_seqno/src/lib.rs - Seq::next (line 88) ... ok\n'
    found = addressing.addresses( 'cargo test --doc -p widget_store', quoted )
    self.assertEqual( [ t for ( _s, t ) in found ], [ 'ring/ring_seqno/src/lib.rs' ] )

  def test_probe_recording_has_no_command_to_read_a_package_from( self ):
    """`recipes.blocks()` yields `command = None` for a probe recording, and
    `addresses()` must reach the doctest scan anyway rather than raising — the
    defect its own docstring records as this gate's fifth.
    """
    self.assertEqual( addressing.addresses( None, self._QUOTED ), [] )


class DocsWalkTest( unittest.TestCase ):
  """`docs_files` on a path that is not there.

  Third instance of one shape. The resolver bug above reported crates clean by
  resolving their sources to None; `g10_pinned_math.sh` reported 742 sources
  clean by globbing a path level that had moved; this one reported an entire
  crate clean by walking a directory that did not exist. All three answer "no
  problems found" and "I could not look" with the same output, and this tree has
  been relocated enough times that a stale docs path is the ordinary input, not
  the exotic one.

  The five sibling checkers all reach `iterdir` and raise there, which
  `common.sh`'s `checker_crashed()` already reads as an abort rather than a
  verdict. Only `addressing.py` reached `rglob`, which tolerates a missing root.
  """

  @classmethod
  def setUpClass( cls ):
    cls.live_docs = [
      addressing.REPO_ROOT / crate / 'docs'
      for ( _n, crate, _d ) in _manifests()
      if ( addressing.REPO_ROOT / crate / 'docs' ).is_dir()
    ]

  def test_missing_docs_directory_raises( self ):
    """The regression. Before the fix this returned `[]`, `check()` looped zero
    times, and `main()` printed `0 problem(s)` and exited 0 — a pass for a crate
    the checker never opened.
    """
    with self.assertRaises( FileNotFoundError ):
      addressing.docs_files( '/no/such/docs/directory/at/all' )

  def test_check_propagates_rather_than_reporting_clean( self ):
    """Through the caller, not just the leaf. `check()` is the only consumer and
    the gate's whole entry point, so a `docs_files` that raises is worth nothing
    if `check()` were ever to swallow it back into an empty problem list.
    """
    with self.assertRaises( FileNotFoundError ):
      addressing.check( '/no/such/docs/directory/at/all' )

  def test_a_path_that_is_a_file_raises_too( self ):
    """`is_dir()` rather than `exists()`. A docs path that resolves to a file is
    the same failure wearing a different hat, and `rglob` on one also yields
    nothing instead of complaining.
    """
    with self.assertRaises( FileNotFoundError ):
      addressing.docs_files( addressing.REPO_ROOT / 'Cargo.toml' )

  def test_an_empty_directory_is_not_an_error( self ):
    """The distinction the fix has to preserve: a crate whose `docs/` genuinely
    holds no markdown is clean, not broken. Without this the fix could be
    "raise on anything that returns nothing", which trades a false pass for a
    false failure.
    """
    with tempfile.TemporaryDirectory() as empty:
      self.assertEqual( addressing.docs_files( empty ), [] )
      self.assertEqual( addressing.check( empty ), [] )

  def test_a_real_docs_tree_still_walks( self ):
    """The false-positive control. Every assertion above passes for a
    `docs_files` that raises unconditionally, which would take the gate offline
    across the whole corpus while looking, in this file, like a fix.

    Quantified over the tree rather than pinned to a path, for the reason this
    module's docstring gives: a literal would be updated by the same edit that
    moved the crate.
    """
    self.assertGreater( len( self.live_docs ), 0, 'no crate in the tree carries docs/' )
    for docs in self.live_docs[ :: 25 ]:
      found = addressing.docs_files( docs )
      self.assertGreater( len( found ), 0, f'{docs} walked to zero markdown files' )
      self.assertTrue( all( p.suffix == '.md' for p in found ) )


if __name__ == '__main__':
  unittest.main()
