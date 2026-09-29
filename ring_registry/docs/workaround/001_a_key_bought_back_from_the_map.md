# Workaround: A Key Bought Back From the Map

### Scope

**Purpose:** Record `occupied.key().clone()` as a workaround for a `std`
asymmetry rather than as a stylistic choice, establish by compilation that it is
forced, price it, and name the two conditions that would delete it.

**Responsibility:** The `String` moved into `entry()` and recovered by cloning;
what `VacantEntry` and `OccupiedEntry` each offer for getting a key back; what the
recovery costs against the same arm without it; and the clone-free form that
compiles today.

**In Scope:** `ring_registry/src/lib.rs:165`, `:167`, `:171`;
`ring_registry/docs/workaround/readme.md:4`.

**Out of Scope:** That the refusal path allocates twice rather than once is
[`algorithm/001`](../algorithm/001_two_branches_and_what_the_refusal_costs.md).
The suppressed lint, the crate's other workaround, is
[`workaround/002`](002_a_lint_turned_off_with_no_condition_for_turning_it_back_on.md).

---

## The Key, Given Away and Bought Back

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the key moved in, and the occupied arm that buys it back --'
sed -n '/^    let name = name\.into();$/p;/^    match self\.rings\.entry( name )$/p;/^      Entry::Occupied( occupied ) =>$/,/^        Err( ( RegistryError::NameTaken { name }, ring ) )$/p' ring_registry/src/lib.rs | sed 's/^/    /'
echo '  -- every line in src matching clone, case-insensitive --'
command grep -r -i 'clone' --include=*.rs ring_registry/src | sed 's|ring_registry/||' | sed 's/^/    /'
echo '  -- what the workaround definition promises for every entry --'
command grep '^cost it imposes' ring_registry/docs/workaround/readme.md | cut -c1-88 | sed 's/^/    /'
echo '  -- and whether register'"'"'s own doc now prices the clone --'
command grep '^  /// # The clone on the occupied arm' ring_registry/src/lib.rs | sed 's/^/    /'
```

Live output:

```
  -- the key moved in, and the occupied arm that buys it back --
        let name = name.into();
        match self.rings.entry( name )
          Entry::Occupied( occupied ) =>
          {
            let name = occupied.key().clone();
            Err( ( RegistryError::NameTaken { name }, ring ) )
  -- every line in src matching clone, case-insensitive --
    src/lib.rs:#[ derive( Debug, Clone, PartialEq, Eq ) ]
    src/lib.rs:  /// # The clone on the occupied arm
    src/lib.rs:  /// The refusal path clones the name back from `OccupiedEntry` — measured at
    src/lib.rs:        let name = occupied.key().clone();
  -- what the workaround definition promises for every entry --
    cost it imposes and the condition under which it can be deleted.
  -- and whether register's own doc now prices the clone --
      /// # The clone on the occupied arm
```

## What `std` Offers for Getting a Key Back

Three one-function library crates against the real `HashMap`, compiled with
`--emit=metadata`:

```rust
// compile/-into_key.rs — the method the occupied arm would want
match m.entry( name )
{
  Entry::Occupied( o ) => Some( o.into_key() ),
  Entry::Vacant( _ ) => None,
}

// compile/-key_recovery.rs — what each arm actually has
pub fn vacant_gives_it_back( m : &mut HashMap< String, u32 >, name : String ) -> Option< String >
{
  match m.entry( name )
  {
    Entry::Vacant( v ) => Some( v.into_key() ),
    Entry::Occupied( _ ) => None,
  }
}

// The occupied arm can too — but only by destroying the entry it was asked about.
pub fn occupied_gives_it_back_at_a_price( m : &mut HashMap< String, u32 >, name : String ) -> Option< String >
{
  match m.entry( name )
  {
    Entry::Occupied( o ) => Some( o.remove_entry().0 ),
    Entry::Vacant( _ ) => None,
  }
}
```

```
=== -into_key ===
error[E0599]: no method named `into_key` found for struct `std::collections::hash_map::OccupiedEntry<'a, K, V, A>` in the current scope
 --> compile/-into_key.rs:9:37
  |
9 |     Entry::Occupied( o ) => Some( o.into_key() ),
  |                                     ^^^^^^^^
  |
help: there is a method `into` with a similar name

error: aborting due to 1 previous error

=== -key_recovery ===
(no diagnostics)
```

## The Clone-Free Form, and What the Clone Costs

```rust
// compile/-clone_free_register.rs
pub fn register< T >( m : &mut HashMap< String, Split< T > >, name : String, ring : Split< T > )
-> Result< (), ( String, Split< T > ) >
{
  if m.contains_key( &name ) { return Err( ( name, ring ) ); }
  m.insert( name, ring );
  Ok( () )
}
```

```
=== -clone_free_register ===
(no diagnostics)
```

The occupied arm against the same arm with the recovery removed. The clone-free
variant is not a candidate implementation — it cannot name the taken name — it
exists only to isolate the price. Median of nine paired repetitions, both
variants run back to back inside each repetition:

```
=== run 1 ===
    occupied arm, key cloned back   median 74.04 ns/call  min 73.63  max 75.53
    occupied arm, no recovery       median 51.71 ns/call  min 51.64  max 51.80
    recovering the name costs       +22.33 ns/call, +43%
=== run 2 ===
    occupied arm, key cloned back   median 74.47 ns/call  min 74.15  max 83.86
    occupied arm, no recovery       median 51.81 ns/call  min 51.71  max 52.29
    recovering the name costs       +22.66 ns/call, +44%
```

---

### RG49 — The Clone Is Forced, and Nothing Says So

`register` takes `impl Into< String >`, materializes an owned `String` at `:152`,
and moves it into `self.rings.entry( name )` at `:154`. The occupied arm then
needs that name back, to put it in `RegistryError::NameTaken`, and buys it with
`occupied.key().clone()`.

The recovery is not a choice. `VacantEntry::into_key()` exists and compiles;
`OccupiedEntry` has no `into_key` at all — `error[E0599]`, with the compiler
suggesting `into` as the nearest name. The one method on `OccupiedEntry` that
yields the key by value is `remove_entry()`, which compiles and returns
`( String, V )` — at the price of removing the entry, which here means deleting
the already-registered ring and every unread record in it. That is precisely the
data loss [`pitfall/001`](../pitfall/001_insert_would_have_replaced_silently.md)
exists to prevent, arrived at from the other direction. So the clone is the only
non-destructive way out of the arm.

The source says none of this. `src/lib.rs:158` is the crate's only `.clone()`,
and the only other line matching "clone" is `RegistryError`'s derive. In a crate
that spends twenty lines refusing to allocate on the failure path — "*Box it.*
That allocates on the failure path" — an unexplained allocation on that same path
reads as an oversight rather than a forced move, and a reader with the
`Into< String >` signature in front of them has every reason to think the name
could simply have been kept.

**Finding.** Recorded as an unmarked workaround. One line above `:158` — that
`entry` consumes the key and `OccupiedEntry` offers no way to take it back
without removing the entry — converts a clone that looks avoidable into one a
reader can verify is not, and gives the `workaround/` definition the entry it
promises to carry for every constraint the crate absorbs.

---

### RG50 — It Costs 22 ns, Which Is Most of the Gap RG2 Found, and Two Things Would Delete It

The recovery costs 22.3–22.7 ns per refusal, 43–44% of the arm's total,
reproducing across two runs to within 0.4 ns. That number settles an attribution
[RG2](../algorithm/001_two_branches_and_what_the_refusal_costs.md) asserted
without isolating: the `Entry` form is 24–26 ns slower than
`contains_key`-then-`insert` on the occupied path, and 22 of those nanoseconds
are this clone. Almost none of the gap is the hash count the crate's own argument
names — with the recovery removed the `Entry` arm runs at 51.7 ns against the
alternative's 48.

`ring_registry/docs/workaround/readme.md:4` promises, for every entry, "the cost it imposes and
the condition under which it can be deleted". Both are available here:

| | |
|---|---|
| **Cost** | 22.3–22.7 ns and one `String` allocation, on the refusal path only |
| **Deleted by** | An `into_key` on `OccupiedEntry`, or any keyed-lookup entry API that borrows the key rather than consuming it |
| **Or by** | Accepting the `contains_key`-then-`insert` form, which compiles today with no clone — at the cost of a live `insert` call site |

The second escape is real and its price is exactly the argument `pitfall/001`
makes second and best: `insert` in the source is a call a later edit can reach,
and `Entry` removes it. Trading 22 ns on a setup-time call for that guarantee is
the right trade. It is a trade nobody has written down.

**Finding.** Recorded as a **measured cost** on a line the crate never mentions.
The twenty lines of cost argument in `register`'s doc are all about the
`Result`'s width — a size no caller pays for unless they refuse — and the one
construct on that path that costs measurable time is the clone, unremarked. The
crate has the numbers for both halves of what this definition asks for, and
records neither.

**Disposition:** applied — `register`'s doc comment now carries a "# The clone
on the occupied arm" section stating the measured cost (22–23 ns, 43–44% of the
arm) and cross-referencing this entry, which itself already carries both halves
this definition asks for: the Cost row above, and the Deleted-by/Or-by rows
naming the two conditions. The crate now records what it previously only
measured.
Now prints: `The clone on the occupied arm`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`workaround/002`](002_a_lint_turned_off_with_no_condition_for_turning_it_back_on.md) | The crate's other workaround, also with no deletion condition |
| [`algorithm/001`](../algorithm/001_two_branches_and_what_the_refusal_costs.md) | The two allocations, and the gap this measurement explains |
| [`pitfall/001`](../pitfall/001_insert_would_have_replaced_silently.md) | Why `insert` is not the escape it looks like |
| [`type/001`](../type/001_registry_error.md) | The `NameTaken { name }` this clone exists to fill |
| [`decisions/001`](../decisions/001_four_closed_questions_and_the_one_measurement_none_took.md) | Closed 1, the entry that chose `Entry` |

### Sources

| Fact | Where |
|------|-------|
| The key materialized and moved in | `ring_registry/src/lib.rs:165`, `:167` |
| The only `.clone()` in the crate | `ring_registry/src/lib.rs:171` |
| `OccupiedEntry` has no `into_key` | Compile probe above |
| `remove_entry` recovers it by deleting the ring | Compile probe above |
| The clone-free form compiles | Compile probe above |
| 22.3–22.7 ns, 43–44% of the arm | Probe above, two runs |
| The definition's promise | `ring_registry/docs/workaround/readme.md:4` |

### Tests

| Test | Covers |
|------|--------|
| `the_error_names_the_taken_name` | The name the clone exists to produce |
| `a_second_registration_under_a_live_name_is_refused` | The arm the clone is in |
| `a_refused_registration_does_not_drop_the_ring_already_there` | What `remove_entry` would have broken |
