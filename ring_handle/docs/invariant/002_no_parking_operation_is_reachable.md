# Invariant: Nothing Reachable From a Handle Can Park

### Scope

- **Purpose**: State the second restriction this crate's surface carries — that no operation reachable from a handle may block, park, or await — and mark precisely where the restriction is this crate's and where it is only imposed on it.
- **Responsibility**: The statement, its enforcement, and the consequences of each way through it.
- **In Scope**: The non-parking property of the handle surface; the boundary between the tick path and everything else.
- **Out of Scope**: Which handle may do what (→ [Capability Follows the Handle](001_capability_follows_the_handle.md)); the wait strategies themselves, which are [`ring_wait`](../../../ring_wait/readme.md)'s; the poll loop, which is [`ring_poll`](../../../ring_poll/readme.md)'s.

### Invariant Statement

> No operation reachable from a handle can park. Where refusal must be
> representable — `try_push`, `try_recv` — the method returns a `Result` or
> an `Option`; the rest of the surface returns whatever shape fits it, and
> that shape is not what enforces the restriction (→ HD52). Blocking variants
> continue to exist for threads outside the tick; they are not reachable from
> the values a system holds.

This is the non-parking design
applied to this crate's surface, and its phrasing of the mechanism is the whole
point: "the restriction is enforced by what is exposed, not by a rule in a
document."

**This crate is named alongside `ring_poll` in the shared row and does not claim it.**
[The acceptance table](../../../bench_harness/docs/acceptance/001_feature_reached_tests.md)
lists this row's owning crates as `ring_poll` and `ring_handle`, and its claiming
test as `ring_poll/tests/poll_test.rs` — the mechanism belongs to `ring_poll`,
**constraining** `ring_handle`. So:

| | Owns the mechanism | Claims the feature | Must satisfy the constraint |
|---|---|---|---|
| `ring_poll` | Yes — the loop and its bound | Yes | — |
| `ring_handle` | No | **No** | **Yes** — this instance |

**A constraint a crate must satisfy but does not claim is the easiest kind to
lose**, because nothing in this crate's own test run goes red when it is
violated. That is the reason this instance exists at all rather than being a
cross-reference in `ring_poll`'s docs.

### Enforcement Mechanism

**By absence, again** — the same mechanism as
[Capability Follows the Handle](001_capability_follows_the_handle.md), applied
along a second axis. The handle surface offers `try_`-shaped operations; the
parking variants are not on it.

| Mechanism | Enforces | Detected when |
|-----------|----------|---------------|
| Every handle method returns `Result`/`Option` | That refusal is representable, so parking is never the only way to answer | Compile time |
| No `push`/`recv`-shaped blocking method exists on either handle | That parking is unreachable from a system | Compile time |
| No method takes a `Duration`, a `Condvar`, or a waker | That parking is unreachable *indirectly* | Compile time, if anyone looks |
| `ring_poll::PARKING_CRATES`, asserted against the manifests on disk | That rows 1–3 stay true, and cannot be made false by a dependency edit | `ring_poll`'s test run — **not this crate's** |
| A bounded-time test on a full ring | That "returns a `Result`" is not satisfied by a `Result` returned after a spin | `ring_poll`'s test run |

**Rows 4 and 5 sit in another crate. That was written up as this invariant's
structural weakness, and the mechanism `ring_poll` actually built turns it into
the opposite.** The concern was that a compile-fail case can only target one
surface: this crate's own compile-fail cases live here and go red when *this*
crate's surface breaks, `ring_poll`'s live there and go red when *its* surface breaks,
so a blocking method added to `Producer` would fall between them.

`ring_poll` did not write a compile-fail case. It made the roster of crates
permitted to park a public constant and asserted it against every sibling
`Cargo.toml` — so what is guarded is **the dependency graph, not a surface**.
A blocking method cannot be added to `Producer` without `ring_handle` depending
on `ring_wait`, and that dependency fails `ring_poll`'s suite whatever the
method is called and whichever crate the method lives in. The guard fires on the
edit that *enables* the violation rather than on the violation itself, which is
why it covers both surfaces at once instead of neither.

**This is not a free win, and the residual gap has a name.** A parking operation
that reaches this crate without a manifest edge — `std::thread::sleep` written
inline in a forwarding method — is invisible to it. Row 3's "if anyone looks" is
still the only thing standing there. See
[`ring_poll`'s account of how its own guard degrades](../../../ring_poll/docs/pattern/001_enforcement_by_dependency_graph.md).

The original framing of the ambiguity, kept because the reasoning is what
generalises: which surface a case targets is not settled by the acceptance
table's wording — "reachable from the system-facing handle" suggests this
crate's surface, "`ring_poll/tests/poll_test.rs`" places
the test elsewhere — and that ambiguity is worth naming rather than assuming
away.

**Row 3 is the one that gets missed.** Absence of a *blocking-shaped* method is
easy to check; absence of a *parameter that implies waiting* is not. A
`try_publish_timeout( d: Duration )` is `try_`-named, returns a `Result`, and
parks for up to `d`. It satisfies rows 1 and 2 and violates the invariant
completely.

### Violation Consequences

| # | Violation | Immediate effect | Real consequence |
|---|-----------|------------------|------------------|
| W1 | A blocking `push` is added to `Producer` | Compiles; every test passes | **Deadlock inside a tick**, exactly when: "It compiles, it works under light load, and it deadlocks the first time the ring is genuinely full during a tick — which is to say, under exactly the conditions the system was written for" |
| W2 | A `try_`-named method takes a `Duration` and parks | Compiles; passes a naming audit | Same deadlock, arriving later and looking innocent — the row-3 gap |
| W3 | A handle method spins until space is available | Compiles; returns a `Result` as required | **Not a deadlock — worse in one way.** The tick does not hang; it silently loses its frame budget under load, and nothing distinguishes that from slow work |
| W4 | A handle exposes the backend, whose blocking API is then reachable | Compiles | The restriction becomes advisory — the same V4 shape as the capability split, along a second axis |
| W5 | A handle method is made `async` | Compiles | The tick step stops being a plain function and needs an executor to complete |

**W3 is the violation a `Result`-returning signature does nothing to prevent**,
and it is why the acceptance table pairs the compile-fail case with a
*bounded-time test on a full ring*. A signature says what a method returns; only
a clock says when.

**W1's severity is not proportional to its difficulty.** It is
"one autocomplete away," which is a claim about how the defect arrives:
not from a design mistake, but from a plausible-looking completion in an editor,
in a crate whose whole surface is small enough to look obviously correct.

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_producer_surface.md](../api/001_producer_surface.md) | The surface W1 and W2 would be added to |
| [../api/002_consumer_surface.md](../api/002_consumer_surface.md) | Same, for the draining end |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_delegating_to_the_backend.md](../algorithm/002_delegating_to_the_backend.md) | Why delegation must not widen the surface — W4's mechanism |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_one_dependency_and_the_backends_beneath.md](../integration/001_one_dependency_and_the_backends_beneath.md) | The `ring_core` seam whose surface W4 would leak, and the `ring_poll` seam rows 4–5 sit across |

### Invariants

| File | Relationship |
|------|--------------|
| [001_capability_follows_the_handle.md](001_capability_follows_the_handle.md) | The first restriction; same enforcement mechanism, different axis, and its V4 is this one's W4 |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_enforce_by_withholding.md](../pattern/001_enforce_by_withholding.md) | The general practice both invariants are instances of |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_a_convenience_method_undoes_the_crate.md](../pitfall/001_a_convenience_method_undoes_the_crate.md) | W1 and W2 as the edits that actually get made |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_poll/docs/invariant/001_no_parking_operation_on_the_tick_path.md`](../../../ring_poll/docs/invariant/001_no_parking_operation_on_the_tick_path.md) | `ring_poll`'s own statement of this restriction, the mechanism, and W1's failure description |
| [`ring_poll/docs/pattern/001_enforcement_by_dependency_graph.md`](../../../ring_poll/docs/pattern/001_enforcement_by_dependency_graph.md) | How the guard actually enforces this, and where it degrades |
| [`bench_harness/docs/acceptance/001_feature_reached_tests.md`](../../../bench_harness/docs/acceptance/001_feature_reached_tests.md) | This crate named alongside `ring_poll`, constraining `ring_handle` |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/handle_test.rs`](../../tests/handle_test.rs) · `no_parking_shaped_name_appears_in_the_source` | The shape half, and **not** in the form this row originally specified. "Every method returns `Result` or `Option`" is false of the surface that was built — `free_capacity` returns `usize`, `is_full` returns `bool` — and it was the wrong shape to ask for anyway: a method returning `Result` can still block before returning it. The test scans `src/lib.rs` for parking-shaped names instead, which is what the invariant is actually about. Its own limits are in [H5](../../tests/manual/readme.md) |
| [`ring_poll/tests/poll_test.rs`](../../../ring_poll/readme.md) | `ring_poll`'s claiming test — the compile-fail case and the bounded-time run on a full ring, which are W1/W2/W3's only detectors. Written and passing; `ring_poll::PARKING_CRATES` is the part of it that reaches back here |

### HD51 — The Graph Guard Reads Every Sibling Manifest and Recognises Exactly One of the Three Names on Its Own Roster

Rows 4 and 5 put this invariant's real enforcement in `ring_poll`, and the
account above is right about the shape: the scan does walk every `ring_*`
manifest on disk, not a hardcoded list. What it looks for in each one is a
single substring:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the roster, and the single substring the scan looks for --'
command grep 'PARKING_CRATES : ' ring_poll/src/lib.rs | sed 's|^|    |'
command grep 'contains( "ring_wait" )' ring_poll/tests/poll_test.rs | sed 's|^|    |'
echo '  -- so membership is decided by one name; the other two qualify because --'
for c in ring_barrier ring_shutdown ring_wait; do
  printf '    %-15s manifest names ring_wait: %s\n' "$c" \
    "$( command grep -c 'ring_wait' ring/$c/Cargo.toml )"
done
echo '  -- an edge to a rostered crate that is not ring_wait is invisible --'
printf '    ring_handle manifest names ring_shutdown: %s\n' \
  "$( command grep -c 'ring_shutdown' ring_handle/Cargo.toml )"
printf '    ring_handle manifest names ring_wait:     %s\n' \
  "$( command grep -c 'ring_wait' ring_handle/Cargo.toml )"
```

Live output:

```
  -- the roster, and the single substring the scan looks for --
    pub const PARKING_CRATES : [ &str; 3 ] = [ "ring_barrier", "ring_shutdown", "ring_wait" ];
        if fs::read_to_string( &manifest ).unwrap().contains( "ring_wait" )
  -- so membership is decided by one name; the other two qualify because --
    ring_barrier    manifest names ring_wait: 1
    ring_shutdown   manifest names ring_wait: 1
    ring_wait       manifest names ring_wait: 1
  -- an edge to a rostered crate that is not ring_wait is invisible --
    ring_handle manifest names ring_shutdown: 0
    ring_handle manifest names ring_wait:     0
```

`PARKING_CRATES` names three crates. The scan classifies a crate as parking if
its manifest text contains `ring_wait` — so `ring_barrier` and `ring_shutdown`
appear in the measured set not because they are recognised as parking crates
but because each depends on `ring_wait`, and `ring_wait` appears because its own
`[package]` stanza spells its name. The assertion then compares that set to the
roster and passes.

**The consequence is a shape the enforcement paragraph above does not cover.**
The claim is that a blocking method "cannot be added to `Producer` without
`ring_handle` depending on `ring_wait`." A blocking method can equally be added
by depending on `ring_shutdown`, which is on the parking roster and whose
`close`/`Stopped` surface is the family's other waiting-shaped API. That edge
adds `ring_shutdown` to `ring_handle`'s manifest and no `ring_wait`, the scan
records nothing, and the roster still equals the measured set — green suite,
parking one hop away.

The guard fires on the edit that enables the violation, as the paragraph says,
for exactly one of the three edits that enable it. Widening it is a one-word
change — match against each name in `PARKING_CRATES` rather than the literal —
and it is `ring_poll`'s to make, not this crate's, which is the structural
weakness rows 4 and 5 were originally written to name and this instance now has
a measurement for.

**Disposition:** declined — the fix this finding names is a one-word change to
`ring_poll/src/lib.rs`'s substring match, a different crate outside
this pass's assigned scope (`ring_gating`, `ring_handle`, `ring_index`); the
finding's own text is explicit that "it is `ring_poll`'s to make, not this
crate's." Nothing in `ring_handle`'s own `src/` or `tests/` can widen a scan
that lives and runs entirely in `ring_poll`'s test suite.

### HD52 — The Invariant Statement Is True of Two of the Twelve Methods on the Surface, and the File's Own Tests Row Says So

The statement at the top of this file opens with "Every method reachable from a
handle returns a `Result` or an `Option`." The surface it describes:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -oE '^  pub (const )?fn [a-z_]+.*' ring_handle/src/lib.rs \
  | sed -E 's/^  pub (const )?fn /    /' | sed -E 's/ *\{$//'
printf '    -- methods: %s · returning Result or Option: %s\n' \
  "$( command grep -cE '^  pub (const )?fn ' ring_handle/src/lib.rs )" \
  "$( command grep -cE '^  pub (const )?fn .*-> (Result|Option)' ring_handle/src/lib.rs )"
echo '  -- and the row that already says so --'
command grep -nE '^\| \[`tests/handle_test' ring_handle/docs/invariant/002_no_parking_operation_is_reachable.md \
  | sed 's|(\.\./[^)]*)||' | sed -E 's/^(.{0,215}).*/\1/' | sed 's|^|    |' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
```

Live output:

```
    new( ring : Ring< T > ) -> Self
    ends( &mut self ) -> Ends< '_, T >
    split( &'a mut self ) -> ( Producer< 'a, T >, Consumer< 'a, T > )
    try_push( &mut self, record : T ) -> Result< (), T >
    try_push_batch( &mut self, records : &mut impl Iterator< Item = T > ) -> usize
    free_capacity( &self ) -> usize
    is_full( &self ) -> bool
    try_recv( &mut self ) -> Option< T >
    try_recv_batch( &mut self, out : &mut Vec< T > ) -> usize
    drain( &mut self ) -> Drain< '_, 'a, T >
    len( &self ) -> usize
    is_empty( &self ) -> bool
    -- methods: 12 · returning Result or Option: 2
  -- and the row that already says so --
    | [`tests/handle_test.rs`] · `no_parking_shaped_name_appears_in_the_source` | The shape half, and **not** in the form this row originally specified. "Every method returns `Result` or `Option`" is false of the su
```

Two of twelve. `free_capacity` and `len` return `usize`, `is_full` and
`is_empty` return `bool`, both batch methods return a `usize` count, and `new`,
`ends`, `split` and `drain` return values. Only `try_push` and `try_recv` match
the statement.

**The file knows.** Its Tests row says the requirement was "**not** in the form
this row originally specified," names `free_capacity` and `is_full` as the
counterexamples, and explains why the shape was wrong to ask for in the first
place — a method returning `Result` can block before returning it. That
correction is at the bottom of the file. The statement it corrects is at the
top, unaltered, formatted as a block quote, and is what
[`invariant/001`](001_capability_follows_the_handle.md) and the two `api/`
instances cite when they refer to this invariant.

**What the invariant actually is survives the correction intact**, which is why
this is a wording defect and not a design one: nothing reachable from a handle
may park. Returning a `Result` was a proxy for it — the observable, greppable
consequence somebody could check — and the proxy was written into the normative
statement in place of the property. The test that exists scans for parking-shaped
names instead, which is closer to the property and, per
[`pitfall/001`](../pitfall/001_a_convenience_method_undoes_the_crate.md)'s HD40,
still cannot see W3.

```sh
cd "$(git rev-parse --show-toplevel)"
# -m1 and no -n: this file is its own subject, so an unbounded match also finds
# this command line and every copy of its own output below, and `-n` re-prefixes
# a fresh line number onto each earlier pass's output — the stacked
# `298:296:294:15:` prefixes this block used to carry
command grep -m1 'that shape is not what enforces the restriction' ring_handle/docs/invariant/002_no_parking_operation_is_reachable.md
```

Live output:

```
> that shape is not what enforces the restriction (→ HD52). Blocking variants
```

**Disposition:** applied — the Invariant Statement block quote at the top of
the file now leads with the actual property (no operation reachable from a
handle can park) and scopes the `Result`/`Option` claim to the two methods
where it is true, instead of asserting it of all twelve, matching the
correction the file's own Tests row already stated.
Now prints: `that shape is not what enforces the restriction`
