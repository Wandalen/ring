# Data Structure: Four Parallel Enums Over One Backend Choice

### Scope

- **Purpose**: Record that the backend choice is represented five times in this crate — once publicly and four times privately — and state what keeps the five in step.
- **Responsibility**: The five enums, why each exists separately, and what a sixth backend would cost.
- **In Scope**: `Backend`, `Storage`, `EndsInner`, `ProducerInner`, `ConsumerInner`.
- **Out of Scope**: The dispatch that reads them (→ [`../algorithm/001`](../algorithm/001_backend_dispatch_and_the_refusal_seam.md)); the storage enum's own variants (→ [`001`](001_three_way_storage_enum.md)).

### Five Enums, One Question

| Enum | Visibility | Holds | Why it cannot be `Storage` |
|------|-----------|-------|-----------------------------|
| `Backend` | **public** | nothing — a bare discriminant | It is the answer, not the thing |
| `Storage< T >` | private | The owned backend | — |
| `EndsInner< 'a, T >` | private | A borrow, plus `ring_mpsc::Ends` | `ring_mpsc` needs a two-step split, so this arm is not a reference |
| `ProducerInner< 'a, T >` | private | Each backend's producer handle | Three unrelated types with no shared trait |
| `ConsumerInner< 'a, T >` | private | Each backend's consumer handle | Same |

**The four private enums are not redundancy — they are one choice re-expressed
at each lifetime and ownership shape the surface passes through.** `Storage` owns;
`EndsInner` borrows mutably; `ProducerInner` and `ConsumerInner` hold handles that
outlive neither. Rust has no way to write "the same variant set, four ownership
shapes", so the crate writes it four times.

### What Keeps Them in Step

Nothing but exhaustive matching. Each enum is matched without a wildcard at
every use, so adding a backend is a compile error in five places rather than a
silent fallthrough in one:

```sh
cd "$(git rev-parse --show-toplevel)"/ring_core
printf 'enums:                '; grep -cE '^(pub )?enum ' src/lib.rs
printf 'wildcard match arms:  '; grep -cE '^ +_ =>' src/lib.rs
printf 'crossbeam cfg gates:  '; grep -c '#\[ cfg( feature = "crossbeam" ) \]' src/lib.rs
```

Live output:

```
enums:                5
wildcard match arms:  0
crossbeam cfg gates:  16
```

**No wildcard exists anywhere in the file**, which is a stronger statement than
this document could make when it was written. One did, and it was not on a
backend enum — it was on `ring_overflow::Resolution`, in `try_push`'s refusal
fold, and it was the one place a new *policy* rather than a new *backend* would
have passed unnoticed. CO1 named both remaining variants, so the count above is
the whole claim rather than four fifths of it
(→ [`../algorithm/001`](../algorithm/001_backend_dispatch_and_the_refusal_seam.md)).

### The Cost of a Sixth Backend

Five variant additions, sixteen `cfg` gates if it is optional — that is what
the crossbeam arm costs today — and, because every match is exhaustive, five
compile errors guiding the work. That is the good
case, and it is bought by writing the enum four times rather than reaching for a
trait object the backends cannot satisfy.

### CO11 — A Sixth Backend Costs Five Enum Edits and Sixteen Gates

The measurement is in the Regenerate block above: five enums, no wildcard arms,
sixteen `cfg` gates. Adding a backend means five variant additions and five
compile errors pointing at the matches that need arms. Making it optional means
gating each of those, which is where the sixteen comes from.

**The cost is real and it is the good kind** — paid at compile time, in the
right file, with the compiler naming each site. The alternative the module
documentation rejects, a trait object over the three backends, would trade these
five errors for a runtime dispatch and a shared method set the backends do not
have.

**Disposition:** declined — this instance's own text concludes the cost "is
the good kind", paid at compile time with the compiler naming each site, and
weighs favorably against the rejected trait-object alternative; the
measurement documents an accepted design tradeoff, not a defect in this
crate's own source or
`data_structure/002_four_parallel_enums_over_one_backend_choice.md`.

### CO12 — Nothing Checks That the Four Private Enums Agree

`Storage`, `EndsInner`, `ProducerInner` and `ConsumerInner` must carry the same
variant set or the dispatch stops being total — but nothing states that
requirement in code. A backend added to `Storage` and forgotten in
`ConsumerInner` is caught only because the `match` in `try_recv` would then be
non-exhaustive, which is a compile error about a match rather than about the
enums disagreeing.

That happens to be sufficient today, and it is sufficient by accident: it works
because every enum is matched somewhere. An enum variant added to all four but
matched in only three would compile if the fourth's match had a wildcard — the
shape the `Resolution` fold carried until CO1 removed it, which is the reason to
keep stating the requirement rather than reading the current zero as a property
(→ [`../algorithm/001`](../algorithm/001_backend_dispatch_and_the_refusal_seam.md), CO1).
