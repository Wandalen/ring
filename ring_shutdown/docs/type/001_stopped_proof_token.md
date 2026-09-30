# Type: The `Stopped` Proof Token

### Scope

- **Purpose**: Explain why `close` returns a value at all, what that value makes unspellable, and what it costs.
- **Responsibility**: The type's shape, the two properties it enforces, and the alternatives rejected.
- **In Scope**: `Stopped< 'a >`, `Shutdown::close`, `Stopped::reopen`.
- **Out of Scope**: The pattern in general (→ [`pattern/001`](../pattern/001_proof_token_orders_two_operations.md)); the termination argument it supports (→ [`invariant/002`](../invariant/002_drain_terminates_because_close_preceded_it.md)).

### Definition

```rust
pub struct Stopped< 'a >
{
  shutdown : &'a Shutdown,
}
```

One reference. No state of its own, no `Drop` impl, and zero runtime cost — it
exists entirely for what the type system does with it.

### Validation

**1. A drain cannot precede a close.** `drain_all` and `discard_all` are
methods on `Stopped`, and `Shutdown::close` is its only constructor — so there
is no expression that reaches *this crate's* drain without closing first. It
does not reach past this crate's own surface: `ring_core::Producer::split`
hands a caller a raw `Consumer`, whose `try_recv`/`try_recv_batch` are public
and drain an open ring with no `Stopped` in the program (→ SD46 below).

**2. A drain cannot follow a reopen.** `reopen` takes `self` by value:

```rust
pub fn reopen( self )
```

so the proof is consumed. Code that drains, reopens, then drains again on the
same token does not compile — which matters because the second drain's
termination argument is exactly as invalid as the first's was valid.

```rust
let stopped = shutdown.close();
stopped.drain_all( &mut consumer, &mut out );   // fine
stopped.reopen();
stopped.drain_all( &mut consumer, &mut out );   // use of moved value
```

The lifetime `'a` gives a third property for free: the token cannot outlive the
`Shutdown` it points at.

### Alternatives Rejected

| Alternative | Why not |
|---|---|
| `close( &self )` returning `()`, with free `drain_all( &Shutdown, .. )` | The precondition becomes a comment. A caller who forgets gets a hang at teardown, not a compile error |
| `drain_all` returning `Result< usize, RingError >` when still open | Moves the failure from compile time to run time for no gain, and needs an error variant meaning "you did not close first" that nothing else would use |
| `close` returning `Option< Stopped >`, `None` on an already-closed flag | Breaks idempotence, which teardown needs: an unwind path and a normal path both close, and neither should have to know which is first |
| A `Drop` impl on `Stopped` that reopens | Silent and wrong-way-round — dropping a token would reopen a ring the caller meant to leave closed, and the common case is exactly that |

### Cost

**The token is only as good as the flag it proves something about**, and the
flag stops nothing on its own (→ [`pitfall/001`](../pitfall/001_close_is_advisory_to_an_unguarded_producer.md)).
`Stopped` guarantees the *ordering* of two operations. It does not guarantee
that the first operation achieved anything — a `Stopped` obtained while a raw
producer is publishing is a valid token proving a fact that does not hold.

That is the honest limit of the technique, and it generalizes: a proof token
proves the call happened, never that the call worked.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_shutdown
D=docs/type/001_stopped_proof_token.md
printf 'the token as declared:         %s\n' "$( awk '/pub struct Stopped/{f=1} f{ sub( /^ */, "" ); printf "%s ", $0 } f&&/^\}/{exit}' src/lib.rs )"
printf 'alternatives it rejects:       %s\n' "$( awk '/^### Alternatives Rejected/{f=1} f&&/^### Cost/{exit} f&&/^\|---/{ h=1; next } f&&h&&/^\| /{ n++ } END{ print n+0 }' $D )"
printf 'of those, also filed as open:  %s\n' "$( awk '/^### Alternatives Rejected/{f=1} f&&/^### Cost/{exit} f&&/Option< Stopped >/{ n++ } END{ print n+0 }' $D )"
printf 'ground given in the type doc:  %s\n' "$( awk -F' \\| ' '/^### Alternatives Rejected/{f=1} f&&/^### Cost/{exit} f&&/Option< Stopped >/{ sub( / \|$/, "", $2 ); print $2 }' $D )"
printf 'where it is open instead:      %s\n' "$( command grep -l 'Option< Stopped >' docs/decisions/*.md )"
printf 'its status there:              %s\n' "$( command grep -h '^- \*\*Status:\*\*' docs/decisions/002_should_a_stopped_token_be_unique.md )"
printf 'ground given in the decision:  %s\n' "$( awk -F' \\| ' '/Option< Stopped >/{ sub( / \|$/, "", $3 ); print $3 }' docs/decisions/002_should_a_stopped_token_be_unique.md )"
printf "this doc's body citing it:     %s\n" "$( awk '/^### Regenerate/{ exit } { print }' $D | command grep -c 'decisions/002' || true )"
printf 'that decision citing this doc: %s\n' "$( command grep -c 'type/001' docs/decisions/002_should_a_stopped_token_be_unique.md || true )"
printf 'what property 1 claims:        %s\n' "$( awk '/^### Regenerate/{ exit } { print }' $D | tr '\n' ' ' | sed 's/  */ /g' | command grep -o 'no expression that reaches .this crate.s. drain without closing first' )"
printf 'what its own probe scopes it to: %s\n' "$( awk '/What this stage does not establish/{f=1} f&&/^## D2/{exit} f' tests/manual/readme.md | tr '\n' ' ' | sed 's/  */ /g' | command grep -o 'That a caller \*cannot\* drain an open ring. They can: .split(). hands out a .Consumer. and .try_recv_batch. is public on it.' )"
printf 'the probe that verifies it:    %s\n' "$( command grep -o '^## D1 — .*' tests/manual/readme.md )"
printf 'error codes it recorded:       %s\n' "$( command grep -ohE 'E0[0-9]{3}' tests/manual/readme.md | sort -u | tr '\n' ' ' )"
printf 'public drains outside a token: %s\n' "$( cd ..; command grep -ohE 'pub fn try_recv(_batch)?' ring_core/src/lib.rs | sort -u | tr '\n' ' ' )"
```

Live output:

```
the token as declared:         pub struct Stopped<'a> { shutdown: &'a Shutdown, } 
alternatives it rejects:       4
of those, also filed as open:  1
ground given in the type doc:  Breaks idempotence, which teardown needs: an unwind path and a normal path both close, and neither should have to know which is first
where it is open instead:      docs/decisions/002_should_a_stopped_token_be_unique.md
its status there:              - **Status:** ❓ open
ground given in the decision:  Kills the idempotence property `close`'s doc names as the reason it exists; a second teardown path gets `None` and cannot drain, which is the case idempotence was for
this doc's body citing it:     0
that decision citing this doc: 2
what property 1 claims:        no expression that reaches *this crate's* drain without closing first
what its own probe scopes it to: That a caller *cannot* drain an open ring. They can: `split()` hands out a `Consumer` and `try_recv_batch` is public on it.
the probe that verifies it:    ## D1 — The two `Stopped` properties are compile errors, not runtime ones
error codes it recorded:       E0382 E0599 
public drains outside a token: pub fn try_recv pub fn try_recv_batch 
```

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_proof_token_orders_two_operations.md](../pattern/001_proof_token_orders_two_operations.md) | The reusable form, its applicability conditions, and this cost stated generally |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_drain_terminates_because_close_preceded_it.md](../invariant/002_drain_terminates_because_close_preceded_it.md) | The argument property 1 exists to support |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/002_open_and_closed.md](../lifecycle/002_open_and_closed.md) | The states the token distinguishes, and the edges it gates |

### Tests

| File | Relationship |
|------|--------------|
| `tests/shutdown_test.rs` | `close_is_idempotent_and_admit_reports_it` — the token points back at its own flag, and closing twice yields two usable tokens |
| `tests/manual/readme.md` | D1 — that properties 1 and 2 are *compile* failures, which a passing test suite cannot demonstrate |

### SD45 — The Same Alternative Is Filed as Rejected Here and as Open Four Directories Away

Row 3 of Alternatives Rejected is *"`close` returning `Option< Stopped >`,
`None` on an already-closed flag"*, dismissed because it *"Breaks idempotence,
which teardown needs: an unwind path and a normal path both close, and neither
should have to know which is first"*. Option 3 of
[`../decisions/002`](../decisions/002_should_a_stopped_token_be_unique.md) is
`close( &self ) -> Option< Stopped >`, still under consideration, dismissed
nowhere, on a document whose Status is `❓ open` — and its cost column gives the
same argument in different words: *"Kills the idempotence property `close`'s doc
names as the reason it exists; a second teardown path gets `None` and cannot
drain, which is the case idempotence was for."*

One alternative, one objection, two files, two statuses. A reader of this
document concludes the design space was explored and closed. A reader of the
decision concludes the crate has not made up its mind. Both are in `docs/`, and
the corpus offers no way to notice, because nothing compares two documents.

The citation graph makes it worse rather than better. The decision cites this
document twice; this document does not cite the decision at all. So the newer,
open, undecided record knows about the older, closed one and did not treat it as
settling anything — which is defensible, since the decision is about token
*uniqueness* and this table is about `close`'s *return shape* — but the reader
who arrives here first never learns the second document exists.

The narrow fix is a link. The real one is a rule: **an "Alternatives Rejected"
table asserts a status, and a status asserted in a type document is invisible to
the decision system that owns statuses.** Either the row cites the decision that
reopened it, or the rejection belongs in the decision record and not here.

### SD46 — Property 1 Is Stated Without Its Qualifier, and Its Own Verification Probe Supplies the Qualifier

The Validation section's first property closes with *"There is no expression that
reaches a drain without closing first."* Unqualified, that is the crate's
strongest claim, and it is the premise
[`../invariant/002`](../invariant/002_drain_terminates_because_close_preceded_it.md)
builds its termination argument on.

The probe this document names as its verification says otherwise, in its own
words. `tests/manual/readme.md` D1 records both compile errors — `E0599` for a
drain without a token, `E0382` for a drain after a `reopen` — and then adds a
paragraph headed *"What this stage does not establish"*: *"That a caller
**cannot** drain an open ring. They can: `split()` hands out a `Consumer` and
`try_recv_batch` is public on it."*

Both `try_recv` and `try_recv_batch` are public on `ring_core::Consumer`, and
`drain_all` is a four-line loop over the second one. A caller who writes that
loop by hand on an open ring gets the identical hang the token exists to
foreclose, with no `Stopped` in the program. What property 1 actually says is
*there is no expression that reaches* **this crate's** *drain without closing
first* — true, verified, and much narrower than the sentence in the document.

The asymmetry is the interesting part: the qualification exists, it is precise,
and it is written down in the artifact that verifies the claim rather than in
the artifact that makes it. A reader who follows the document to its Tests table
finds it; a reader who stops at the Validation heading, which is where the claim
is stated as a heading-level property, does not. This is the same shape as
[`../invariant/002`](../invariant/002_drain_terminates_because_close_preceded_it.md)'s
absolute enforcement claim (→ SD23), reached from the opposite direction: there
the counterexample was in the test suite, here the qualifier is in the manual
probe, and in both cases the strong sentence and the honest one are in different
files.

**Disposition:** applied — Property 1 now reads "there is no expression that
reaches *this crate's* drain without closing first" and names the way past
this crate's own surface (`ring_core::Producer::split` → a raw `Consumer`
whose `try_recv`/`try_recv_batch` are public), matching what D1's manual
probe already establishes rather than leaving the qualifier only in the
probe's own "What this stage does not establish" paragraph. Now prints:
`what property 1 claims:        no expression that reaches *this crate's* drain without closing first`

```sh
cd "$(git rev-parse --show-toplevel)"/ring_shutdown
D=docs/type/001_stopped_proof_token.md
awk '/^### Validation/{f=1} f&&/^### Alternatives Rejected/{exit} f' $D | tr '\n' ' ' | sed 's/  */ /g' | command grep -o 'this crate.s. drain without closing first. It does not reach past this crate.s own surface'
awk '/^### Regenerate/{f=1} f&&/^### Patterns/{exit} f' $D | command grep 'what property 1 claims:' | command grep -v '^printf'
```

Live output:

```
this crate's* drain without closing first. It does not reach past this crate's own surface
what property 1 claims:        no expression that reaches *this crate's* drain without closing first
```
