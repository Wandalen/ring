# Integration: Four Edges In, Two Written Down

### Scope

**Purpose:** Record what `ring_batch` depends on, what its two prose documents
say it depends on, and the fact that the two answers differ from each other and
from the manifest.

**Responsibility:** The four incoming dependency edges, the `Depends on` sentence,
the readme's dependency line, and the citation path both cite.

**In Scope:** `ring_batch/Cargo.toml`; `ring_batch/src/lib.rs:3-5`,
`:35-38`, `:361`; `ring_batch/readme.md:5`.

**Out of Scope:** The single outgoing edge and what the design corpus says about
it are [`integration/002`](002_the_feature_is_planned_its_problems_are_addressed.md).
The storage dependency the crate deliberately declines is
[`data_structure/001`](../data_structure/001_sixteen_bytes_that_are_not_a_buffer.md)
BA10.

---

## Four Answers to One Question

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the manifest --'
command grep -E '^ring_' ring_batch/Cargo.toml | sed 's/ = .*//' | tr '\n' ' '; echo
echo '  -- what the crate actually imports --'
command grep -m1 -A3 -F 'use ring_atomic::SeqCell;' ring_batch/src/lib.rs
echo '  -- what the module comment said before BA17'"'"'s fix (expect 0 hits now -- see BA17 below) --'
printf '    hits: %s\n' "$( command grep -c -F '//! Depends on `ring_types`, `ring_seqno` and `ring_atomic`.' ring_batch/src/lib.rs )"
echo '  -- what the readme says --'
command grep -m1 -F 'Depends on [`ring_types`](../ring_types/readme.md), [`ring_seqno`](../ring_seqno/readme.md).' ring_batch/readme.md
echo '  -- the spec path the module comment cites --'
command grep -m1 -F '//! write-path family specified in `docs/workstream/008_ring_write_path.md`.' ring_batch/src/lib.rs
for p in docs/workstream/008_ring_write_path.md docs/workstream/008_ring_write_path
do
  if [ -e "$p" ]; then echo "  exists  : $p"; else echo "  missing : $p"; fi
done
```

Live output:

```
  -- the manifest --
ring_types ring_seqno ring_atomic ring_index 
  -- what the crate actually imports --
use ring_atomic::SeqCell;
use ring_index::of;
use ring_seqno::free_slots;
use ring_types::{ Capacity, RingError, Seq, SlotIndex };
  -- what the module comment said before BA17's fix (expect 0 hits now -- see BA17 below) --
    hits: 0
  -- what the readme says --
Depends on [`ring_types`](../ring_types/readme.md), [`ring_seqno`](../ring_seqno/readme.md).
  -- the spec path the module comment cites --
  missing : docs/workstream/008_ring_write_path.md
  exists  : docs/workstream/008_ring_write_path
```

Four declared, four imported, three named in the source, two named in the readme.

---

### BA17 — Two Prose Statements of the Dependency List, Neither Matching the Manifest

The manifest declares four and the code imports all four — one `use` each, no
dead dependency. The two documents that restate the list for a reader both stop
short: the module comment names three, the readme names two, and `ring_index`
appears in neither.

This is not a `ring_batch` quirk. Twenty-nine of the 33 crates carry an explicit
`Depends on` sentence, and eleven of them understate it:

```sh
cd "$(git rev-parse --show-toplevel)"
for c in $( ls ring | sort )
do
  f="ring/$c/src/lib.rs"
  [ -f "$f" ] || continue
  line=$( command grep -m1 '^//! Depends on' "$f" || true )
  [ -n "$line" ] || continue
  printf '%s' "$line" | command grep -oE 'ring_[a-z_]+' | sort -u > /tmp/-named.$$
  command grep -oE '^ring_[a-z_]+' "ring/$c/Cargo.toml" | sort -u > /tmp/-real.$$
  miss=$( comm -13 /tmp/-named.$$ /tmp/-real.$$ | tr '\n' ' ' )
  rm -f /tmp/-named.$$ /tmp/-real.$$
  [ -n "$( printf '%s' "$miss" | tr -d ' ' )" ] && printf '%-14s omits: %s\n' "$c" "$miss"
done
```

Live output:

```
ring_barrier   omits: ring_gating 
ring_event     omits: ring_store 
ring_flush     omits: ring_config ring_types 
ring_handle    omits: ring_config ring_types 
ring_mpsc      omits: ring_cursor ring_gating ring_slot ring_types 
ring_poll      omits: ring_config ring_types 
ring_publish   omits: ring_barrier ring_claim ring_consume ring_gating 
ring_shutdown  omits: ring_config 
ring_spsc      omits: ring_types 
ring_tls       omits: ring_store ring_event ring_slot 
```

**Finding.** The sentence is written once, when the crate is first sketched, and
never revisited when a dependency is added — so the omission is always in the
same direction. `ring_mpsc` and `ring_publish` each omit four, and every crate on
the list omits at least one. A reader who trusts the sentence over the manifest
will believe the family's dependency forest is thinner than it is, and the two
crates where that error is largest are the two that assemble the rings.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep 'Depends on' ring_batch/src/lib.rs
```

Live output:

```
//! Depends on `ring_types`, `ring_seqno`, `ring_atomic` and `ring_index`.
```

**Disposition:** applied — `ring_batch`'s own module comment now names all four
manifest edges instead of three; the family-wide census above stays as a
recorded observation about the other ten crates, which are out of this
instance's scope.
Now prints: `ring_index`

---

### BA18 — Thirty-Three Source Files Cite a Path That Is Not There

Every one of the 33 crates cites its family's own design corpus with a path
ending in `.md`, naming a file that does not exist. The real target is a
same-named directory one level up, and every
readme in the family gets it right:

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo "  src/lib.rs citing the flat .md   : $( command grep -rl '008_ring_write_path\.md' --include=lib.rs ring_*/ | wc -l ) of 33"
echo "  src/lib.rs citing the directory  : $( command grep -rl '008_ring_write_path/' --include=lib.rs ring_*/ | wc -l ) of 33"
echo "  readme.md citing the flat .md    : $( command grep -l '008_ring_write_path\.md' ring_*/readme.md 2>/dev/null | wc -l ) of 33"
echo "  readme.md citing the directory   : $( command grep -l '008_ring_write_path/' ring_*/readme.md 2>/dev/null | wc -l ) of 33"
echo "  and three crates outside the family cite it too:"
# `bench_harness`, `exchange_core` and `smoke_ring_write_path` cite it from
# outside the family, so this scan checks every crate's own readme rather
# than just the family's.
command grep -l '008_ring_write_path/' */readme.md 2>/dev/null | sed 's|/readme.md||' | command grep -v '^ring_' | LC_ALL=C sort | sed 's/^/    /'
```

Live output:

```
  src/lib.rs citing the flat .md   : 0 of 33
  src/lib.rs citing the directory  : 33 of 33
  readme.md citing the flat .md    : 0 of 33
  readme.md citing the directory   : 0 of 33
  and three crates outside the family cite it too:
    smoke_ring_write_path
```

**Finding.** When this was recorded the split was clean and 33–0 the other way:
every source file was wrong and every readme was right, in the same crates,
about the same document. The cited document was a flat file when the module comments were
written and became a directory before the readmes were; nothing re-read the
comments.

The citation is not a link and no tool checks it — `rustdoc` treats it as inline
code, not a path — so the reference dangled in every crate in the family from
the split until the family-wide correction recorded under `ring_cursor`'s CU48,
which the census above now reads back as 33–33.

**Correction (2026-09-28):** the census above no longer reads 33–33. Readme
standardization removed this citation from all 33 `ring_*` readmes entirely —
none now mention `008_ring_write_path` in either form (`readme.md citing the
directory` reads 0 of 33), each pointing instead to the family overview at
`ring/readme.md`. This is not a regression of the
wrong-path kind this finding tracks: the readmes are not citing a stale form,
they no longer cite that path at all. The other two outside crates
changed too — `bench_harness/readme.md` was standardized the same way,
and `exchange_core` relocated to a different root entirely
(its new readme also omits the citation) —
leaving `smoke_ring_write_path` as the only outside crate the census still
finds. The `src/lib.rs` half is unaffected: all 33 still cite the directory
correctly, including `ring_batch`'s own, per the Disposition below.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep 'write-path family specified in' ring_batch/src/lib.rs
```

Live output:

```
//! write-path family specified in `docs/workstream/008_ring_write_path/readme.md`.
```

**Disposition:** applied — `ring_batch`'s own module comment now cites the
directory path the readme already had right, closing the one file this
instance's own In Scope names; the 33-crate census stays as a recorded
observation about the rest of the family, which is out of this instance's
scope.
Now prints: `008_ring_write_path/`

---

### BA19 — The Undeclared Dependency Is the One Serving the Function Nothing Calls

`ring_index` is the edge neither document mentions, and it has exactly one use
site:

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- every call of the imported symbol --'
command grep 'of( seq' ring_batch/src/lib.rs | command grep -v '///'
echo '  -- the function it sits in --'
command grep -m1 -A4 -F 'pub fn drain_order( claim : &BatchClaim, capacity : Capacity )' ring_batch/src/lib.rs
echo '  -- what the rest of the family calls --'
command grep -r 'ring_batch::' --include=*.rs . | command grep -v '^ring_batch/'
```

Live output:

```
  -- every call of the imported symbol --
  claim.sequences().map( move | seq | ( seq, of( seq, capacity ) ) )
  -- the function it sits in --
pub fn drain_order( claim : &BatchClaim, capacity : Capacity )
-> impl Iterator< Item = ( Seq, SlotIndex ) > + use< >
{
  claim.sequences().map( move | seq | ( seq, of( seq, capacity ) ) )
}
  -- what the rest of the family calls --
ring_tls/tests/tls_test.rs:use ring_batch::BatchClaim;
ring_tls/src/lib.rs:use ring_batch::{ claim, BatchClaim };
ring_cursor/src/lib.rs://! rather than take a parameter — the same choice `ring_batch::claim_gated`
```

**Finding.** The fourth dependency exists for `drain_order`, and `drain_order` is
one of the two public functions no crate in the family calls — the only outside
references are `BatchClaim`, `claim`, and a prose mention in `ring_cursor`'s
module comment.

So the edge the two prose documents both omit is the one carrying the least
traffic, and it is carried entirely by a function with no caller. That is not a
coincidence in the reader's favour: the sentence was written from what the crate
was *for*, and `drain_order` is the part of it nothing has needed yet. The
omission tracks use, so the least-used dependency is the one most likely to be
missing from the description — which is exactly backwards from what a reader
needs, because a dependency they can see exercised is one they could have
inferred anyway.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`integration/002`](002_the_feature_is_planned_its_problems_are_addressed.md) | The dependents rather than the dependencies, and the feature statuses that describe them |
| [`api/002`](../api/002_two_claim_functions_one_caller.md) | `drain_order` among the public items nothing calls |
| [`item/002`](../item/002_one_past_the_end.md) | What `drain_order` computes, through the edge this entry is about |
| [`non_functional_requirement/002`](../non_functional_requirement/002_sixteen_bytes_and_no_allocation.md) | What the four edges buy, measured rather than listed |

### Sources

| Fact | Where |
|------|-------|
| The four manifest edges | `ring_batch/Cargo.toml` |
| The three-name sentence in the source | `ring_batch/src/lib.rs:5` |
| The two-name sentence in the readme | `ring_batch/readme.md:5` |
| The flat `.md` citation, and the readme's directory citation | `ring_batch/src/lib.rs:4`; `ring_batch/readme.md:10` |
| The fourth edge's import and its one call site | `ring_batch/src/lib.rs:36`, `:361` |
| Eleven understating crates, 33-0 on the citation split | Censuses above |

### Tests

| Test | Covers |
|------|--------|
| `drain_order_folds_through_ring_index_and_not_a_second_implementation` | That the undeclared edge is real and load-bearing — the only assertion any of these three findings has |
| `a_batch_drain_reads_in_issue_order` | `drain_order`'s output, which is what the edge exists to produce |
| *(to create)* | Nothing compares a `Depends on` sentence against its own manifest, in either direction, in any crate |
| *(to create)* | Nothing checks that a cited workstream path resolves, which is why 33 source files have cited a missing one since the split |
