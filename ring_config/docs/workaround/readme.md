# workaround

Two things in this crate are shaped by something outside it. One function is not
`const` because the language does not yet allow what its body does, and one
attribute is written by hand because a table three directory levels up carries the
wrong word. Neither is a decision about rings, and neither is recorded as a
workaround anywhere in the source.

They differ in how they end. The `const` limitation is tracked upstream and the
compiler's own error text names the issue number, so the workaround has an
expected expiry. The lint escalation has no expiry at all — it is a live choice
between two mechanisms, and a measured check shows the central one would carry it
today for free.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_the_question_mark_forecloses_const.md) | The Question Mark Forecloses `const` | Three blockers in one constructor, and the one `const fn` returning `Result` in the workspace |
| [002](002_a_lint_escalated_by_hand_in_every_crate.md) | A Lint Escalated by Hand in Every Crate | Sixty hand-written denies, and what the table entry would cost |

## Three Blockers Where the Documentation Implies One

`RingConfig::new` is the one function of twelve that is not `pub const fn`, and
the `?` on `Capacity::new( slots )?` is the visible reason — `?` desugars through
`Try`, which is not const, so a constant function cannot use it. Probing the body finds two more:
`WaitKind::default()` and `OverflowPolicy::default()` are non-const associated
functions, `E0015` each.

Only the first has a free fix. `ring_types::Capacity::new` — the function this
constructor calls — shows the technique: explicit `return Err( … )` instead of
`?`. Twelve more `pub const fn`s returning a `Result` now sit under the crate
tree, all in `exact_decimal` and `exact_qty` and all written the same way, so
the form is the workspace's ordinary one rather than this family's alone. The other two
blockers would have to become literal variants, which trades a live coupling to
`#[ default ]` for a copy that the existing test cannot tell from the original.

## An Escalation That Measured Free

`#![ deny( missing_docs ) ]` is `ring_config`'s only crate-level attribute, and
fifty-nine other crates carry the same line while all two hundred and
sixty-eight inherit `missing_docs = "warn"` from the workspace table. Whether
one word in that table could replace all sixty depends on the two hundred
and eight crates that never opted in, and the check settles it:
the whole workspace compiles clean under `-Dmissing_docs`, all features, exit
0.

So the lines are redundant today. What they are not is future-proof — a crate
added tomorrow without the line inherits `warn`, and so does the undocumented item
it ships.

**Correction (2026-09-28):** the numbers above — fifty-seven escalating by hand,
two hundred and twenty-six total — are what the census below printed when this
paragraph was written. A fresh run now prints sixty and two hundred and
sixty-eight; both moved because the workspace grew, not because of anything
about this crate. [`workaround/002`](002_a_lint_escalated_by_hand_in_every_crate.md)
tracks this count on an ongoing basis and has since stopped quoting the raw
total altogether, for the reason recorded there — this paragraph's numbers are
regenerated to match a fresh run, not independently re-derived against that
reasoning.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- const and non-const functions in this crate --'
command grep -c 'pub const fn' ring_config/src/lib.rs || true
command grep 'pub fn ' ring_config/src/lib.rs || true
echo '  -- the three things in its body a const fn cannot do --'
command grep 'Capacity::new( slots )?\|WaitKind::default()\|OverflowPolicy::default()' ring_config/src/lib.rs
echo '  -- every const fn returning a Result under the crate tree --'
command grep -r 'pub const fn [a-z_]*(.*) -> Result' --include=*.rs */src 
echo '  -- the lint table entry, and how many crates escalate it by hand --'
command grep 'missing_docs' Cargo.toml   # anchored: the members list above it grows
command grep -rl 'deny( missing_docs' --include=*.rs */src  | cut -d/ -f1 | sort -u | wc -l
ls -d */ | wc -l
```

Live output:

```
  -- const and non-const functions in this crate --
11
  pub fn new( slots : usize ) -> Result< Self, RingError >
  -- the three things in its body a const fn cannot do --
        capacity : Capacity::new( slots )?,
        wait : WaitKind::default(),
        overflow : OverflowPolicy::default(),
  -- every const fn returning a Result under the crate tree --
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
  -- the lint table entry, and how many crates escalate it by hand --
missing_docs = "warn"
60
268
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| RC49 | `ring_config` | n/a — doc gap | `RingConfig::new` is non-`const` for three independent reasons, not the one its `?` suggests — probes give `E0658` for the `?` and `E0015` for each of `WaitKind::default()` and `OverflowPolicy::default()` — and while the `?` has a mechanical replacement that a `const` item proves works, making the two defaults const means naming `Spin` and `DropNewest` literally, which trades a live coupling to `#[ default ]` for a copy that `defaults_are_the_documented_ones` could not detect diverging, since it pins the same literals |
| RC50 | `ring_types` | n/a — unenforced | Thirteen `pub const fn`s under the crate tree return a `Result`, all written with explicit `return Err( … )` — one is `ring_types::Capacity::new`, the function `RingConfig::new` calls, and twelve are in `exact_decimal` and `exact_qty`, which reached the same shape with no dependency edge to this family — so the form is the workspace's ordinary way to write a fallible `const` constructor rather than a local curiosity, and not one of the thirteen sites documents why it is written that way, leaving a convention with thirteen instances and zero rationales, any of which a maintainer could tidy into `?` and silently make non-`const` with nothing failing |
| RC51 | `ring_config` | n/a — duplication | Sixty of the two hundred and sixty-eight crates under the crate tree hand-write `#![ deny( missing_docs ) ]` while all two hundred and sixty-eight inherit `missing_docs = "warn"` from the workspace lints table, and a measured `RUSTFLAGS=-Dmissing_docs cargo check --workspace --all-features` exits 0 — with `cargo check -v` confirming the table's `--warn=missing_docs` is passed before the command line's `-D`, so the deny is genuinely in force — which makes the one-word table change free today and the sixty lines redundant, though nothing preserves that for a crate added tomorrow without the line |
| RC52 | `ring_config` | n/a — doc gap | Sixty crates are held to `deny` and two hundred and eight to `warn` with the split recorded nowhere but in the presence of one line per file, and although all thirty-three ring crates escalate — which looks like a family convention — no rulebook, readme or task file the crates cite states it, so the two real arguments for the per-crate line (visible where the file is edited; survives extraction from the workspace, where `workspace = true` has nothing to resolve against) go unmade and the line is indistinguishable from template boilerplate |
