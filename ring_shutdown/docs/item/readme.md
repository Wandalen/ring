# Item Doc Definition

### Scope

- **Purpose**: The crate's declared surface counted item by item — what exists, what attributes each declaration carries, and which items make claims about each other that nothing checks.
- **Responsibility**: The twenty-three public functions, their `#[ must_use ]` / `const` / `# Errors` status, and the four of them whose relationships are asserted in prose rather than in code.
- **In Scope**: Every `pub fn` in `src/lib.rs`, across five inherent `impl` blocks and three free functions.
- **Out of Scope**: What the surface promises, which is [`../api/readme.md`](../api/readme.md)'s by-construction/by-convention grading; the derive attributes on the types the items hang off (→ [`../data_structure/002`](../data_structure/002_five_public_types_and_their_derive_sets.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [The Declared Surface and Its Attributes](001_the_declared_surface_and_its_attributes.md) | Twenty-two items, and which of the three declaration-site attributes each one carries | 🔄 |
| 002 | [Two Checks and Two Waiters](002_two_checks_and_two_waiters.md) | Four items in detail — the pair that spells one predicate twice, the pair that reports one event as two errors | 🔄 |

**The first document is a census; the second is a reading of four entries in
it.** They are separate because they go stale on different edits — `001` on any
declaration or attribute change anywhere in the crate, `002` only on a change to
`admit`, `Guarded`'s consulting sites, or either free waiter — and because a
reader who wants to know whether an item is `must_use` should not have to read
an argument about error vocabulary to find out.

The four findings split the same way, and the split is between two kinds of
gap. `001`'s are about **an attribute applied by the wrong question**: nine
items carried `#[ must_use ]` and the one method whose return value is genuinely
unrecoverable was not among them (SD25), while four operations report what they
did through a count nothing requires reading (SD26). `002`'s are about **a
claim made in prose and held nowhere else**: a documented equivalence between
two expressions the compiler cannot compare and no test compares either (SD27),
and two sibling waiters that turn one underlying failure into two different
errors, each with an honest justification and no rule between them (SD28).

Read together they describe a surface that is carefully specified per item and
unspecified across items. Every one of the twenty-three is documented — the crate
denies `missing_docs` and has no undocumented item — and the four findings all
live in the space between two declarations rather than inside any one of them.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_shutdown/docs/item
printf 'instances:                    %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:      %s\n' "$( command grep -hoE '^### SD[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:      %s\n' "$( command grep -coE '^\| SD[0-9]+ ' readme.md )"
printf 'each instance has a recipe:   %s\n' "$( command grep -lc '^### Regenerate' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'public functions:             %s\n' "$( command grep -cE '^ *pub (const )?fn ' ../../src/lib.rs )"
printf 'of those, must_use:           %s\n' "$( command grep -c '^ *#\[ must_use \]' ../../src/lib.rs )"
printf 'impl blocks:                  %s\n' "$( command grep -cE '^impl' ../../src/lib.rs )"
printf 'of those, trait impls:        %s\n' "$( command grep -cE '^impl.* for ' ../../src/lib.rs )"
printf 'free functions:               %s\n' "$( command grep -cE '^pub fn ' ../../src/lib.rs )"
printf 'items with no doc comment:    %s\n' "$( awk '/^ *\/\/\//{ d=1; next } /^ *#\[/{ next } /^ *\/\//{ next } /^ *pub (const )?fn /{ if ( !d ) c++; d=0; next } /^ *$/{ next } { d=0 } END{ print c+0 }' ../../src/lib.rs )"
```

Live output:

```
instances:                    2
finding headings inside:      4
rows in the table below:      4
each instance has a recipe:   2
public functions:             23
of those, must_use:           9
impl blocks:                  6
of those, trait impls:        1
free functions:               3
items with no doc comment:    0
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| SD25 | the payload-recovery method was the one value the compiler let you drop; it is `must_use` now | **latent hazard** | [`type/002`](../type/002_refusal_carries_the_record.md) argues the crate refuses a guarded push with a two-armed type carrying the record rather than with a `RingError`, so a record that could not be published comes back rather than being lost, and `Refusal::into_record` is the method performing that hand-back; nine of the crate's public functions carried `#[ must_use ]` and `into_record` was not one of them, so `refusal.into_record();` compiled silently and destroyed the record — the exact loss the type exists to prevent, reachable in one statement, in a crate whose `must_use` discipline covered every accessor and the constructor but not the one item returning a value that no longer exists anywhere else; the attribute is on it now, one of three added in the same pass that took the crate from nine to twelve. |
| SD26 | four operations report what they did in a return value nothing requires reading | n/a — unenforced | `Guarded::try_push_batch`, `Stopped::drain_all`, `Stopped::discard_all` and `reset` each return a count that is the sole channel through which the operation reports what happened — none fails, none takes an out-parameter, and a partial result is indistinguishable from a complete one at the call site — yet none of the four is `#[ must_use ]`; the consequence is worst for `try_push_batch`, which stops at the first refusal and leaves the caller having published an unknown prefix of its iterator, precisely the distinction the crate's own `a_closed_batch_push_consumes_nothing` test exists to preserve. |
| SD27 | a documented equivalence between two code paths that never meet | n/a — unenforced | `Shutdown::admit`'s doc comment calls it "the check [`Guarded`] performs, exposed for a caller who holds a raw producer", but `Guarded` never calls it — its three consulting sites read `self.shutdown.is_closed()` directly and `admit` has zero call sites in the crate; the two return different types (`Result< (), RingError >` against `Result< (), Refusal< T > >`) so the compiler cannot be asked whether they agree, and no test names `admit` and a guarded push in the same body, leaving an asserted equivalence that a second admission condition on either side would break with the doc unchanged and the suite green. |
| SD28 | one event, two errors, and the function with no ring returns "Empty" | n/a — inconsistency | Both free waiters call `ring_wait::wait_until`, whose sole failure is `Err( RingError::Empty )`; `wait_for_close` forwards it unchanged and `for_space_or_close` rewrites it to `RingError::Full` (matching `ring_wait::for_space`, which it re-implements rather than calls), so one crate turns one physical event into two errors with no stated rule for when a wrapper forwards its callee's error and when it translates it — and the forwarding side reports `Empty`, defined in `ring_types` as "the ring has no unread item", from the one item in the crate whose signature contains no ring at all. |
