# Decision: Two Doors, Not One That Routes

**Status:** accepted, 2026-08-28. Rules Pendings 9 and 10 together, which
[`readme.md`](readme.md) recorded separately and which turn out to be one
question asked from two ends.

### Scope

- **Purpose**: Rule Pendings 9 and 10 together, since a refusal that must be routed and a backend that must be reachable turn out to be two halves of one arrangement.
- **Responsibility**: Why the two pendings are one question, the routing answer that looks right and is not, the ruling, how a refusal is relayed rather than re-decided, and what the shape costs.
- **In Scope**: How `ring_core`'s `DropOldest` refusal reaches a caller, and whether the crossbeam backend is reachable through the Contract.
- **Out of Scope**: What `build` returns and who owns it (→ [`001_the_owner_is_the_return_value.md`](001_the_owner_is_the_return_value.md)); the policy's own semantics (→ [`ring_core/docs/lifecycle/001`](../../../ring_core/docs/lifecycle/001_construction_and_backend_selection.md)).

### Why they are one question

Pending 9: `ring_core::Ring::new` refuses `OverflowPolicy::DropOldest`, from a
config `ring_config` validated and accepted. How does that refusal reach the
caller?

Pending 10: `ring_core` grew an optional crossbeam backend behind
`new_crossbeam`. Is it reachable through the Contract's door?

**They are the same question because the crossbeam backend is the one that
accepts `DropOldest`:**

```sh
cd "$(git rev-parse --show-toplevel)"
command grep 'force_push\|DropOldest' ring_core/src/lib.rs
```

Live output:

```
//! | `OverflowPolicy::DropOldest` | **rejected at construction** | rejected | supported |
  /// [`RingError::PolicyUnsupported`] for `OverflowPolicy::DropOldest`, which neither
  /// let evicting = RingConfig::new( 8 ).unwrap().with_overflow( OverflowPolicy::DropOldest );
    if config.overflow() == OverflowPolicy::DropOldest
  /// This is feature 187's entry point. It accepts `OverflowPolicy::DropOldest`
  /// where [`new`](Self::new) refuses it, because `ArrayQueue::force_push` does
  /// `DropOldest` cannot arrive here — [`Ring::new`] refuses it, and
        if self.overflow == OverflowPolicy::DropOldest
          let _evicted = queue.force_push( record );
```

`ArrayQueue::force_push` evicts, which is exactly what the policy asks for and
exactly what the in-house rings' exactly-once delivery forbids. So the refusal
Pending 9 must route and the backend Pending 10 must reach are two halves of one
arrangement: **`build` refuses the policy, and the thing it refuses in favour of
is the door Pending 10 is about.**

### The tempting answer, and why it is wrong

Route inside one function: if `overflow == DropOldest`, call `new_crossbeam`.

It looks like it resolves both pendings at zero cost — the third backend becomes
reachable through `build` without a non-config input, so
[`invariant/001`](../invariant/001_configuration_fully_determines_the_ring.md)'s
"configuration fully determines the ring" appears to survive.

**It does not survive.** `new_crossbeam` is behind `#[ cfg( feature =
"crossbeam" ) ]`, so under routing the same `RingConfig` produces a crossbeam
ring in one build and an error in another, decided by a cargo feature the config
knows nothing about. That is precisely the failure `invariant/001` names, and it
would be worse than the honest version of it, because the config is *silently*
sufficient to explain the result in one build and not the other.

[`algorithm/001`](../algorithm/001_selecting_a_backend_from_one_boolean.md)
already recorded `ring_core`'s reason for keeping the backend out of
`RingConfig`: it is "a build-time opt-in rather than a property of the
workload." Routing would smuggle a build-time opt-in back into a workload
property.

### The ruling

**Two functions.**

```rust
pub fn build< S : Send >( &self, cfg : RingConfig ) -> Result< Split< S >, BuildError >;

#[ cfg( feature = "crossbeam" ) ]
pub fn build_crossbeam< S : Send >( &self, cfg : RingConfig ) -> Result< Split< S >, BuildError >;
```

| Pending | Resolved by |
|---|---|
| 9 | `BuildError::Unsupported( RingError )` — `build` and `build_named` both relay `ring_core`'s refusal rather than re-deciding it |
| 10 | `build_crossbeam` — the optional crossbeam backend's door, documented as outside the one-door promise rather than pretending to be inside it |

`invariant/001` stays true of `build` **without a proviso**: given the config,
the ring is determined. `build_crossbeam` is a second door and says so; a reader
asking "what determines the ring?" gets "the config" for one function and "the
config, and the fact that you called this one" for the other. Both are
answerable. A routing `build` would be answerable only with a footnote about
feature flags.

### The refusal is relayed, never re-decided

`build` does not check the policy itself. It hands the config to
`ring_core::Ring::new` and wraps whatever comes back:

```rust
let ring = Ring::new( &cfg ).map_err( BuildError::Unsupported )?;
```

The alternative — refusing `DropOldest` here before delegating — was Pending 9's
second option and is rejected for a reason that has already come true once in
this family: it duplicates a policy decision that belongs to the backend, and a
fourth backend disagreeing would make the duplicate silently wrong. This crate
is one level above the crate that knows; it should not hold a second copy of the
answer. Asserted by `the_policy_refusal_matches_the_backend_that_makes_it`,
which compares the two side by side rather than hard-coding the expected
variant.

Pending 9's third option — remove `DropOldest` from `OverflowPolicy` so it
cannot be expressed — is refused outright: the policy is meaningful, the
crossbeam backend implements it, and deleting vocabulary to avoid an error case
would make the family unable to describe a ring it can build.

### What this costs

**`build` returns `Result` where the original design says it returns "a handle pair".**
That phrasing predates any backend that could refuse anything, and
[`api/001`](../api/001_the_build_surface.md)'s B2 already records the reversal.
Both operations now return `Result`, so the two signatures differ only by the
`name` argument and by which variants they can produce — `NameTaken` is
unreachable from `build`, asserted by
`the_unnamed_path_can_never_return_name_taken`.

**A consumer wanting eviction must know to call the other function**, and
nothing in the type system tells them. The mitigation is that the error says so:
`BuildError::Unsupported` renders `ring_core`'s own wording, which names the
policy as the thing the backend cannot honour. It is a weaker guarantee than a
compile error and it is the one available without giving `RingConfig` a
backend field.

### Verification

```bash
cd "$(git rev-parse --show-toplevel)"
cargo test -p ring_factory --all-features      # includes the crossbeam door
cargo test -p ring_factory                     # the door is absent, everything else passes
```

| Claim | Test |
|---|---|
| Both paths relay the same refusal | `both_paths_relay_the_same_refusal` |
| The relayed refusal is the backend's own, not a copy | `the_policy_refusal_matches_the_backend_that_makes_it` |
| The crossbeam door accepts what the in-house backends refuse | `the_crossbeam_door_accepts_the_policy_the_in_house_backends_refuse` |
| The two doors otherwise agree | `the_two_doors_agree_on_capacity` |
| A refusal reaches the caller before any ring exists | `both_paths_relay_the_same_refusal`'s `registry.is_empty()` |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_core/src/lib.rs`](../../../ring_core/src/lib.rs) | `Ring::new`'s refusal and `new_crossbeam`'s acceptance — the asymmetry this ADR is about |
| [`ring_types/src/error.rs`](../../../ring_types/src/error.rs) | `RingError::PolicyUnsupported`, the payload `Unsupported` carries |

### FC15 — The Two Doors Have the Same Signature, So the Ruling Is Enforced by a Name

Routing was rejected so that one config could not produce two different rings.
What the two doors actually differ by, at the type level, is nothing:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep 'pub fn build< S : Send >\|pub fn build_crossbeam< S : Send >' \
  ring_factory/src/lib.rs
```

Live output:

```
  pub fn build< S : Send >( &self, cfg : RingConfig ) -> Result< Split< S >, BuildError >
  pub fn build_crossbeam< S : Send >( &self, cfg : RingConfig ) -> Result< Split< S >, BuildError >
```

Identical receiver, identical argument, identical return type. A caller swapping
`build` for `build_crossbeam` changes one identifier and gets a ring on a
different backend with no type error, no warning, and — under
`--all-features`, which is what every verification level in this family runs —
no compile failure either.

That is the ruling working as designed and it is also its whole enforcement
surface. The invariant that a config determines the ring is preserved by the two
functions being *named* differently, which means it survives exactly as long as
callers read the names. A wrapper that picked between them — the thing the
ruling forbids — would be four lines in a consumer crate and nothing in this
family would notice.

The narrower reading is the one to keep: the ruling is sound and its guarantee
does not compose. It holds for a caller of this crate and says nothing about
what a caller may build on top.

`build_crossbeam`'s own doc comment now names the composability trap
explicitly, immediately after the sentence that used to state the identical
signature as a bare positive:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -A6 "signature matches \[\`build\`\](Self::build)" ring_factory/src/lib.rs
```

Live output:

```
    /// None currently. The signature matches [`build`](Self::build) so the two
    /// are interchangeable at a call site, which is the point of a swappable
    /// backend — **at a call site fixed ahead of time**, never inside a wrapper
    /// that reads `cfg` and picks between them at runtime. That would
    /// reintroduce exactly the second input `docs/invariant/001` forbids, and
    /// nothing in either signature stops it from being written.
    #[cfg(feature = "crossbeam")]
```

**Disposition:** applied — `Factory::build_crossbeam`'s doc comment in
`ring_factory/src/lib.rs` now states that the shared signature is
"the point of a swappable backend" only at a call site fixed ahead of time,
and explicitly names the hazard this finding describes — a runtime-dispatching
wrapper reading `cfg` — as the thing that would reintroduce the second input
`invariant/001` forbids. This does not narrow the ruling `decisions/002`
already accepted (the guarantee still does not compose beyond a direct
caller); it puts the warning where a future reader of the source, not only
the ADR, will meet it. The crate's test suite re-verified passing
(`cargo test -p ring_factory --all-features`, 2026-09-04). Now prints:
`nothing in either signature stops it from being written.`

### FC16 — Eight of Ten Pendings Closed, and the Two Left Are Not the Ones This Ruling Created

Two ADRs closed six pendings between them, and the manifest closed a seventh.
What remains open is checkable:

```sh
cd "$(git rev-parse --show-toplevel)"/ring_factory/docs/decisions
echo '  -- the open pendings --'
command grep '^\*\*Pending [0-9]* —' readme.md
echo '  -- and the rulings that closed the rest --'
command grep '^| \[00' readme.md
echo '  -- the register own count --'
command grep 'pendings stood here' readme.md
```

Live output:

```
  -- the open pendings --
**Pending 5 — does the factory resolve `WaitKind`, or pass the record down?**
**Pending 8 — should `Ring::with_config` exist?**
  -- and the rulings that closed the rest --
| [001 — The Owner Is The Return Value](001_the_owner_is_the_return_value.md) | Pendings 1, 2, 3, 4 | accepted 2026-08-28 |
| [002 — Two Doors, Not One That Routes](002_two_doors_not_one_that_routes.md) | Pendings 9, 10 | accepted 2026-08-28 |
  -- the register own count --
**Ten pendings stood here; two remain.** Eight were closed when this crate was
```

Pendings 5 and 8 — where `WaitKind` is carried, and whether
`Ring::with_config` should exist. Neither is this ruling's residue.

The residue this ruling did create is unregistered. `build_crossbeam` folds away
the moment backend selection becomes a config value
(→ [`workaround/002`](../workaround/002_a_feature_cannot_be_a_value.md) FC52),
and that condition appears in no pending, no task, and no gate. A decisions
register that tracks two open questions accurately while omitting the expiry of
a public method is measuring the wrong thing well — the register's shape assumes
questions arrive open and are then closed, and has nowhere to put a question that
a closed ruling will reopen later.
