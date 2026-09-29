# Workaround: A Lint Turned Off With No Condition for Turning It Back On

### Scope

**Purpose:** Test the two claims the crate makes about its `clippy::result_large_err`
suppression — that the lint offers exactly two remedies, and that writing the
`allow` on `register` alone leaves a later oversized `Result` still failing the
build — and record that the suppression has no deletion condition and cannot
have one.

**Responsibility:** The `allow` and its `reason =`; the remedies enumerated
against the remedy that exists and is not enumerated; the narrow-scope guarantee,
measured; and what the `workaround/` definition asks for that this entry cannot
supply.

**In Scope:** `ring_registry/src/lib.rs:127`, `:130`, `:134`, `:156`;
`ring_registry/docs/decisions/readme.md:42`.

**Out of Scope:** The attribute as a declaration is
[`item/001`](../item/001_two_lints_one_allow_and_the_reason_beside_it.md). The
448 bytes it is about are
[`data_structure/001`](../data_structure/001_forty_eight_bytes_that_do_not_move.md).
The crate's other workaround is
[`workaround/001`](001_a_key_bought_back_from_the_map.md).

---

## The Suppression, Its Reasons, and Its Scope

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the suppression, and the reason written on it --'
command grep -n 'allow( clippy::result_large_err' ring_registry/src/lib.rs | cut -c1-92 | sed 's/^/    /' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
echo '  -- the remedies the crate enumerates, and how it counts them --'
command grep -n 'Both of the remedies the lint suggests\|Shrink the payload\|Box it\.' ring_registry/src/lib.rs ring_registry/docs/decisions/readme.md | cut -c1-92 | sed 's|ring_registry/||' | sed 's/^/    /' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
echo '  -- the guarantee the decisions file claims for the narrow scope --'
command grep -n 'never crate-wide' ring_registry/docs/decisions/readme.md | cut -c1-92 | sed 's/^/    decisions:/' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
echo '  -- and whether any deletion condition is recorded for it --'
printf '    lines in src/lib.rs naming a condition that would retire the allow: %s\n' \
  "$( command grep -rc -e 'until\|retire\|can be deleted\|revisit' ring_registry/src/lib.rs 2>/dev/null | awk -F: '{ s += $2 } END { print s + 0 }' )"
printf '    crate-wide allow anywhere in the crate: %s\n' \
  "$( command grep -c '^#!\[ allow' ring_registry/src/lib.rs || true )"
echo '  -- and whether the third lever is now named --'
command grep -n "^  /// A third lever" ring_registry/src/lib.rs | sed 's/^/    /' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
```

Live output:

```
  -- the suppression, and the reason written on it --
      #[ allow( clippy::result_large_err, reason = "the large payload is the caller's ring, 
  -- the remedies the crate enumerates, and how it counts them --
    src/lib.rs:  /// Both of the remedies the lint suggests are refused:
    src/lib.rs:  /// - *Shrink the payload.* The payload **is** the point
    src/lib.rs:  /// - *Box it.* That allocates on the failure path, to f
  -- the guarantee the decisions file claims for the narrow scope --
    decisions:`register` alone with a `reason =`, never crate-wide, so a different oversized
  -- and whether any deletion condition is recorded for it --
    lines in src/lib.rs naming a condition that would retire the allow: 0
    crate-wide allow anywhere in the crate: 0
  -- and whether the third lever is now named --
      /// A third lever, outside the lint's own suggestions, is not pulled either:
```

## The Narrow Scope, Tested

A two-function library crate against the real types — the first suppressed
exactly as `register` suppresses it, the second identical and bare:

```rust
// src/lib.rs
/// Suppressed the way `register` suppresses it.
#[ allow( clippy::result_large_err, reason = "the large payload is the caller's ring" ) ]
pub fn suppressed( ring : Split< u32 > ) -> Result< (), ( RegistryError, Split< u32 > ) >
{
  Err( ( RegistryError::NameTaken { name : "a".to_string() }, ring ) )
}

/// The same width, one function later, with no attribute.
pub fn unsuppressed( ring : Split< u32 > ) -> Result< (), ( RegistryError, Split< u32 > ) >
{
  Err( ( RegistryError::NameTaken { name : "a".to_string() }, ring ) )
}
```

```
warning: the `Err`-variant returned from this function is very large
  --> src/lib.rs:13:47
   |
13 | pub fn unsuppressed( ring : Split< u32 > ) -> Result< (), ( RegistryError, Split< u32 > ) >
   |                                               ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ the `Err`-variant is at least 448 bytes
   |
   = help: try reducing the size of `(ring_registry::RegistryError, ring_handle::Split<u32>)`, for example by boxing large elements or replacing it with `Box<(ring_registry::RegistryError, ring_handle::Split<u32>)>`
   = help: for further information visit https://rust-lang.github.io/rust-clippy/rust-1.97.0/index.html#result_large_err
   = note: `#[warn(clippy::result_large_err)]` on by default
```

## The Third Remedy

`clippy.toml` in the crate root, one key, with `cargo clippy` re-run against the
same two functions each time. The key's existence is established by the fact that
a bogus one is rejected — `error: error reading Clippy's configuration file:
unknown field \`not-a-real-clippy-key\``, exit 101 — while this one is accepted:

```
    large-error-threshold = 447   warnings: 1
    large-error-threshold = 448   warnings: 1
    large-error-threshold = 449   warnings: 0
    large-error-threshold = 512   warnings: 0
    no clippy.toml (default)      warnings: 1
```

---

### RG51 — "Both of the Lint's Suggested Remedies" Is Two of Three, and the Third Is the One a Crate Would Regret

`src/lib.rs:127` writes "Both of the lint's suggested remedies are refused" and
enumerates them: shrink the payload, box it. Each is refused with a reason that
holds, and the analysis is the strongest piece of argument in the crate.

The word doing the damage is *both*. The lint carries a configurable threshold,
`large-error-threshold` in `clippy.toml`, and it is a real key — a bogus one is
rejected outright with exit 101 and a list of the valid fields, while this one is
accepted silently. Measured against the same two functions, the boundary is
exact: at 448 the warning still fires, at 449 it stops. Setting it to 449 or
above silences the lint everywhere in the crate, for every function, forever, and
would have made the twenty lines of argument unnecessary.

That option is worse than the one taken, for exactly the reason the decisions
file gives for keeping the `allow` narrow. But "both" says the enumeration is
complete, and it is not — so a reader who later discovers the threshold has no
way to tell whether it was considered and rejected or simply not known.

**Finding.** Recorded as an enumeration stated as exhaustive that is not.
Two words of repair: "both of the lint's suggested remedies" becomes "both of the
remedies the lint suggests", which is true — the lint's own help text offers
shrinking and boxing, not the config key — and one clause naming the threshold as
a third lever deliberately not pulled turns an omission into a decision. The
crate already has the argument for refusing it, in the decisions file, applied to
a crate-wide `allow`; a crate-wide threshold is the same objection.

**Disposition:** applied — `register`'s doc comment now reads "Both of the
remedies the lint suggests are refused", so the enumeration no longer claims
completeness on its own terms, and a new paragraph names the third lever —
`large-error-threshold` in `clippy.toml` — declined for the same reason
`docs/decisions/readme.md` Closed 2 gives against a crate-wide `allow`.
Now prints: `A third lever, outside the lint's own suggestions, is not pulled either`

---

### RG52 — The Narrow Scope Does Hold, and the Suppression Still Has No Exit

`decisions/readme.md:42` claims a guarantee: the `allow` is written on `register`
alone, "never crate-wide, so a different oversized `Result` appearing later still
fails the build." Tested against the real types, it holds exactly. The suppressed
function produces nothing; the identical function one declaration later fires,
reports the same 448 bytes, and would fail a build run under `-D warnings`. The
census confirms the other half: zero crate-level `allow` attributes in the file.

That is the only claim in this crate's cost argument that survives measurement
unqualified, and it is worth saying so — the narrow scope was chosen for a stated
reason and the reason is true.

What the entry still lacks is the thing `ring_registry/docs/workaround/readme.md` promises for
every workaround: "the condition under which it can be deleted." Zero lines in
the source name one, and unlike
[`workaround/001`](001_a_key_bought_back_from_the_map.md) — whose clone a future
`OccupiedEntry::into_key` would retire — no upstream change deletes this one. The
lint fires because the `Err` variant is 448 bytes; the variant is 448 bytes
because it carries a ring; it carries a ring because Closed 2 decided it should.
The suppression is downstream of a permanent decision, so it is permanent.

**Finding.** Recorded as a category correction with a positive result attached.
The narrow-scope guarantee is real and tested. The suppression is not a
workaround in the sense the definition means — it absorbs no external constraint
awaiting an upstream fix — and the honest entry says its deletion condition is
"reversing Closed 2", which is to say none. Writing that down is what stops a
later reader from treating it as debt to be paid off, and it is the difference
between a suppression that was decided and one that was merely tolerated.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`workaround/001`](001_a_key_bought_back_from_the_map.md) | The crate's other workaround — this one has a deletion condition |
| [`item/001`](../item/001_two_lints_one_allow_and_the_reason_beside_it.md) | The attribute as a declaration, against the family's seven |
| [`decisions/001`](../decisions/001_four_closed_questions_and_the_one_measurement_none_took.md) | Closed 2, the decision this suppression is downstream of |
| [`data_structure/001`](../data_structure/001_forty_eight_bytes_that_do_not_move.md) | Why 448 is a constant rather than a floor |
| [`api/001`](../api/001_the_registry_surface.md) | The boxing alternative, priced |

### Sources

| Fact | Where |
|------|-------|
| The `allow` and its `reason =` | `ring_registry/src/lib.rs:156` |
| The third lever, named and declined | `ring_registry/src/lib.rs:139-142` |
| "Both of the lint's suggested remedies are refused" | `ring_registry/src/lib.rs:127` |
| The two remedies enumerated | `ring_registry/src/lib.rs:130`, `:134` |
| The narrow-scope guarantee | `ring_registry/docs/decisions/readme.md:42` |
| A sibling oversized `Result` still fires | Clippy run above |
| The threshold key, and the 448/449 boundary | Clippy runs above |
| No deletion condition, no crate-wide `allow` | Census above |

### Tests

| Test | Covers |
|------|--------|
| `a_refused_registration_hands_the_ring_back` | The payload that makes the `Result` 448 bytes |
| `a_second_registration_under_a_live_name_is_refused` | The method the `allow` is written on |
| `the_error_names_the_taken_name` | The other half of the oversized variant |
