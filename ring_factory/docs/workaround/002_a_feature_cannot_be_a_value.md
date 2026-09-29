# Workaround: A Feature Cannot Be a Value

### Scope

**Purpose:** Record the cargo constraint that forces the crossbeam backend to be
reached through a second function rather than through a `RingConfig` field,
what that costs a caller, and the exact condition under which the second door can
be deleted.

**Responsibility:** `Factory::build_crossbeam` and the `crossbeam` feature
declarations that gate it.

**In Scope:** The four `crossbeam` feature stanzas in the family's manifests;
`ring_factory/src/lib.rs`, the `#[ cfg ]` on the third verb.

**Out of Scope:** The ruling itself — why routing was rejected — is
[`decisions/002`](../decisions/002_two_doors_not_one_that_routes.md). That the
door is closed on every default build is
[`item/001`](../item/001_three_verbs_one_of_them_conditional.md) FC25. This
instance is about the language-level constraint underneath both.

---

## The Constraint

A cargo feature is resolved at build time and is **additive**: enabling it may
add capability and may never remove any, and the same crate compiled twice in one
dependency graph gets the union of every feature any dependent asked for. So a
feature is not a value a running program can branch on to choose behaviour a
caller requested — it is a fact about the binary.

`OverflowPolicy::DropOldest` is a value. The in-house backends refuse it and
`crossbeam_queue::ArrayQueue::force_push` implements it exactly. Writing the
obvious `build` — inspect the policy, route to crossbeam when it is `DropOldest`
— makes one `RingConfig` produce a ring on one backend or a refusal, depending on
whether some *other* crate in the graph happened to turn the feature on.

That collides head-on with `invariant/001`: a configuration fully determines the
ring. So the routing was refused and the capability got a door of its own.

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- every crossbeam feature declaration in the tree --'
# anchored on the declaration, not on `[features]` with a fixed window: both
# stanzas carry several comment lines before the entry, so an `-A3` window after
# the header prints comments and reports the feature as declared nowhere
command grep -r --exclude-dir=-target_gate --exclude-dir=target '^crossbeam = ' --include=Cargo.toml /home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/ . /home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/spike/ | sed -E 's#^(/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/division/[^/]+|/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/substrate/[^/]+|module|ring|spike)/##'
echo '  -- and the one declaration it gates --'
command grep -A1 'cfg( feature = "crossbeam" )' ring_factory/src/lib.rs
```

Live output:

```
  -- every crossbeam feature declaration in the tree --
ring_handle/Cargo.toml:crossbeam = [ "ring_core/crossbeam" ]
ring_bench/Cargo.toml:crossbeam = [ "ring_factory/crossbeam" ]
ring_core/Cargo.toml:crossbeam = [ "dep:crossbeam-queue" ]
ring_factory/Cargo.toml:crossbeam = [ "ring_core/crossbeam" ]
  -- and the one declaration it gates --
  #[ cfg( feature = "crossbeam" ) ]
  pub fn build_crossbeam< S : Send >( &self, cfg : RingConfig ) -> Result< Split< S >, BuildError >
```

Four declarations, one dependency. Only `ring_core`'s stanza names a crate
(`dep:crossbeam-queue`); the other three forward to it, from two independent
roots — `ring_bench` through this crate, and `ring_handle` straight to
`ring_core`. So the additive rule bites across a chain rather than at one door:
a consumer that turns the feature on for `ring_handle` also turns it on for the
`ring_core` that this crate builds through, and this crate's own third verb is
still absent unless someone names `ring_factory/crossbeam` specifically. The
feature is one word in four places and does not mean the same thing in all of
them.

---

### FC51 — The Invariant Is Preserved by Making It Not Apply

`invariant/001` says the configuration fully determines the ring. `build`
satisfies it. `build_crossbeam` does not even attempt to — it selects crossbeam
whatever the configuration says, and its own doc comment leads with "Outside the
one-door promise, deliberately."

So the invariant survives in the form "*`build`* is fully determined by its
config", which is weaker than the sentence `invariant/001` states and is not the
sentence anybody would quote. The mechanism that keeps the invariant true is that
the traffic which would falsify it was moved to a function the invariant does not
cover.

This is the honest resolution and it is worth naming as what it is, because the
same move is available for any future capability that conflicts with the
invariant, and applying it twice more leaves an invariant that is true of a
shrinking fraction of the surface while still reading as a property of the crate.
The guard against that is not a rule; it is that each new door has to be argued
for individually, in `decisions/`, which is where the second one was.

---

### FC52 — The Deletion Condition Is Reachable and Nothing Is Watching for It

The second door folds away the moment backend selection becomes a value rather
than a build-time fact — that is, the moment `RingConfig` grows a `backend` field
naming a backend explicitly. Then `build` reads the field, the config determines
the ring in the full sense again, and `build_crossbeam` has nothing left to do
that `build` cannot.

That is not hypothetical. Backend selection is **already** config-driven, through
a predicate `ring_config` derives from its `producers` field — so the in-house
half of the choice is a value already, and only the crossbeam half is a build-time
fact:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the branch, and the predicate it reads --'
# `is_multi_producer()`, not `producers`: the field is private and `ring_core`
# never names it, so a scan for the field reports no backend branch at all
command grep 'is_multi_producer' ring_core/src/lib.rs ring_config/src/lib.rs \
  | sed 's|ring/||'
echo '  -- and the two arms it picks between --'
command grep 'Storage::Mpsc( ring_mpsc\|Storage::Spsc( ring_spsc' ring_core/src/lib.rs
```

Live output:

```
  -- the branch, and the predicate it reads --
ring_core/src/lib.rs:  /// [`RingConfig::is_multi_producer`] chooses between [`ring_spsc`] and
ring_core/src/lib.rs:    let storage = match config.is_multi_producer()
ring_config/src/lib.rs:/// assert!( cfg.is_multi_producer() );
ring_config/src/lib.rs:  /// assert!( !RingConfig::new( 8 ).unwrap().is_multi_producer() );
ring_config/src/lib.rs:  /// assert!( RingConfig::new( 8 ).unwrap().with_producers( 2 ).is_multi_producer() );
ring_config/src/lib.rs:  pub const fn is_multi_producer( &self ) -> bool
  -- and the two arms it picks between --
      true => Storage::Mpsc( ring_mpsc::Ring::with_config( config ) ),
      false => Storage::Spsc( ring_spsc::Ring::with_config( config ) ),
```

Nothing records this as a pending question. `decisions/` closed eight of its ten
pendings and neither of the remaining two is this one, so the condition that
would retire a whole public method is written down here and in no register that
anybody reviews on a schedule. That is the finding: not that the workaround is
wrong, but that its expiry is undated and unwatched, which is how a workaround
becomes permanent by default.

---

### Sources

| Source | What it establishes |
|--------|---------------------|
| The four `crossbeam` stanzas in `ring_core`, `ring_factory`, `ring_handle`, `ring_bench` | The feature is declared four times, pulls a dependency once, and is enabled nowhere |
| `ring_factory/src/lib.rs` | The `#[ cfg ]`, and `build_crossbeam`'s own "outside the one-door promise" comment |
| `ring_config/src/lib.rs`, `ring_core/src/lib.rs` | `producers` already selects a backend from a config value, which is the shape the deletion condition needs |

### Tests

| Test | What it holds |
|------|---------------|
| `the_two_doors_agree_on_capacity` | The two backends are interchangeable wherever both accept the config — the precondition for ever folding them |
| `the_backend_does_change_with_producer_count_where_it_can_still_be_seen` | Backend selection is already driven by a config field, which is what makes the deletion condition reachable |
