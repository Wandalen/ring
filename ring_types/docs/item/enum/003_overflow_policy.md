# OverflowPolicy

## Representation

What a producer does when the ring has no free slot — a closed, three-variant,
`Copy` set. `DropNewest` discards the item being published, `DropOldest` evicts
the oldest unread item, `Fail` publishes nothing and returns
[`RingError::Full`](002_ring_error.md).

**The interesting property is the fourth variant that does not exist.**
This enum is explicit that there is no variant that overwrites unread data, and
the absence is deliberate: a publish that reports success has kept the item,
under every policy here. `DropOldest`
looks like the exception and is not — it evicts a *different* item and says so,
rather than silently overwriting the one it was handed.

**One variant is configurable, refused, and supported, in that order.**
`ring_config` accepts `DropOldest`; `ring_core` rejects it at construction on the
default backend (`src/lib.rs:152-155`, returning `PolicyUnsupported`); the
`crossbeam` backend honours it via `force_push` (`src/lib.rs:379-384`). Which of
the three a caller meets depends on a feature flag
(→ [`../../lifecycle/004`](../../lifecycle/004_an_overflow_policy_from_declaration_to_refusal.md)).

## Kind

Enum (§ Item Kind Taxonomy : Stable Item Kinds #7)

## Definition

`ring_types/src/policy.rs:105`

```rust
#[ derive( Debug, Clone, Copy, PartialEq, Eq, Hash, Default ) ]
pub enum OverflowPolicy
{
  #[ default ]
  DropNewest,
  DropOldest,
  Fail,
}
```

`DropNewest` is the default because it is the only variant that is always
implementable: `Fail` pushes a decision onto a caller who may have none to make,
and `DropOldest` is the one the default backend refuses.

## File Usage

| File | Line(s) | Context |
|------|---------|---------|
| `ring_types/src/policy.rs` | 99-102, 105, 116, 128-129, 137-138, 166-168 | Type doc example (99-102); **definition (105**, variants to 113**)**; inherent `impl` header (116); `ALL`'s doc example (128-129); `reports_failure`'s doc example (137-138); `drops_silently`'s doc example (166-168) |
| `ring_types/src/error.rs` | 52-53, 72, 75 | `Full`'s doc naming the `Fail` policy (52-53); `PolicyUnsupported`'s doc naming the constraint and `DropOldest` as its concrete case (72, 75) |
| `ring_types/src/lib.rs` | 12, 20, 44 | Discriminant/handler-split prose (12); module responsibility table (20); **re-export (44)** |

**The four `error.rs` lines are the crate's only prose coupling between two of its
modules**, and they run the opposite way to its only code coupling: `capacity`
imports from `error`, while `error`'s documentation reaches across to `policy`
without importing anything. Both are doc-comment intra-links (`[`crate::OverflowPolicy`]`),
so they cost nothing at compile time. They are checked, but not by anything in
this repository:

```bash
RUSTDOCFLAGS="-D warnings" cargo doc -p ring_types --no-deps
```

passes today, and `rustdoc::broken_intra_doc_links` is warn-by-default, so that
flag turns a renamed type into a build failure. **Exactly one of the 27 gate
scripts under `bench_harness/gate/` sets `RUSTDOCFLAGS`** —
`grep -rn RUSTDOCFLAGS bench_harness/gate/` returns 1 line, `g2_docs.sh:91`,
which applies `-D warnings` to `cargo test --doc` rather than to a doc build.
No gate runs a doc build at all: `grep -rn 'cargo doc' bench_harness/gate/`
returns nothing. So the command above lives in the operator's verification level,
not in the family's own gates — the conclusion an earlier form of this note drew,
but from two wrong figures (it claimed none of six scripts set the flag). The line
break had fallen inside the quoted command, which kept the claim from ever being
re-run.

**The shape of the claim survived; both its numbers moved anyway.** Six scripts
became nineteen became twenty-seven, and the one hit slid from `:86` to `:91` —
the ratio the sentence exists to state (one gate of many, applied to doctests and
not to a doc build) is what stayed true, and it is the only part worth asserting.
A count of files in a growing directory, and a line number in an edited script,
are two of the shortest-lived facts a document can hold.

Test-only references: `ring_types` — 14 `OverflowPolicy::` paths in
`tests/types_test.rs` bodies, across three tests: `overflow_policy_has_no_overwrite_variant`
(`:198`, nine — the length, the three-variant `contains` loop, and the
wildcard-free `match`), `overflow_policies_partition_by_reporting` (`:220`, three),
and `overflow_policy_defaults_to_drop_newest` (`:239`, two), plus one more in a
test's own doc comment (`:191`) for 15 in the file. Then 12 consumer crates' suites. The source set is
ten crates and the test set is twelve: `ring_flush` and `ring_spsc` name it in
`tests/` without naming it in `src/`.

## Crate Usage

**Contact** is measured, not inferred from the crate appearing in a grep: *code*
means the source names the type outside a doc comment, *value* means the policy
flows through without the type ever being named, *prose* means every occurrence
in `src/` is inside a `//`, `///` or `//!`.

| Crate | Via File | Contact | Purpose |
|-------|----------|---------|---------|
| `ring_types` | `src/policy.rs`, `src/error.rs`, `src/lib.rs` | code | Defining crate — declares the discriminants |
| `ring_config` | `src/lib.rs` | code | Carries the configured policy on a `RingConfig` — the field (`:46`), `with_overflow` (`:104`), `overflow()` (`:180`) |
| `ring_core` | `src/lib.rs` | code | **Refuses `DropOldest`** on the default backend (`:154-157`); honours it on `crossbeam` via `force_push` (`:388-390`) |
| `ring_overflow` | `src/lib.rs` | code | **The handler** — `resolve` (`:192`) and `would_resolve` (`:229`) dispatch on it through wildcard-free `match`es |
| `ring_stats` | `src/lib.rs` | code | **A third dispatcher, and the only one that dispatches twice** — `record_drop` (`:285`) and `dropped` (`:341`) each select one of three counters by the same wildcard-free `match`, one to write it and one to read it |
| `ring_bench` | `src/lib.rs` | value | Forwards the *configured* policy twice — into `record_drop` (`:961`) and a `Debug` line (`:1396`). Names the type only in prose |
| `ring_factory` | `src/lib.rs` | prose | Takes a whole `RingConfig` (`build`, `:150`); the setter is `ring_config`'s, re-exported at `:104` |
| `ring_handle` | `src/lib.rs` | prose | Doc comments describe what the ring's policy does (`:129`, `:137`) |
| `ring_poll` | `src/lib.rs` | prose | One doc comment on `DropNewest`'s effect (`:238`) |
| `ring_shutdown` | `src/lib.rs` | prose | Two doc comments on reachability under `DropNewest` (`:411`, `:455`) |
| `ring_testkit` | `src/lib.rs` | prose | Doc comments describe policy-dependent outcomes (`:177`, `:548`) |

**Six of the ten consumers do not reach this type from code at all, and an
earlier form of this table said all ten did.** Its verbs were active and specific
— `ring_bench` "sweeps policies as a benchmark axis", `ring_factory` "a builder
setter takes it", `ring_handle` "reports the policy a handle was built with",
`ring_poll` "chooses poll behaviour under a full ring", `ring_shutdown` "decides
what a close does with pending items", `ring_testkit` "builds fixtures across all
three policies" — and each names a mechanism that does not exist in that crate's
source. `ring_handle` has eleven `pub fn` and none returns a policy. `ring_poll`
mentions the type once, in a doc comment. `ring_bench` never sweeps: it reads the
one configured policy off the workload and forwards it.

**`ring_testkit`'s row was the inverse of its own module doc.** The row said
fixtures are built "across all three policies"; `src/lib.rs:35` says the fixture
runs "only in that policy" — singular, `DropNewest`, named two lines earlier. The
claim did not merely lack support in the crate, it contradicted the sentence the
crate leads with.

The mechanism is the one this crate's `ALL` constants already demonstrate
(→ [`../../non_functional_requirement/002`](../../non_functional_requirement/002_the_enum_sets_are_closed_and_asserted.md)):
a name appearing in a file is evidence the file *mentions* something, and it was
read as evidence the file *does* something. A grep for `OverflowPolicy` returns
all ten crates. Only the four `code` rows survive `grep -v` on comment lines, and
`ring_bench` survives only a second, different search — for the value's path
(`.overflow()`) rather than the type's name. Three searches, three different
answers, and the table had recorded the widest one.

**`ring_stats` is a dispatcher the discriminant/handler split does not account
for.** The split names `ring_overflow` as the handler; `ring_stats` matches on the same
discriminant twice — `record_drop` (`:285`) to pick the counter to increment,
`dropped` (`:341`) to pick the one to read. Neither is a strategy; they attribute
a drop rather than deciding one. But it is a third crate that must be edited when
a variant is added, and no rule says so
(→ [`../../pattern/001`](../../pattern/001_discriminants_here_handlers_elsewhere.md)).

**The declared handler is the one nothing calls.** `ring_overflow::resolve` — the
function that both resolves *and* records — has zero callers outside its own
crate; `ring_core` took the pure `would_resolve` instead (`src/lib.rs:69` imports
it, `:400` calls it) and `ring_bench` populates the counters afterwards from
offered-minus-received. So the family's drop accounting is written by the
measuring layer, never by the dropping layer.

**Which makes the compile lever narrower than the crate count suggests.** Adding a
fourth variant breaks exactly three files and five `match`es: `ring_overflow`'s two
(`:202-204`, `:233-235`), `ring_stats`' two (`:289-291`, `:345-347`), and this
crate's own `overflow_policy_has_no_overwrite_variant` (`tests/types_test.rs:212`).
`ring_core` compares with `==` and keeps compiling.

**Four of the five dispatch; the fifth exists only to break.** The production
`match`es each give their three variants three different answers. The test's is one
arm — `DropNewest | DropOldest | Fail => {}` — binding nothing and doing nothing in
every case. An or-pattern arm that discards its input has no runtime behaviour at
all to get wrong; its whole value is the compile error it raises the day the set
grows. It is a static assertion wearing a `match`'s syntax, and it is the only one
of the five whose purpose is exactly that.

**The count of files held and the count of sites did not, and the miss was on the
side that matters.** An earlier form of this paragraph named one `ring_stats`
`match` where there are two, having stopped at `record_drop` because that is the
function the row above it already discussed. The second is `dropped` (`:341`) —
the one [`dropped_total`](../associated_constant/003_overflow_policy_all.md) calls
on every element while summing over `ALL` (`:378`). So the crate's single
production `ALL` sweep runs through a wildcard-free `match` on each iteration, and
a fourth variant stops that sweep at compile time rather than letting it silently
sum three of four. That is the strongest link in the whole lever, and the sentence
written to describe the lever had left it out. The six `prose` crates keep compiling too, and
nothing there would fail either. Exactly one of their doc comments quantifies over
the whole set — `ring_testkit/src/lib.rs:276`, "Impossible within one run under
every policy" — and that one silently becomes an unchecked claim about a variant
its author never saw. The other five name specific variants, so a fourth leaves
them incomplete rather than false, which is the quieter of the two failures. Either
way the `match`es defend the *code* against a new variant and nothing defends the
*prose*.
