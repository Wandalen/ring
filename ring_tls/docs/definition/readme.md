# Doc Definitions

## Master Doc Definitions Table

| Type | Purpose | Master File | Instances |
|------|---------|-------------|----------:|
| `algorithm/` | The append and reset procedures — steps, branches, and the one growth allocation | [algorithm/readme.md](../algorithm/readme.md) | 2 |
| `api/` | The two caller surfaces — a synchronization-free writer, and a consolidator reading three primitives rather than one fused call | [api/readme.md](../api/readme.md) | 2 |
| `data_structure/` | The per-thread append log at its decided grain — identity and operations, and the `Vec` that was built instead | [data_structure/readme.md](../data_structure/readme.md) | 2 |
| `decisions/` | The two open rulings this crate cannot take alone — a corpus that specifies an unbuilt API, and a refusal path that destroys its payload | [decisions/readme.md](../decisions/readme.md) | 2 |
| `integration/` | Two dependencies, four conspicuous absences, and the export boundary this crate sits *on* rather than behind | [integration/readme.md](../integration/readme.md) | 2 |
| `invariant/` | The single-writer epoch discipline that makes zero-lock appends sound, and the allocation count that keeps it lock-free | [invariant/readme.md](../invariant/readme.md) | 2 |
| `item/` | The declared surface as a count — twenty-two items, and which crates reach each public name | [item/readme.md](../item/readme.md) | 2 |
| `lifecycle/` | A buffer's life bounded by its thread's, and the epoch and registration states it holds between consolidations | [lifecycle/readme.md](../lifecycle/readme.md) | 4 |
| `non_functional_requirement/` | The measured adoption gate, and the POD/alignment preconditions zero-copy needs | [non_functional_requirement/readme.md](../non_functional_requirement/readme.md) | 2 |
| `pattern/` | The ordering rule that closes the silent-loss window, and the two-stage composition this crate is half of | [pattern/readme.md](../pattern/readme.md) | 2 |
| `pitfall/` | Traps this crate's own vocabulary invites — readings that compile and mislead | [pitfall/readme.md](../pitfall/readme.md) | 2 |
| `type/` | The values the correctness arguments are written in — the tag that makes a region walkable, and the epoch that dates a snapshot | [type/readme.md](../type/readme.md) | 2 |
| `workaround/` | Two constraints the language imposes on a staging buffer — a leaked `std` type and an error variant that cannot carry a payload | [workaround/readme.md](../workaround/readme.md) | 2 |

## Master Doc Instances Table

| Definition | ID | Name | File |
|------------|----|------|------|
| `algorithm/` | 001 | Tagged-Record Bump Append | [001_tagged_record_bump_append.md](../algorithm/001_tagged_record_bump_append.md) |
| `algorithm/` | 002 | The Fused Claim and Drain | [002_the_fused_claim_and_drain.md](../algorithm/002_the_fused_claim_and_drain.md) |
| `api/` | 001 | Writer Append Surface | [001_writer_append_surface.md](../api/001_writer_append_surface.md) |
| `api/` | 002 | Consolidator Read Surface | [002_consolidator_read_surface.md](../api/002_consolidator_read_surface.md) |
| `data_structure/` | 001 | Thread-Local Append Log | [001_thread_local_append_log.md](../data_structure/001_thread_local_append_log.md) |
| `data_structure/` | 002 | A `Vec` and a Limit | [002_a_vec_and_a_limit.md](../data_structure/002_a_vec_and_a_limit.md) |
| `decisions/` | 001 | The Corpus Specifies an API the Crate Did Not Build | [001_the_corpus_specifies_an_api_the_crate_did_not_build.md](../decisions/001_the_corpus_specifies_an_api_the_crate_did_not_build.md) |
| `decisions/` | 002 | A Refused `push` Destroys the Item Its Doc Promises to Return | [002_a_refused_push_destroys_the_item_its_doc_promises_to_return.md](../decisions/002_a_refused_push_destroys_the_item_its_doc_promises_to_return.md) |
| `integration/` | 001 | Family Dependency Seam | [001_family_dependency_seam.md](../integration/001_family_dependency_seam.md) |
| `integration/` | 002 | Prospective Consumer Adoption | [002_prospective_consumer_adoption.md](../integration/002_prospective_consumer_adoption.md) |
| `invariant/` | 001 | Single-Writer Append | [001_single_writer_append.md](../invariant/001_single_writer_append.md) |
| `invariant/` | 002 | Zero Allocations in Steady State | [002_zero_allocations_in_steady_state.md](../invariant/002_zero_allocations_in_steady_state.md) |
| `item/` | 001 | Twenty-Two Items, and the Three Crates That Reach Them | [001_twenty_two_items_and_the_three_crates_that_reach_them.md](../item/001_twenty_two_items_and_the_three_crates_that_reach_them.md) |
| `item/` | 002 | No Published Constant, and a Caller-Supplied Ordering | [002_no_published_constant_and_a_caller_supplied_ordering.md](../item/002_no_published_constant_and_a_caller_supplied_ordering.md) |
| `lifecycle/` | 001 | Thread Registration and Teardown | [001_thread_registration_and_teardown.md](../lifecycle/001_thread_registration_and_teardown.md) |
| `lifecycle/` | 002 | Consolidation Cycle | [002_consolidation_cycle.md](../lifecycle/002_consolidation_cycle.md) |
| `lifecycle/` | 003 | Buffer Epoch Cycle | [003_buffer_epoch_cycle.md](../lifecycle/003_buffer_epoch_cycle.md) |
| `lifecycle/` | 004 | Registration State | [004_registration_state.md](../lifecycle/004_registration_state.md) |
| `non_functional_requirement/` | 001 | Measured Before Adopted | [001_measured_before_adopted.md](../non_functional_requirement/001_measured_before_adopted.md) |
| `non_functional_requirement/` | 002 | POD, Pointer-Free, Page-Aligned Payloads | [002_pod_pointer_free_payloads.md](../non_functional_requirement/002_pod_pointer_free_payloads.md) |
| `pattern/` | 001 | Register Before First Append | [001_register_before_first_append.md](../pattern/001_register_before_first_append.md) |
| `pattern/` | 002 | Staging Then Merge | [002_staging_then_merge.md](../pattern/002_staging_then_merge.md) |
| `pitfall/` | 001 | Implicit Thread-Locals Are Hidden Global State | [001_implicit_thread_locals_are_hidden_state.md](../pitfall/001_implicit_thread_locals_are_hidden_state.md) |
| `pitfall/` | 002 | A Flush Empties the Buffer Whether or Not It Is Read | [002_a_flush_empties_the_buffer_whether_or_not_it_is_read.md](../pitfall/002_a_flush_empties_the_buffer_whether_or_not_it_is_read.md) |
| `type/` | 001 | Record Tag | [001_record_tag.md](../type/001_record_tag.md) |
| `type/` | 002 | Epoch | [002_epoch.md](../type/002_epoch.md) |
| `workaround/` | 001 | The Only `std` Type in a Public Signature | [001_the_only_std_type_in_a_public_signature.md](../workaround/001_the_only_std_type_in_a_public_signature.md) |
| `workaround/` | 002 | A Unit Error Variant Cannot Carry the Refused Item | [002_a_unit_error_variant_cannot_carry_the_refused_item.md](../workaround/002_a_unit_error_variant_cannot_carry_the_refused_item.md) |

## Reading the Corpus

**Twenty-eight instances across thirteen definitions, and nineteen of them
describe a crate that was never built.**

That is not a figure of speech. The nineteen instances written before
implementation specify a per-thread byte region — records appended as a tag
byte plus little-endian operands behind a bump pointer, an epoch counter, a
thread registry, a `seal`/`drain`/`reset` read surface. `src/lib.rs` declares a
typed `Vec< T >` with a fused claim-and-drain and none of those eight names.
The nine instances added afterwards — under `item/`, `decisions/`,
`workaround/`, and the second slot of `algorithm/`, `data_structure/` and
`pitfall/` — document what exists.

**A reader must therefore check which crate an instance is about before
trusting it.** The divergence, its measurement, and the three ways out are
filed at
[`decisions/001`](../decisions/001_the_corpus_specifies_an_api_the_crate_did_not_build.md);
until that divergence is resolved, this paragraph is the warning label. The five
corpus checkers cannot supply one — none of them compares a doc claim against a
declaration, and an instance carrying no `sh` block at all passes the recipe
checker unexamined, which is exactly the shape all nineteen have.

The `invariant/` and `non_functional_requirement/` instances sit at the
**contract grain** this crate's own design decided; `algorithm/`,
`data_structure/`, `pattern/`, and `pitfall/` sit at the **mechanism grain**
that follows from this crate being a per-thread append log; `api/`,
`lifecycle/`, `type/`, and `integration/` sit at the
**surface grain** — what a caller touches, when, and across which crate
boundary. The surface grain is not decoration here the way it might be for an
internal crate: `ring_tls` is one of the family's five exported crates, so its
surface *is* the contract, and the open questions the `api/` instances leave
open are correspondingly more expensive than a hidden crate's
(→ [Family Dependency Seam](../integration/001_family_dependency_seam.md)).

No grain picks a winning candidate — that stays a future benchmark's
verdict, and each instance marks which of its own details the verdict still
owns (→ [`../readme.md`](../readme.md)).

No `format/` directory exists — the concrete buffer layout is undecided,
pending that same verdict; the instances above document the contract grain
this crate's own design decided. `algorithm/001` is the append *procedure*, which that
grain does fix, and it names the layout questions it deliberately leaves to
that future verdict rather than answering them early.

**`item/` now exists, and the argument that kept it absent is why.** The
earlier text here asked whether cataloguing items by hand adds anything rustdoc
does not compute from the same declarations, and answered *not yet*. Rustdoc
renders what is declared; it cannot report that eight names the corpus
specifies are not among them, or that two of the sixteen public items are
reachable trait methods a `pub`-keyed count files as private. Both are counts,
and both are the kind a generated page cannot produce because it has nothing to
compare against (→ [`item/001`](../item/001_twenty_two_items_and_the_three_crates_that_reach_them.md)).



## Findings

52, each argued in the instance named under **Where** and verified
there by a command whose output is quoted beneath it.

| ID | Finding | Subject | Reachable | Where |
|----|---------|---------|-----------|-------|
| TL1 | Every step of the append procedure specified here begins at a function `src/lib.rs` does not declare. | the append procedure | **wrong doc** | [algorithm/001](../algorithm/001_tagged_record_bump_append.md) |
| TL2 | Both the specified and the built procedure make growth unreachable rather than handling it, by different means. | the growth path | n/a — observation | [algorithm/001](../algorithm/001_tagged_record_bump_append.md) |
| TL3 | The one atomic a flush costs is inside `ring_batch::claim`; this crate's own source has none. | the flush cost | n/a — observation | [algorithm/002](../algorithm/002_the_fused_claim_and_drain.md) |
| TL4 | `Flush` derives each sequence with a hand-rolled `u64` counter while `BatchClaim::sequences()` returns exactly that range. | `Flush::next` | n/a — duplication | [algorithm/002](../algorithm/002_the_fused_claim_and_drain.md) |
| TL5 | `Flush` terminates when the drain runs out, never checking `claim.end()`, so a claim and a drain that disagree yield sequences nobody owns. | `Flush::next` | **latent hazard** | [algorithm/002](../algorithm/002_the_fused_claim_and_drain.md) |
| TL6 | `claim( cursor, 0, order )` issues a `fetch_add( 0 )`, so a poll loop over an idle buffer pays one atomic per iteration. | `flush_into` | **measured cost** | [algorithm/002](../algorithm/002_the_fused_claim_and_drain.md) |
| TL7 | Neither `append` nor `append_with` is declared; the exported writer surface is `push`, `is_full` and `len`. | the writer surface | **wrong doc** | [api/001](../api/001_writer_append_surface.md) |
| TL8 | The instance still presents three candidates for a question `src/lib.rs` settled, and `decisions/readme.md` still calls it the crate's one open trade-off. | the append signature | n/a — drift | [api/001](../api/001_writer_append_surface.md) |
| TL9 | The `seal`/`drain`/`reset` triple specified here was built as the single fused `flush_into`; only `drain` exists, added afterwards for `ring_flush`. | the read surface | **wrong doc** | [api/002](../api/002_consolidator_read_surface.md) |
| TL10 | `ring_flush` still narrates its steps as "seal, drain and reset" and files an algorithm instance under that name, for a sequence this crate cannot perform. | `ring_flush` | n/a — inconsistency | [api/002](../api/002_consolidator_read_surface.md) |
| TL11 | This instance specifies a byte region with a bump pointer; `TlsBuffer` declares `items : Vec< T >` and `limit : usize`. | `TlsBuffer` | **wrong doc** | [data_structure/001](../data_structure/001_thread_local_append_log.md) |
| TL12 | `TlsBuffer` is the one identifier common to the specified design and the built one, and it names a thread-local mechanism neither has. | `TlsBuffer` | n/a — observation | [data_structure/001](../data_structure/001_thread_local_append_log.md) |
| TL13 | `limit` is stored beside the `Vec` rather than read from `Vec::capacity`, so refusal cannot drift with the allocator's rounding. | `limit` | n/a — observation | [data_structure/002](../data_structure/002_a_vec_and_a_limit.md) |
| TL14 | Nothing but the `>=` in `push` keeps `Vec` from reallocating; a second insertion path added later would silently restore the growth the invariant forbids. | the growth path | **latent hazard** | [data_structure/002](../data_structure/002_a_vec_and_a_limit.md) |
| TL15 | A typed `Vec< T >` serves all three current consumers and cannot serve the walkable byte stream the specified design existed for. | `Vec< T >` | n/a — observation | [data_structure/002](../data_structure/002_a_vec_and_a_limit.md) |
| TL16 | `readme.md` opens by calling the crate a bump-allocated log with no atomics, then states four paragraphs later that `TlsBuffer< T >` is a `Vec< T >`. | `readme.md` | n/a — inconsistency | [data_structure/002](../data_structure/002_a_vec_and_a_limit.md) |
| TL17 | The pre-implementation corpus and `src/lib.rs` share no function name at all. | the corpus | **wrong doc** | [decisions/001](../decisions/001_the_corpus_specifies_an_api_the_crate_did_not_build.md) |
| TL18 | `recipes.py` compares an instance against reality only where the instance publishes a `sh` block, and not one of the nineteen bodies does. | the corpus checkers | n/a — coverage | [decisions/001](../decisions/001_the_corpus_specifies_an_api_the_crate_did_not_build.md) |
| TL19 | `ring_flush/src/lib.rs` compares both designs and explains why the built one failed its requirement; `ring_tls`'s own corpus says nothing. | `ring_flush` | n/a — doc gap | [decisions/001](../decisions/001_the_corpus_specifies_an_api_the_crate_did_not_build.md) |
| TL20 | `TlsBuffer::drain` exists because `ring_flush` could not use `flush_into`, and only `ring_flush` records why. | `drain` | n/a — doc gap | [decisions/001](../decisions/001_the_corpus_specifies_an_api_the_crate_did_not_build.md) |
| TL21 | `push` states that a refused item "is returned to the caller by never being taken", and `item : T` is moved in and dropped on the refusal path. | `push` | **wrong doc** | [decisions/002](../decisions/002_a_refused_push_destroys_the_item_its_doc_promises_to_return.md) |
| TL22 | All three consumers stage `Copy` types, so the drop is invisible today; a `T` owning a resource loses one per refused push. | the refusal path | **latent hazard** | [decisions/002](../decisions/002_a_refused_push_destroys_the_item_its_doc_promises_to_return.md) |
| TL23 | `ring_registry::register` returns `Result< (), ( RegistryError, Split< T > ) >`, handing the payload back in the error tuple. | `ring_registry` | n/a — unadopted | [decisions/002](../decisions/002_a_refused_push_destroys_the_item_its_doc_promises_to_return.md) |
| TL24 | The instance describes two dependencies; the manifest declares three, plus three dev-dependencies. | the dependency seam | n/a — drift | [integration/001](../integration/001_family_dependency_seam.md) |
| TL25 | `ring_claim`, `ring_publish`, `ring_consume` and `ring_gating` are still not dependencies — not by the design this instance argues from, but because `ring_batch::claim` subsumes the one step this crate takes. | the absent dependencies | n/a — observation | [integration/001](../integration/001_family_dependency_seam.md) |
| TL26 | The instance projects adoption by a specific named consumer; the three crates that actually declare the dependency are `ring_flush`, `ring_bench` and `ring_testkit`. | the consumer set | n/a — drift | [integration/002](../integration/002_prospective_consumer_adoption.md) |
| TL27 | Every crate naming `ring_tls` in code also declares it, and one crate names it only in a comment saying it was removed. | the reach picture | n/a — observation | [integration/002](../integration/002_prospective_consumer_adoption.md) |
| TL28 | Every mutating method takes `&mut self`, so the borrow checker enforces the invariant this instance argues for by convention. | the single-writer rule | n/a — observation | [invariant/001](../invariant/001_single_writer_append.md) |
| TL29 | The soundness argument turns on a per-buffer epoch counter; no epoch exists in the crate or anywhere in the family. | the epoch discipline | **wrong doc** | [invariant/001](../invariant/001_single_writer_append.md) |
| TL30 | Zero allocations in steady state holds for `Vec::with_capacity` plus a refusing `push`, by a different mechanism than the one specified. | the allocation count | n/a — observation | [invariant/002](../invariant/002_zero_allocations_in_steady_state.md) |
| TL31 | This crate's own reached-test asserts zero allocations "by a counting allocator", and `ring_tls` carries no `#[ global_allocator ]` to do the asserting — four sibling crates do. | the test coverage | n/a — coverage | [invariant/002](../invariant/002_zero_allocations_in_steady_state.md) |
| TL32 | `Iterator::next` and `size_hint` on `Flush` are reachable by every caller and spelled `fn`, so a census keyed on the `pub` token miscounts them. | the census | n/a — observation | [item/001](../item/001_twenty_two_items_and_the_three_crates_that_reach_them.md) |
| TL33 | `TlsBuffer` and `with_capacity` are named by all three consumers; `Flush` is never spelled anywhere outside this crate. | the reach | n/a — observation | [item/001](../item/001_twenty_two_items_and_the_three_crates_that_reach_them.md) |
| TL34 | The scan filters TOML comments with `^\s*#` as well as Rust comments, because one manifest names this crate only to record its removal. | the reach scan | n/a — diagnostics | [item/001](../item/001_twenty_two_items_and_the_three_crates_that_reach_them.md) |
| TL35 | `ring_spsc` publishes two ordering constants and `ring_mpsc` four; this crate publishes none and takes an `Ordering` argument. | the ordering surface | n/a — inconsistency | [item/002](../item/002_no_published_constant_and_a_caller_supplied_ordering.md) |
| TL36 | `flush_into` accepts any `Ordering`; a `Relaxed` claim allocates correct sequences and publishes nothing, failing in a consumer rather than here. | `order` | **latent hazard** | [item/002](../item/002_no_published_constant_and_a_caller_supplied_ordering.md) |
| TL37 | A buffer's life is `with_capacity` to drop; no `register`, no thread-exit hook, and no registry to leave. | the buffer lifecycle | **wrong doc** | [lifecycle/001](../lifecycle/001_thread_registration_and_teardown.md) |
| TL38 | The cycle specified here is driven by a `consolidate_all` walking a thread registry; the built equivalent is a caller calling `flush_into`. | the consolidation cycle | **wrong doc** | [lifecycle/002](../lifecycle/002_consolidation_cycle.md) |
| TL39 | The states are transitions of an epoch counter; `TlsBuffer` has two fields and neither is one. | the epoch cycle | **wrong doc** | [lifecycle/003](../lifecycle/003_buffer_epoch_cycle.md) |
| TL40 | Unregistered/registered/deregistered collapses to "exists", because construction is the only entry and drop the only exit. | the registration state | **wrong doc** | [lifecycle/004](../lifecycle/004_registration_state.md) |
| TL41 | `ring_bench` measures the built `TlsBuffer` against the requirement written for the byte region, and the requirement is layout-neutral enough to apply. | the adoption gate | n/a — observation | [non_functional_requirement/001](../non_functional_requirement/001_measured_before_adopted.md) |
| TL42 | `TlsBuffer< T >` has no bound on `T`, so the POD, pointer-free and alignment requirements stated here are documentation only. | the payload preconditions | n/a — unenforced | [non_functional_requirement/002](../non_functional_requirement/002_pod_pointer_free_payloads.md) |
| TL43 | "Register before first append" closes a silent-loss window that cannot open when construction is the only entry point. | the ordering rule | **misleading doc** | [pattern/001](../pattern/001_register_before_first_append.md) |
| TL44 | Stage locally, merge under one atomic — true of the byte region and true of `flush_into`, with the merge in a different crate. | the staging pattern | n/a — observation | [pattern/002](../pattern/002_staging_then_merge.md) |
| TL45 | This instance argues against implicit `thread_local!` storage, and the crate contains none — nor does any of the thirty-three. | the hidden-state trap | n/a — observation | [pitfall/001](../pitfall/001_implicit_thread_locals_are_hidden_state.md) |
| TL46 | Dropping a `Flush` after two of sixty-four pairs leaves sixty-two claimed sequences owned by nothing, and the buffer empty either way. | `Flush` | **measured cost** | [pitfall/002](../pitfall/002_a_flush_empties_the_buffer_whether_or_not_it_is_read.md) |
| TL47 | `drain` also empties when its iterator is dropped unread, but advances no cursor — which is exactly why `ring_flush` needed it. | `drain` | n/a — observation | [pitfall/002](../pitfall/002_a_flush_empties_the_buffer_whether_or_not_it_is_read.md) |
| TL48 | A one-byte discriminant preceding each record is defined here and appears nowhere in the family's code. | the record tag | **wrong doc** | [type/001](../type/001_record_tag.md) |
| TL49 | The epoch that dates a consolidation snapshot has no declaration, and there is no consolidation to date. | the epoch | **wrong doc** | [type/002](../type/002_epoch.md) |
| TL50 | `pub fn drain( &mut self ) -> std::vec::Drain< '_, T >` is the family's only `std` type in a public signature. | `drain` | n/a — observation | [workaround/001](../workaround/001_the_only_std_type_in_a_public_signature.md) |
| TL51 | `Drain` puts `Vec` in the public contract, so changing the storage is breaking even though `drain`'s meaning would not change. | `Vec` | **latent hazard** | [workaround/001](../workaround/001_the_only_std_type_in_a_public_signature.md) |
| TL52 | Two of nine `RingError` variants carry data, both a number the caller passed in, so the constraint is specific to `Full` rather than a property of the enum. | `RingError` | n/a — observation | [workaround/002](../workaround/002_a_unit_error_variant_cannot_carry_the_refused_item.md) |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_tls/docs
printf 'doc definitions:          '; ls -d */ | grep -vc '^definition/'
printf 'instances:                '; ls */[0-9][0-9][0-9]_*.md | wc -l
printf 'findings in the corpus:   '; grep -rhoE '^### TL[0-9]+ ' */[0-9][0-9][0-9]_*.md | wc -l
printf 'rows in the table below:  '; grep -coE '^\| TL[0-9]+ ' definition/readme.md
# doc definitions:          13
# instances:                28
# findings in the corpus:   52
# rows in the table below:  52
```
