# A `debug_assert` Where the Type System Cannot Reach

### Scope

- **Purpose**: Record why the MPSC arm of `try_push` verifies an obligation with a `debug_assert` rather than with a type, and state the condition that would retire it.
- **Responsibility**: The obligation, why it is unexpressible, the release-build gap, and a falsifiable removal trigger.
- **In Scope**: `reserved.set( record )`'s discarded return value.
- **Out of Scope**: The claim-before-move pattern that creates the obligation (→ [`../pattern/002`](../pattern/002_claim_before_move_so_a_refusal_can_hand_the_record_back.md)).

### The Obligation

`ring_slot`'s reservation `set` returns `Option< T >` — the record it displaced —
because a slot may in general hold one. A slot just returned by `claim()` cannot,
so the return is always `None` here, and the code says so:

```rust
let displaced = reserved.set( record );
debug_assert!( displaced.is_none(), "a claimed slot held a record" );
```

**The type cannot say it.** `claim()` returns the same reservation type whether
or not the slot was empty; expressing "this one is known empty" needs either a
second type (`EmptyReservation`, converting on first write) or a typestate
parameter on the existing one. Both are `ring_slot`'s to introduce, not this
crate's.

### What the Gap Is

```sh
cd "$(git rev-parse --show-toplevel)"/ring_core
printf 'debug_assert:  '; grep -c 'debug_assert' src/lib.rs
printf 'assert!:       '; grep -cE '^\s+assert!' src/lib.rs
printf 'expect:        '; grep -c '\.expect(' src/lib.rs
# The command above can legitimately exit nonzero -- a count of zero, a diff
# that differs -- and the sweep that keeps this output current skips any block
# that exits nonzero, so without this the block freezes instead of refreshing.
:
```

Live output:

```
debug_assert:  1
assert!:       0
expect:        0
```

In a release build the assertion is compiled out and `displaced` is dropped. If
the invariant were ever violated, a record would be silently destroyed rather
than delivered — the same failure class the family's exactly-once guarantee
exists to prevent, on the one path where it is checked by a build-mode-dependent
mechanism.

**This is a workaround rather than a defect** because the alternative is not
available from here: it is a change to `ring_slot`'s reservation type, affecting
every crate that claims a slot.

### Removal Trigger

`ring_slot` gains a reservation type that distinguishes a known-empty slot. Then
`set` on it returns `()` and the assertion has nothing to check.

```sh
cd "$(git rev-parse --show-toplevel)"
grep -cE '^pub (struct|enum) ' ring_slot/src/lib.rs
```

Live output:

```
2
```

Two today. The day that count rises with a known-empty reservation type in it,
this instance is obsolete. Until then the `debug_assert` is the strongest check available at this
layer.

### CO52 — The Crate's Only Remaining Obligation Is the One a Release Build Drops

```sh
cd "$(git rev-parse --show-toplevel)"/ring_core
printf 'debug_assert: '; grep -c 'debug_assert' src/lib.rs
printf 'expect:       '; grep -c '\.expect(' src/lib.rs
printf 'unwrap:       '; grep -c '\.unwrap()' src/lib.rs
printf 'unsafe:       '; grep -c 'unsafe' src/lib.rs
# The command above can legitimately exit nonzero -- a count of zero, a diff
# that differs -- and the sweep that keeps this output current skips any block
# that exits nonzero, so without this the block freezes instead of refreshing.
:
```

Live output:

```
debug_assert: 1
expect:       0
unwrap:       18
unsafe:       0
```

**What was found.** Two runtime obligations, failing differently. The `expect`
in `Ring::capacity` aborted in every build — loud, and the one place this crate
could end a program. The `debug_assert` in `try_push` is compiled out of a
release build, and its failure mode there is a silently dropped record. The
quieter one guarded the more valuable property: a lost record is the exact
failure the family's exactly-once guarantee exists to prevent, and it was the
obligation checked only in debug.

**One of the two is gone, and the reading gets sharper rather than milder.**
`Ring::capacity` no longer re-runs a fallible validation to answer an infallible
question — the variant carries an already-validated `Capacity` and the accessor
reads it — so the `expect` has no call site left. What remains is not two
obligations of differing loudness but one: the crate's *only* runtime obligation
is now the one a release build drops, and it is the one standing between a
claimed slot and a silently destroyed record.

**Disposition:** applied — `Ring::capacity` reads a validated `Capacity` carried in the variant instead of re-running a fallible validation, so the `expect` is gone and the two obligations are one. Retiring the surviving `debug_assert` needs the known-empty reservation type named in the Removal Trigger above, which is `ring_slot`'s to introduce. Now prints: `expect:       0`
