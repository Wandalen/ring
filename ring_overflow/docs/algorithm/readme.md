# algorithm

The crate computes two functions from a three-variant enum to a small set of
constants, and nothing else. No loop, no arithmetic, no value read back after
being written. Both are exhaustive `match` blocks with no `_` arm, so the
compiler is what keeps them total.

What is worth reading is not the computation but where the two functions differ —
one statement — and what happens to their result at the one place a production
crate consumes it.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_two_mappings_over_three_policies.md) | Two Mappings Over Three Policies, Differing by One Line | Both `match` bodies, arm by arm, and the counter statement between them |
| [002](002_where_the_third_outcome_goes.md) | Where the Third Outcome Goes | The single consumer, its second arm, and which policies can actually arrive |

## One Statement of Difference

`resolve` and `would_resolve` have identical first and second arms and differ in
the third: `Resolution::Refused` against `Err( RingError::Full )`. Ahead of the
`match`, `resolve` executes `stats.record_drop( policy, 1 )` — unconditionally,
before the branch, so every policy is counted including the one that reports an
error rather than a loss.

That unconditional accounting is the crate's stated design: a stats read should
account for every full-ring event, not only the lossy ones. It is also what puts
a refusal into a counter named `dropped`, which the crate that owns the counter
then documents as items lost
([`ring_stats` § ST43](../../../ring_stats/docs/pitfall/002_the_total_that_counts_a_refusal_as_a_loss.md)).

## Three Outcomes Into One Bit

`Resolution` exists, in the module comment's own words, to make the alternative
outcomes explicit "rather than leaving them to a boolean." Its single production
consumer matches one variant by name, groups the other two in a second arm, and returns
`Result< (), T >`.

The fold is correct for that signature. What the census shows is that it is the
only place the type is spent: neither `lost_an_item` nor `accepted_incoming` has
a caller outside this crate's tests, and with `crossbeam` off by default the
`DropOldest` arm — the only source of `EvictedOldest` — cannot be reached at all.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the whole computation --'
command grep -m1 -B2 -A3 -F '    OverflowPolicy::DropNewest => Ok( Resolution::DroppedIncoming ),' ring_overflow/src/lib.rs
command grep -m1 -A7 -F 'pub const fn would_resolve( policy : OverflowPolicy ) -> Resolution' ring_overflow/src/lib.rs | tail -n 6
echo '  -- the statement between them --'
command grep -n 'record_drop' ring_overflow/src/lib.rs
echo '  -- and every production consumer --'
command grep -rn 'use ring_overflow' --include=*.rs */src/ | command grep -v '^ring_overflow/' | sed -E 's#^(/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/division/[^/]+|/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/substrate/[^/]+|module|ring|spike)/##'
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| OV1 | `ring_overflow` | n/a — observation | The crate's whole production computation is two exhaustive `match` blocks over a three-variant enum with no `_` arm and no loop, arithmetic or read-back anywhere, so `resolve` is `would_resolve` with one statement in front of it — `stats.record_drop( policy, 1 )` at `:199`, placed before the branch so that every policy is counted including the one that returns an error |
| OV2 | `ring_overflow` | n/a — coverage | The two mappings agree on `DropNewest` and `DropOldest` and diverge on `Fail`, and the suite asserts the agreement over the two-arm subset where it holds and the divergence in a separate test, with nothing stating that the two are halves of one property — where `resolve( p, &stats ).unwrap_or( Resolution::Refused ) == would_resolve( p )` holds across all three and is asserted nowhere |
| OV3 | `ring_core` | n/a — observation | `Resolution` is justified in the module comment as making outcomes explicit "rather than leaving them to a boolean", and its one production consumer matches `DroppedIncoming` by name, groups the other two in a second arm, and returns `Result< (), T >` — one bit plus the record, reached without either of the two predicates the type ships for reading it |
| OV4 | `ring_core` | **latent hazard** | `would_resolve` is only ever called with `DropNewest` or `Fail`: with `crossbeam` off by default `DropOldest` is rejected at construction by `Ring::new`, and with the feature on the crossbeam arm handles it with `force_push` and returns early — so `Resolution::EvictedOldest`, the one variant for which `accepted_incoming()` is true, has no production caller in either configuration, a fact nothing in `ring_overflow` recorded until the doc comment now at `Resolution::EvictedOldest` naming both builds |
