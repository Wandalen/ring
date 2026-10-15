"""G15 — every published recipe still prints what it says it prints.

Takes one or more `docs/` directories, executes every ```sh block in them, and
compares. What it compares against depends on which file the block is in, and
the split is deliberate rather than an exemption:

  * **Instance files** publish a recipe and quote its output beneath a
    `Live output:` line. The quoted block must equal the command's stdout+stderr
    byte for byte, after one normalization: cargo's `Blocking waiting for file
    lock` lines are dropped, because whether they appear depends on what else
    was running rather than on anything about the crate (see
    `_CARGO_LOCK_NOISE`). A block with no quoted output is a problem — a recipe
    nobody can check against anything is the shape the repository's own
    conventions rulebook names under *Documentation : Executable Recipes*.

  * **Probe recordings** are a narrower case of an instance file: three crates
    (ring_event, ring_registry, ring_trace) pair a lang-tagged fence — the
    probe's own source, not a shell recipe — with a bare fence holding what
    running it printed, with no `Live output:` label between them. The probe
    binary is a hyphen-prefixed scratch file, gitignored and swept between
    sessions by this project's own naming convention, so there is nothing left
    to re-run: the pairing is recognized by the probe's own
    `// -x_probe/src/bin/name.rs` source comment as the fence's first body
    line, and reported as unverifiable rather than compared for staleness.

  * **Definition readmes** publish a `### Regenerate` block that prints the
    counts stated in the prose above it, and quote nothing. That is correct
    house style and not a gap: quoting those counts would make every definition
    readme in the crate go stale the instant a finding is added anywhere, which
    is a worse failure than the one quoting would catch. A block with no quoted
    output is held to exit 0 instead, which is what the sanity-run idiom checks
    by hand today. But a definition readme can also publish a recipe that *does*
    quote a `Live output:` fence — a measurement rather than a Regenerate action
    — and that fence is compared exactly like an instance file's, STALE on
    mismatch, once its own exit-0 check passes.

  * **The crate-level `docs/readme.md`** — one level up from every definition
    readme above — is graded the same way: exit 0, nothing compared, because a
    recipe there is the same shape, a demo rather than a claim about a
    specific count. Unlike a definition readme it is not required to carry
    one at all: most crates' top-level readme is prose with no recipe, and a
    still-scaffolded crate may carry no top-level readme.md yet (`docs/`
    holding only `workaround/`). Neither absence is a G15 problem — only a
    recipe that is actually published there and fails to run cleanly is.

The distinction is the whole reason this gate is not `-verify_blocks.py` run
over `docs/**`: that script reports 14 "no quoted output block to compare"
problems per crate, one per definition readme, so its output on a perfect crate
and on a broken one differ only in the count of an expected message.
"""

import pathlib
import re
import subprocess
import sys

import corpus_lib as lib


# Cargo writes this to stderr, once per invocation, whenever another cargo
# process holds the lock it wants. It is a scheduling artefact of whatever else
# happened to be running that minute, never evidence about the crate. Because
# `run` compares stdout+stderr, a recipe carrying an unfiltered cargo
# call would otherwise pass alone and fail beside a concurrent build. Dropped
# from the *actual* side only: a document that already baked one of these in
# stays a loud STALE until someone cleans it, which is the correct outcome.
_CARGO_LOCK_NOISE = re.compile( r'^\s*Blocking waiting for file lock on .*$' )

# A probe recording's first body line: `// -ev_probe/src/bin/recycled_bytes.rs`.
# The probe directory is always hyphen-prefixed (temporary, gitignored) and
# always named `*_probe`, per the three crates that use this convention.
_PROBE_COMMENT = re.compile( r'^// -\w+_probe/src/bin/\w+\.rs' )


def _fence_body( lines, start ):
  """Lines from `start` up to (not including) the next closing ``` fence.

  Returns ( collected, index-of-closing-fence ). Shared by every block shape
  `blocks()` recognizes — a recipe body, a recipe's quoted output, and a
  probe's quoted output are each just "run to the next bare fence."
  """
  out = []
  m = start
  while m < len( lines ) and lines[ m ].rstrip() != '```':
    out.append( lines[ m ] )
    m += 1
  return ( out, m )


def blocks( path ):
  """Yield ( lineno, command-or-None, quoted-or-None ) for every recipe or
  probe recording in a file.

  Fence parsing is line-anchored rather than regex-based: this corpus publishes
  recipes that grep for fence markers, so a non-anchored parser closes a block
  on a pattern inside it.

  Two shapes are recognized. A ```sh fence is a recipe: `command` is its body,
  and its quoted output, if any, follows a `Live output:` label. A fence
  tagged anything else whose first body line matches `_PROBE_COMMENT` is a
  probe recording instead: `command` is None — there is nothing left to
  re-run, see `_PROBE_COMMENT` — and its paired bare fence, with no
  `Live output:` label, is captured as `quoted` regardless.

  Finding(blocks_unrecognized_fence_body_not_skipped), not fixed here: a
  fence tagged with anything else — ```rust, ```toml, ```text — that is
  NOT probe-shaped matches neither branch below and falls through to a bare
  `i += 1`, walking its body one line at a time rather than jumping past its
  closing fence the way a recognized ```sh or probe fence does. If that
  body happens to contain a line that is, verbatim, `` ```sh `` — a doc
  comment or string literal quoting shell-recipe syntax as an example,
  nested inside the outer fence — this function would misread that inner
  line as a NEW top-level recipe opener, the identical "nested fence-marker
  content fools a line-scanner" hazard `corpus_lib.py`'s own
  `Finding(prose_lines_blockquoted_fence_invisible)` names for a different
  parser, and the one this file's own module docstring already warns a
  non-anchored parser would fall into — line-anchoring alone does not fully
  close it for a fence type this function does not otherwise recognize. No
  live trigger: measured directly, not assumed, over the entire ring
  corpus's 2353 real recipe/probe blocks — every extracted command
  containing a literal ``` substring (32 of them) parses as syntactically
  valid bash, which a truncated or wrongly-bounded extraction from this
  hazard would be unlikely to do by chance, and none contains Rust or
  markdown syntax suggesting content pulled from the wrong side of a fence
  boundary. A full fix would give every unrecognized fence the same
  `_fence_body`-based skip the ```sh and probe branches already get,
  rather than leaving it to fall through.
  """
  lines = pathlib.Path( path ).read_text().splitlines()
  out = []
  i = 0
  while i < len( lines ):
    fence = lines[ i ].rstrip()
    if fence == '```sh':
      ( body, j ) = _fence_body( lines, i + 1 )
      k = j + 1
      while k < len( lines ) and lines[ k ].strip() == '':
        k += 1
      if k < len( lines ) and lines[ k ].strip() == 'Live output:':
        k += 1
        while k < len( lines ) and lines[ k ].strip() == '':
          k += 1
        if k < len( lines ) and lines[ k ].rstrip() == '```':
          ( quoted, m ) = _fence_body( lines, k + 1 )
          out.append( ( i + 1, '\n'.join( body ), '\n'.join( quoted ) ) )
          i = m + 1
          continue
      out.append( ( i + 1, '\n'.join( body ), None ) )
      i = j + 1
      continue
    if fence.startswith( '```' ) and len( fence ) > 3:
      next_line = lines[ i + 1 ] if i + 1 < len( lines ) else ''
      if _PROBE_COMMENT.match( next_line ):
        ( _, j ) = _fence_body( lines, i + 1 )
        k = j + 1
        while k < len( lines ) and lines[ k ].strip() == '':
          k += 1
        if k < len( lines ) and lines[ k ].rstrip() == '```':
          ( quoted, m ) = _fence_body( lines, k + 1 )
          out.append( ( i + 1, None, '\n'.join( quoted ) ) )
          i = m + 1
          continue
    i += 1
  return out


def run( cmd ):
  r = subprocess.run( [ 'bash', '-c', cmd ], capture_output = True, text = True )
  merged = ( r.stdout + r.stderr ).rstrip( '\n' )
  kept = [ l for l in merged.split( '\n' ) if not _CARGO_LOCK_NOISE.match( l ) ]
  return ( r.returncode, '\n'.join( kept ).rstrip( '\n' ) )


def _check_readme( readme, rel, problems, optional = False ):
  """Append one readme's own G15 problems: missing file, or a `### Regenerate`
  recipe that does not exit 0. `rel` is the directory's display name —
  `item/struct` for a kind-nested readme, `pattern` for a definition readme,
  `.` for the crate-level `docs/readme.md` itself.

  `optional` is True only for the crate-level call. Every definition readme
  publishes a `### Regenerate` block by house style, so a missing file or a
  missing block is always a defect there. The crate-level readme is a
  free-form Module Index — some crates give it a demo recipe, most do not, and
  a still-scaffolded crate may have no crate-level readme.md at all — so
  neither absence is a G15 defect for it. Only a block that is actually
  present and exits non-zero is one.
  """
  if not readme.exists():
    if not optional:
      problems.append( f'README {rel}/  no readme.md' )
    return
  found = blocks( readme )
  if not found:
    if not optional:
      problems.append( f'REGEN  {rel}/readme.md  no ### Regenerate block' )
    return
  for ( ln, cmd, quoted ) in found:
    if cmd is None:
      problems.append( f'PROBE  {rel}/readme.md:{ln}  probe recording has no runnable recipe — cannot verify freshness' )
      continue
    ( code, got ) = run( cmd )
    if code != 0:
      problems.append( f'REGEN  {rel}/readme.md:{ln}  exit {code}' )
      for line in got.splitlines()[ :4 ]:
        problems.append( f'         {line}' )
    elif quoted is not None and got != quoted.rstrip( '\n' ):
      problems.append( f'STALE  {rel}/readme.md:{ln}  quoted output is not what it prints now' )
      for line in _diff( quoted.rstrip( '\n' ), got ):
        problems.append( f'         {line}' )


def check( docs ):
  problems = []
  docs = pathlib.Path( docs )

  for ( _, _, p ) in lib.instances( docs ):
    rel = p.relative_to( docs ).as_posix()
    for ( ln, cmd, quoted ) in blocks( p ):
      if cmd is None:
        problems.append( f'PROBE  {rel}:{ln}  probe recording has no runnable recipe — cannot verify freshness' )
        continue
      ( code, got ) = run( cmd )
      if quoted is None:
        problems.append( f'UNQUOTED {rel}:{ln}  recipe publishes no expected output' )
        continue
      if got != quoted.rstrip( '\n' ):
        problems.append( f'STALE  {rel}:{ln}  quoted output is not what it prints now' )
        for line in _diff( quoted.rstrip( '\n' ), got ):
          problems.append( f'         {line}' )
      elif code != 0 and not got:
        problems.append( f'EXIT   {rel}:{ln}  exit {code} with no output' )

  for d in sorted( x for x in docs.iterdir() if x.is_dir() ):
    _check_readme( d / 'readme.md', d.name, problems )
    if d.name == 'item':
      for kind in sorted( x for x in d.iterdir() if x.is_dir() ):
        _check_readme( kind / 'readme.md', f'{d.name}/{kind.name}', problems )

  # The crate-level readme itself, one level up from every definition readme
  # above. It was previously never opened at all, since `lib.instances` requires
  # a directory and the loop above walks only `docs`'s own subdirectories.
  _check_readme( docs / 'readme.md', '.', problems, optional = True )

  return problems


def _diff( want, got ):
  """The first few lines where two outputs disagree, quoted-then-actual."""
  w = want.splitlines()
  g = got.splitlines()
  out = []
  for i in range( max( len( w ), len( g ) ) ):
    a = w[ i ] if i < len( w ) else '<absent>'
    b = g[ i ] if i < len( g ) else '<absent>'
    if a != b:
      out.append( f'quoted |{a}' )
      out.append( f'actual |{b}' )
      if len( out ) >= 6:
        out.append( '…' )
        break
  return out


def main( argv ):
  bad = 0
  for docs in argv[ 1 : ]:
    bad += lib.report( lib.crate_name( docs ), check( docs ) )
  print( f'{bad} problem(s)' )
  return 1 if bad else 0


if __name__ == '__main__':
  sys.exit( main( sys.argv ) )
