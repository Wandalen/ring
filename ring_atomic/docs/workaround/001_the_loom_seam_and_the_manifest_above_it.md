# Workaround: The Loom Seam and the Manifest Above It

### Scope

**Purpose:** Record the external constraint `ring_atomic` absorbs so that five other
crates can run model checks, what that costs inside the crate, and the one part of
the cost it cannot absorb and pushes upward instead.

**Responsibility:** The `cfg( loom )` import switch, the four duplicated constructors
and one hand-written `Default` it forces, and the `check-cfg` entry in the root
manifest.

**In Scope:** `ring_atomic/src/lib.rs:63-67`, `:168-180`, `:195-209`,
`:349-376`; `Cargo.toml:235-240`.

**Out of Scope:** The creation-site pattern the seam depends on is
[`pattern/001`](../pattern/001_one_place_where_an_atomic_is_created.md), where the
"no other crate needs to know" claim is examined as a doc defect rather than as a
build cost. The `const` the split exists to preserve is unused —
[`item/001`](../item/001_six_constructors_for_two_types.md) AT26.

---

## The Seam, What It Forces, and What It Cannot Contain

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the seam itself --'
command grep -m1 -B2 -A2 -F 'use loom::sync::atomic::{ AtomicU64, AtomicUsize };' ring_atomic/src/lib.rs
echo '  -- every site it forces in this crate --'
command grep 'cfg( loom )\|cfg( not( loom ) )' ring_atomic/src/lib.rs
echo '  -- the entry that legalises the name, and where it has to live --'
# anchored on the entry's own comment, not a line number: the members list
# above it grows with every crate added, and a fixed offset then quotes
# whichever member comment moved into its place
sed -n '/^# .loom. is set by RUSTFLAGS/,/^unexpected_cfgs/p' Cargo.toml
echo '  -- against every crate that actually reads the cfg --'
command grep -rl 'cfg( loom )' --include=*.rs . | sed 's|ring/||' | sort
```

Live output:

```
  -- the seam itself --

#[ cfg( loom ) ]
use loom::sync::atomic::{ AtomicU64, AtomicUsize };
#[ cfg( not( loom ) ) ]
use core::sync::atomic::{ AtomicU64, AtomicUsize };
  -- every site it forces in this crate --
#[ cfg( loom ) ]
#[ cfg( not( loom ) ) ]
  #[ cfg( not( loom ) ) ]
  #[ cfg( loom ) ]
  #[ cfg( not( loom ) ) ]
  #[ cfg( loom ) ]
  -- the entry that legalises the name, and where it has to live --
# `loom` is set by RUSTFLAGS, not by any feature, so rustc has no other way to
# learn it is a real cfg. Declared once here rather than per crate: the lints
# table is inherited workspace-wide, and a crate cannot both inherit it and add
# its own. Only ring_atomic, ring_cursor and ring_publish read the cfg — see
# ring_atomic's module documentation on the seam.
unexpected_cfgs = { level = "warn", check-cfg = [ 'cfg(loom)' ] }
  -- against every crate that actually reads the cfg --
ring_atomic/src/lib.rs
ring_cursor/src/lib.rs
ring_mpsc/tests/mpsc_test.rs
ring_publish/tests/handshake_test.rs
ring_publish/tests/publish_test.rs
ring_spsc/tests/spsc_test.rs
ring_testkit/tests/exhaustive_test.rs
ring_testkit/tests/testkit_test.rs
```

---

### AT49 — The Crate Absorbs Loom's API Faithfully and Cannot Absorb Its Build Requirement

The constraint is real and external: `loom`'s instrumented atomics carry per-execution
model state, so they have no `const` constructor and their trait implementations are
loom's business rather than this crate's. `ring_atomic` absorbs both facts properly.
The import switch at `:64-67` selects the atomic source; `new` is written twice for
each of the two cells, `const` in an ordinary build and not under `--cfg loom`; and
`Default` is written out by hand rather than derived, with the reason stated — "so
that it does not depend on whichever `AtomicU64` is in scope having its own
`Default`".

That is six `cfg` sites, two duplicated constructors, and one hand-written impl, in
exchange for five other crates being able to run loom models against this crate's own
cells rather than against a re-implementation. The trade is clearly worth it.

One part of the cost cannot be absorbed here. `loom` is set by `RUSTFLAGS`, never by a
feature, so rustc has no way to learn the name is legitimate without a `check-cfg`
entry — and the manifest comment names exactly why that entry cannot live in this
crate: "the lints table is inherited workspace-wide, and a crate cannot both inherit
it and add its own." So it sits in the root manifest, shared by all 33 crates.

**And the comment beside that entry is now wrong.** It reads "Only ring_atomic,
ring_cursor and ring_publish read the cfg"; the scan two lines below it returns
six — `ring_atomic` and `ring_cursor` in `src/`, and `ring_mpsc`, `ring_publish`,
`ring_spsc` and `ring_testkit` in `tests/`. The comment was accurate when the
seam had two source readers and one test reader, and three crates grew loom
models afterwards without anyone revisiting the sentence that counted them.

The recipe above has printed both halves side by side since it was written, which
is the point worth extracting: the contradiction was visible in this instance's
own quoted output before it was visible to anyone reading the manifest, because
the block asks for the claim and the measurement together rather than trusting
either alone. What it did not do was *compare* them — that took a re-run under
the Regression Gate, and the comparison is what this paragraph is.

**Finding.** The consequence is that `ring_atomic` does not build standalone under
`-D warnings`. Lifted out of the workspace with its own manifest and no lints table:

```
error: unexpected `cfg` condition name: `loom`
  --> src/lib.rs:58:9
error: unexpected `cfg` condition name: `loom`
  --> src/lib.rs:60:14
error: unexpected `cfg` condition name: `loom`
   --> src/lib.rs:147:16
error: unexpected `cfg` condition name: `loom`
   --> src/lib.rs:155:11
error: unexpected `cfg` condition name: `loom`
   --> src/lib.rs:268:16
error: unexpected `cfg` condition name: `loom`
   --> src/lib.rs:283:11
error: could not compile `ring_atomic` (lib) due to 6 previous errors
```

One error per `cfg` site. This is what the module documentation's "no other crate
needs to know the seam exists"
([`pattern/001`](../pattern/001_one_place_where_an_atomic_is_created.md) AT38) is
false about: the seam is invisible to siblings and unmissable to whatever builds them.
The fix is four lines of `[lints.rust]` in the crate's own manifest — at the price of
giving up the inherited workspace table, which is precisely the trade the root comment
declined.

**Deletion condition:** when `loom` provides `const` constructors, the constructor
split collapses to one function each and only the import switch remains. The
`check-cfg` entry survives until either loom is dropped or Cargo permits a crate to
extend an inherited lints table.

---

### AT50 — The Comment Explaining the Workaround Names Half the Crates It Serves

The root manifest's comment is doing real work: it explains why the entry is there,
why it is at the root and not per-crate, and where to read more. It closes by naming
its beneficiaries — "Only ring_atomic, ring_cursor and ring_publish read the cfg".

Eight files read it, across six crates: `ring_atomic` and `ring_cursor` in `src/`, and
`ring_mpsc`, `ring_publish`, `ring_spsc` and `ring_testkit` in `tests/`, with
`ring_publish` and `ring_testkit` carrying two files each. Five crates declare `loom`
as a dependency in their own manifests. The comment names three.

**Finding.** This is the fourth instance of one habit in this crate's neighbourhood: a
count written correctly at the time and never recomputed. It joins `ring_atomic`'s own
"one place in 33 crates"
([`pattern/001`](../pattern/001_one_place_where_an_atomic_is_created.md) AT37),
`ring_cursor`'s "six manifests" for a re-export used by eight, and the module doc
naming `handshake_test.rs` as the seam's only user.

The distinguishing feature of this one is the word "Only". A reader trusting it would
conclude that removing `ring_publish`'s loom test frees the entry, when in fact three
other crates would break. The entry is load-bearing for twice as many crates as its
own comment admits, and the sentence that would be true — *read by six crates; see
`ring_atomic`'s module documentation on the seam* — is shorter than the one written.

Every one of these is cheap to prevent: the counts are all one `grep` away, and none
of the four has a test or check that would notice the drift.

**Disposition:** declined — the stale "Only ring_atomic, ring_cursor and
ring_publish" count lives in the workspace root `Cargo.toml:235-240`'s shared
comment, not this crate's own doc; correcting shared root-manifest state
belongs to its own dedicated, freshness-checked pass across all six affected
crates rather than a single crate's corpus disposition pass.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`workaround/002`](002_what_the_seam_does_not_switch.md) | The seam's two edges — the name that crosses unswitched, and the suite that cannot run |
| [`pattern/001`](../pattern/001_one_place_where_an_atomic_is_created.md) | The creation-site pattern the seam depends on, and the claim this disproves |
| [`item/001`](../item/001_six_constructors_for_two_types.md) | The four constructors the split produces, and the `const` nobody uses |
| [`integration/001`](../integration/001_two_declared_one_used.md) | The crate's declared dependencies, including one it does not use |

### Sources

| Fact | Where |
|------|-------|
| The import switch | `ring_atomic/src/lib.rs:63-67` |
| The hand-written `Default` and its reason | `ring_atomic/src/lib.rs:168-180` |
| The two constructor splits | `ring_atomic/src/lib.rs:195-209`, `:349-376` |
| The `check-cfg` entry, its rationale, and its count | `Cargo.toml:235-240` |
| Eight files across six crates reading the cfg | Census above |
| Six errors when built outside the workspace | `RUSTFLAGS="-D warnings" cargo build` on a lifted copy, quoted above |

### Tests

| Test | Covers |
|------|--------|
| *(to create)* | Nothing builds this crate standalone, so the six-error failure is invisible to the suite |
| *(to create)* | No test runs under `--cfg loom` in this crate, so the seam's own branch is exercised only by four other crates' test files |
