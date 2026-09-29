# item

Twenty-four declarations: seven nouns and seventeen verbs. The catalogue splits
on that line, and both halves report the same asymmetry from opposite ends — the
nouns are almost entirely this crate's own invention, and the verbs are almost
entirely borrowed.

Six of the seven nouns exist nowhere else in the family, and seven of the
seventeen verbs are a single unchanged call into somebody else's container.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [Seven Nouns, Thirteen Variants, and One Measured Width](001_seven_nouns_thirteen_variants.md) | FL25, FL26 — what the vocabulary costs in bytes, and which of those costs anything asserts |
| 002 | [Seventeen Verbs That Never Touch the Producer](002_seventeen_verbs_that_never_touch_the_producer.md) | FL27, FL28 — the borrowed half, and the one dependency no public method can reach |

### The Twenty-Four, by Kind

The twenty-four are the seven `pub struct`/`pub enum` and the seventeen inherent
`pub fn`. The `Display` impl's `fmt` is reachable public behaviour but is a trait
method rather than a declaration of this crate's own, so it closes the table as
one row rather than counting toward the total.

| Kind | Declaration | Wraps | Notes |
|------|-------------|-------|-------|
| noun | `pub enum FlushPolicy` | — | 3 variants, one carrying `usize`. 16 bytes |
| noun | `pub enum FlushCause` | — | 4 variants, all unit. 1 byte |
| noun | `pub enum FlushOutcome` | — | 4 variants, two carrying `usize`. 16 bytes |
| noun | `pub enum ConfigError` | — | 2 variants. The only noun with hand-written `Display` and `Error` impls |
| noun | `pub struct FlushEntry` | — | 3 public fields, one of them a `FlushOutcome`. 40 bytes |
| noun | `pub struct FlushLog` | `Vec< FlushEntry >` | One private field. The only noun deriving `Default` |
| noun | `pub struct Flusher< 'a, T >` | `TlsBuffer< T >` + `Producer< 'a, T >` | Four private fields. The only noun deriving nothing but `Debug` |
| verb | `FlushEntry::count` | — | Derives a count from the stored outcome, so the two cannot disagree |
| verb | `FlushLog::new` | `Vec::new` | Forward |
| verb | `FlushLog::entries` | — | Returns the slice; the `Vec` itself is never exposed |
| verb | `FlushLog::len` | `Vec::len` | Forward |
| verb | `FlushLog::is_empty` | `Vec::is_empty` | Forward |
| verb | `FlushLog::clear` | `Vec::clear` | Forward |
| verb | `Flusher::new` | — | Validates, then binds buffer, producer and policy together |
| verb | `Flusher::with_log` | — | The only verb taking `self` by value |
| verb | `Flusher::policy` | — | Returns the bound policy by `Copy` |
| verb | `Flusher::staged` | `TlsBuffer::len` | Forward |
| verb | `Flusher::buffer_capacity` | `TlsBuffer::capacity` | Forward |
| verb | `Flusher::log` | — | `Option< &FlushLog >`; absent unless `with_log` was called |
| verb | `Flusher::clear_log` | `FlushLog::clear` | Forward, through an `Option` |
| verb | `Flusher::append` | `TlsBuffer::push` | Forward. **No flush** — the driven-not-self-firing commitment, at one line |
| verb | `Flusher::drive` | — | Evaluates the policy without announcing a barrier |
| verb | `Flusher::drive_at_barrier` | — | The only route by which `OnBarrier` can fire |
| verb | `Flusher::drain_final` | — | Ignores the policy. Once-only by convention, not by signature |
| verb | `ConfigError::fmt` | — | `impl Display`, added by the first outside consumer |

**`ring_core::Producer` is named in the noun table and in no verb row**, and
that is the catalogue's single most surprising line
(→ [`002`](002_seventeen_verbs_that_never_touch_the_producer.md)).

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the seven nouns --'
command grep -nE '^pub (struct|enum) ' ring_flush/src/lib.rs
echo '  -- the seventeen verbs --'
command grep -cE '^  pub (const )?fn ' ring_flush/src/lib.rs
echo '  -- the impl blocks --'
command grep -nE '^impl' ring_flush/src/lib.rs
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| FL25 | the four measured widths | n/a — coverage | `tests/manual/readme.md` measured four types and the suite asserts one of them, so three recorded widths would survive a layout change with nothing going red |
| FL26 | `ConfigError` | n/a — observation | Five of the seven nouns carry an identical five-trait derive list; the two that differ are the two with state, and the only hand-written impls in the crate belong to a noun that has none |
| FL27 | the seventeen verbs | n/a — observation | Seven of the seventeen forward a single call into a container this crate does not own, and the containers are two different crates' — `ring_tls`'s buffer and the standard library's `Vec` |
| FL28 | `Flusher::drain_final` | **latent hazard** | The one verb whose stated contract is "called once, at teardown" has a `&mut self` signature that permits any number of calls, and the crate's own test suite exercises the second call and asserts it succeeds |
