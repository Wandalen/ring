# Workaround: The Question Mark Forecloses `const`

### Scope

**Purpose:** Record why the crate's one non-`const` function is non-`const`, what
it would take to change that, and where the technique already exists in the
family.

**Responsibility:** The three independent blockers in `RingConfig::new`'s body,
every `const fn` returning `Result` anywhere in the crate tree, and the coupling that
would be traded away.

**In Scope:** `ring_config/src/lib.rs:65-77`;
`ring_types/src/capacity.rs:40-51`;
`/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/substrate/exact/exact_decimal/src/lib.rs`, `/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/substrate/exact/exact_qty/src/lib.rs`;
`ring_config/tests/config_test.rs:41-42`.

**Out of Scope:** The `const` surface as a whole is
[`api/001`](../api/001_twelve_functions_eleven_of_them_const.md). Why the
constructor is fallible at all is
[`decisions/001`](../decisions/001_capacity_is_the_one_parameter_that_is_not_a_with.md).

---

## The One Non-`const` Function, and the One That Manages It

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the one function of twelve that is not const --'
command grep -c 'pub const fn' ring_config/src/lib.rs || true
command grep 'pub fn ' ring_config/src/lib.rs || true
echo '  -- its body, and the three things in it that a const fn cannot do --'
command grep -m1 -A10 -F '    Ok' ring_config/src/lib.rs
echo '  -- the same familys fallible constructor, which is const --'
command grep -m1 -A11 -F '  pub const fn new( slots : usize ) -> Result< Self, RingError >' ring_types/src/capacity.rs
echo '  -- every const fn returning a Result anywhere in the crate tree --'
command grep -r 'pub const fn [a-z_]*(.*) -> Result' --include=*.rs */src | sed -E 's#^(/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/division/[^/]+|/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/substrate/[^/]+|module|ring|spike)/##'
echo '  -- and what the suite pins the two defaults to --'
command grep -m1 -A1 -F '  assert_eq!( cfg.wait(), WaitKind::Spin );' ring_config/tests/config_test.rs
```

Live output:

```
  -- the one function of twelve that is not const --
11
  pub fn new( slots : usize ) -> Result< Self, RingError >
  -- its body, and the three things in it that a const fn cannot do --
    Ok
    (
      Self
      {
        capacity : Capacity::new( slots )?,
        wait : WaitKind::default(),
        overflow : OverflowPolicy::default(),
        producers : 1,
        batch : 1,
      }
    )
  -- the same familys fallible constructor, which is const --
  pub const fn new( slots : usize ) -> Result< Self, RingError >
  {
    if slots == 0
    {
      return Err( RingError::CapacityZero );
    }
    if !slots.is_power_of_two()
    {
      return Err( RingError::CapacityNotPowerOfTwo( slots ) );
    }
    Ok( Self( slots ) )
  }
  -- every const fn returning a Result anywhere in the crate tree --
ring_types/src/capacity.rs:  pub const fn new( slots : usize ) -> Result< Self, RingError >
exact_decimal/src/lib.rs:  pub const fn from_minor( minor : Backing ) -> Result< Self, DecimalError >
exact_decimal/src/lib.rs:  pub const fn from_int( whole : Backing ) -> Result< Self, DecimalError >
exact_decimal/src/lib.rs:  pub const fn checked_add( self, rhs : Self ) -> Result< Self, DecimalError >
exact_decimal/src/lib.rs:  pub const fn checked_sub( self, rhs : Self ) -> Result< Self, DecimalError >
exact_decimal/src/lib.rs:  pub const fn checked_mul_int( self, n : Backing ) -> Result< Self, DecimalError >
exact_decimal/src/lib.rs:  pub const fn checked_neg( self ) -> Result< Self, DecimalError >
exact_qty/src/lib.rs:  pub const fn from_decimal( value : Decimal< SCALE > ) -> Result< Self, QtyError >
exact_qty/src/lib.rs:  pub const fn from_minor( minor : Backing ) -> Result< Self, QtyError >
exact_qty/src/lib.rs:  pub const fn from_int( whole : Backing ) -> Result< Self, QtyError >
exact_qty/src/lib.rs:  pub const fn checked_add( self, rhs : Self ) -> Result< Self, QtyError >
exact_qty/src/lib.rs:  pub const fn checked_sub( self, rhs : Self ) -> Result< Self, QtyError >
exact_qty/src/lib.rs:  pub const fn checked_mul_int( self, n : Backing ) -> Result< Self, QtyError >
  -- and what the suite pins the two defaults to --
  assert_eq!( cfg.wait(), WaitKind::Spin );
  assert_eq!( cfg.overflow(), OverflowPolicy::DropNewest );
```

---

## What the Compiler Says to Each Form

Three probe binaries over a copy of the record. The first is `RingConfig::new`'s
body with `const` added and nothing else changed. The second replaces `?` with the
`match` `Capacity::new` uses. The third keeps that `match` and puts the two
`::default()` calls back.

Each probe is written, compiled and read in one block, so the program producing
the evidence is the evidence. Two details make bare `rustc` run rather than
nearly run: it defaults to edition 2015, and `--crate-name` is required because
a hyphen-prefixed output path is not a legal crate name. Only `^error` lines are
kept — the `-->` spans and caret rules underneath them address lines inside
these throwaway files, which no reader can navigate to and which shift whenever
the probe is edited.

```sh
cd "$(git rev-parse --show-toplevel)"

cat > /tmp/-const_qmark.rs <<'RS'
pub struct RingError;
#[ derive( Clone, Copy ) ] pub struct Capacity( usize );
#[ derive( Clone, Copy ) ] pub enum WaitKind { Spin }
#[ derive( Clone, Copy ) ] pub struct Cfg { capacity : Capacity, wait : WaitKind }
impl Capacity
{
  pub const fn new( slots : usize ) -> Result< Capacity, RingError >
  {
    if slots == 0 { return Err( RingError ) }
    Ok( Capacity( slots ) )
  }
}
impl Cfg
{
  // RingConfig::new's body, with `const` added and nothing else changed
  pub const fn new( slots : usize ) -> Result< Cfg, RingError >
  {
    Ok( Cfg { capacity : Capacity::new( slots )?, wait : WaitKind::Spin } )
  }
}
fn main() {}
RS
echo '  -- the constructor body as written, with const added --'
rustc -O -A dead_code --crate-name const_qmark \
  -o /tmp/-const_qmark /tmp/-const_qmark.rs 2>&1 | command grep '^error'

cat > /tmp/-const_match.rs <<'RS'
#[ derive( Debug ) ] pub struct RingError;
#[ derive( Debug, Clone, Copy ) ] pub struct Capacity( usize );
#[ derive( Debug, Clone, Copy ) ] pub enum WaitKind { Spin }
#[ derive( Debug, Clone, Copy ) ] pub struct Cfg { capacity : Capacity, wait : WaitKind }
impl Capacity
{
  pub const fn new( slots : usize ) -> Result< Capacity, RingError >
  {
    if slots == 0 { return Err( RingError ) }
    Ok( Capacity( slots ) )
  }
}
impl Cfg
{
  // the same constructor, written the way Capacity::new writes its own
  pub const fn new( slots : usize ) -> Result< Cfg, RingError >
  {
    let capacity = match Capacity::new( slots )
    {
      Ok( capacity ) => capacity,
      Err( error ) => return Err( error ),
    };
    Ok( Cfg { capacity, wait : WaitKind::Spin } )
  }
}
// the proof of const-ness: an array length can only come from a value the
// compiler already has, so this file does not link unless `new` ran at compile time
const BUILT : Cfg = match Cfg::new( 8 )
{
  Ok( cfg ) => cfg,
  Err( _ ) => panic!( "unreachable for 8" ),
};
const SLOTS : usize = BUILT.capacity.0;
static PROOF : [ u8 ; SLOTS ] = [ 0 ; SLOTS ];
fn main()
{
  println!( "{:?}", BUILT );
  println!( "array length fixed at compile time from the constructor: {}", PROOF.len() );
}
RS
echo '  -- the same constructor, written the way Capacity::new writes its own --'
rustc -O -A dead_code --crate-name const_match \
  -o /tmp/-const_match /tmp/-const_match.rs 2>&1 | command grep '^error'
/tmp/-const_match

cat > /tmp/-const_match_default.rs <<'RS'
#[ derive( Debug ) ] pub struct RingError;
#[ derive( Debug, Clone, Copy ) ] pub struct Capacity( usize );
#[ derive( Debug, Clone, Copy ) ] pub enum WaitKind { Spin }
#[ derive( Debug, Clone, Copy ) ] pub enum OverflowPolicy { DropNewest }
impl Default for WaitKind { fn default() -> WaitKind { WaitKind::Spin } }
impl Default for OverflowPolicy { fn default() -> OverflowPolicy { OverflowPolicy::DropNewest } }
#[ derive( Debug, Clone, Copy ) ] pub struct Cfg
{ capacity : Capacity, wait : WaitKind, overflow : OverflowPolicy }
impl Capacity
{
  pub const fn new( slots : usize ) -> Result< Capacity, RingError >
  {
    if slots == 0 { return Err( RingError ) }
    Ok( Capacity( slots ) )
  }
}
impl Cfg
{
  // the match form, with the two ::default() calls the real constructor uses
  pub const fn new( slots : usize ) -> Result< Cfg, RingError >
  {
    let capacity = match Capacity::new( slots )
    {
      Ok( capacity ) => capacity,
      Err( error ) => return Err( error ),
    };
    Ok( Cfg { capacity, wait : WaitKind::default(), overflow : OverflowPolicy::default() } )
  }
}
fn main() {}
RS
echo '  -- the match form, keeping the two ::default() calls the real constructor uses --'
rustc -O -A dead_code --crate-name const_match_default \
  -o /tmp/-const_match_default /tmp/-const_match_default.rs 2>&1 | command grep '^error'
```

Live output:

```
  -- the constructor body as written, with const added --
error[E0658]: `?` is not allowed on `Result<Capacity, RingError>` in constant functions
error: `Try` is not yet stable as a const trait
error[E0658]: `?` is not allowed on `Result<Cfg, RingError>` in constant functions
error: `FromResidual` is not yet stable as a const trait
error: aborting due to 4 previous errors
  -- the same constructor, written the way Capacity::new writes its own --
Cfg { capacity: Capacity(8), wait: Spin }
array length fixed at compile time from the constructor: 8
  -- the match form, keeping the two ::default() calls the real constructor uses --
error[E0015]: cannot call non-const associated function `<WaitKind as Default>::default` in constant functions
error[E0015]: cannot call non-const associated function `<OverflowPolicy as Default>::default` in constant functions
error: aborting due to 2 previous errors
```

**This section used to quote a scratch crate that no longer exists.** Its three
programs lived in `-cfg_probe/src/bin/`, a hyphen-prefixed directory outside any
workspace member, and the quoted compiler output named paths under it. That
directory has since been swept, so every claim here rested on a file nobody
could open and no gate could re-run — the recipe/quote gate reads `sh` blocks
with a `Live output:` beneath them and has no opinion at all about a bare fence.
The evidence is now inline: three programs written, compiled and read in the
block above, which regenerate on every gate run and fail loudly if the compiler
ever changes its mind about any of the three.

The `const` proof also got stronger in the move. The old probe bound the
constructor's result to a `const` item and printed a hand-written `true`
alongside it, which shows compile-time evaluation only to a reader who already
knows that a `const` item forces it. The replacement makes the array length
depend on the constructor's returned capacity, so the file cannot link at all
unless `Cfg::new` ran during compilation — and `PROOF.len()` printing `8` is
that fact, rather than an assertion about it.

---

### RC49 — Three Blockers, Not One, and the Third One Is a Tradeoff Rather Than a Fix

Eleven of the crate's twelve functions are `pub const fn`. The twelfth is `new`,
and the obvious explanation is the `?` on line `:71` — `?` desugars through the
`Try` trait, which is not yet const, so a constant function cannot use it. The
probe confirms that: `error[E0658]`, twice, once for each `Result` in the
expression.

That is not the whole answer. Rewriting the `?` as the `match` that
`ring_types::Capacity::new` uses does make the function `const` — the probe
takes the capacity out of the constructed value and uses it as an array length,
which the compiler cannot resolve unless the constructor already ran, and it
compiles. But putting the original body's other two lines back produces two more
errors of a different kind: `WaitKind::default()` and `OverflowPolicy::default()`
are `E0015`, non-const associated functions, and `Default::default` is a trait
method that no derive makes const.

**Finding.** So `new` is non-`const` for three independent reasons and the
documented one is the least interesting. `?` has a mechanical replacement with no
semantic cost. The two `::default()` calls do not: making them const means naming
the variants literally, `WaitKind::Spin` and `OverflowPolicy::DropNewest`, and
that trades a live coupling for a copy.

The coupling is real and the copy would be unguarded. Today, moving `#[ default ]`
to a different `WaitKind` variant changes what `RingConfig::new` produces, and
`defaults_are_the_documented_ones` fails, because it asserts the concrete variant.
With the literal in place the same move would leave the constructor on `Spin`, the
enum's default on something else, and the test still green — it pins the same
literal the constructor would then contain, so it would be comparing a copy to
itself.

Nothing in the crate records any of this. The constructor carries no note that it
is the one non-`const` function, no note of why, and no note that the cheapest
route to changing it costs a coupling the suite cannot watch.

---

### RC50 — The Technique Is Now Routine Elsewhere, Undocumented at Every Site

A census over every `src/` file under `/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/` finds thirteen `pub const fn`s
returning a `Result`. One is `ring_types::Capacity::new` — the function
`RingConfig::new` calls on line `:71`. The other twelve are in `exact_decimal`
and `exact_qty`, two crates with no dependency edge to this family in either
direction.

**Correction (2026-09-28):** "under `/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/`" is stale. The recipe above already
searches five roots — `/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/`, `/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/division/`, `ring/`, `/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/spike/` and
`/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/substrate/` — and labels itself accordingly; none of the thirteen hits are
under `/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/` itself. `ring_types::Capacity::new` is under `ring/`, and the
other twelve are under `/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/substrate/exact/`. The count and the split — one in
`ring_types`, twelve in `exact_decimal`/`exact_qty` — are unchanged; only this
sentence's root name is corrected to match the recipe it describes.

`Capacity::new`'s body is the workaround written out — two `if` blocks with
explicit `return Err( … )`, and a bare `Ok( Self( slots ) )` at the end. No `?`,
no combinator, nothing that reaches for `Try`. That shape is what allows a
fallible constructor to be evaluated at compile time, and `Capacity` gets the
benefit: a `const` capacity can be built in a constant. All twelve `exact_*`
constructors are written the same way, down to the `return Err` inside an `if`.

**When this was filed the census returned exactly one hit, and the finding was
that the family's sole demonstration of the technique sat one call away from the
one function that would use it.** That reading is now the weaker one. Twelve more
sites appeared, in a family that never consulted this one, arriving at the
identical shape independently — which says the `return Err` form is not a local
curiosity of `ring_types` but the ordinary way to write a fallible `const`
constructor in this workspace.

**Finding.** That makes `RingConfig::new`'s non-`const` form a choice rather than
a constraint, and sharpens what the choice costs. The `?` blocker has thirteen
worked examples of its replacement; what remains genuinely blocking is the pair of
`::default()` calls, which is the finding above and has nothing to do with `Try`.

And the documentation gap widened with the census. Not one of the thirteen sites
says why it is written this way: `Capacity::new`'s doc explains what it rejects
and why, not why it uses `return Err` instead of `?`, and the twelve `exact_*`
constructors carry no doc comment on the point at all — a search of both crates
for any mention of `const`, `?`, or `return Err` in a `///` or `//!` line returns
nothing. So a convention now exists at thirteen sites with a rationale at zero of
them. A maintainer tidying any one of them into `?` would silently make it
non-`const`, and nothing would fail — `const` is not load-bearing for any current
caller of `Capacity::new`, so the loss would be invisible until someone wanted it.

Recorded as a workaround rather than a decision because that is what the shape is:
`return Err` in place of `?` is not better code, it is the same code written
around a language limitation. The limitation is tracked upstream — the probe's own
error text cites rust-lang issue #143874 — so this is a workaround with an
expected end date, which is exactly the kind worth writing down before it is
forgotten and imitated as a convention.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`workaround/002`](002_a_lint_escalated_by_hand_in_every_crate.md) | The family's other standing workaround, and its scale |
| [`api/001`](../api/001_twelve_functions_eleven_of_them_const.md) | The eleven `const` functions this one is the exception to |
| [`decisions/001`](../decisions/001_capacity_is_the_one_parameter_that_is_not_a_with.md) | Why the constructor is the fallible one |
| [`algorithm/002`](../algorithm/002_one_fallible_path_and_it_is_not_this_crates.md) | The validation this constructor delegates |

### Sources

| Fact | Where |
|------|-------|
| Eleven `const fn`, one plain `fn` | Census above |
| The constructor's body and its three blockers | `ring_config/src/lib.rs:65-77` |
| `Capacity::new` written with `return Err` | `ring_types/src/capacity.rs:40-51` |
| Thirteen `const fn`s returning `Result` under the crate tree, twelve outside the family | Census above |
| No rationale for the form at any of the thirteen | `/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/substrate/exact/exact_decimal/src/lib.rs`, `/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/substrate/exact/exact_qty/src/lib.rs` — no `///` or `//!` line mentions it |
| The test pinning the two default variants | `ring_config/tests/config_test.rs:41-42` |
| E0658 for `?`, E0015 for `::default()`, and a `const` item proving the match form | Probes above |

### Tests

| Test | Covers |
|------|--------|
| `defaults_are_the_documented_ones` | The two variants a `const` rewrite would have to name literally |
| `capacity_is_validated_at_construction` | The fallibility that makes the workaround necessary |
| `every_named_field_is_carried` | The constructor's output, whichever form it takes |
