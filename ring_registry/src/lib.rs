//! Named ring registry.
//!
//! One of the 33 crates of the ring family, which implements the concurrency write-path.
//!
//! # What it is
//!
//! A name-to-ring map that owns the rings it holds. One name, one ring, and a
//! name that is taken cannot be taken again until it is given up.
//!
//! ```
//! use ring_config::RingConfig;
//! use ring_core::Ring;
//! use ring_handle::Split;
//! use ring_registry::Registry;
//!
//! let config = RingConfig::new( 16 ).unwrap();
//! let mut registry = Registry::new();
//!
//! let ring : Ring< u32 > = Ring::new( &config ).unwrap();
//! registry.register( "events", Split::new( ring ) ).unwrap();
//!
//! assert!( registry.get_mut( "events" ).is_some() );
//! assert!( registry.get_mut( "telemetry" ).is_none(), "and by no other name" );
//!
//! let ring : Ring< u32 > = Ring::new( &config ).unwrap();
//! assert!( registry.register( "events", Split::new( ring ) ).is_err() );
//! ```
//!
//! # Two things it deliberately does not do
//!
//! **It is generic over one record type.** A registry holding rings of
//! different `T` would need type erasure and downcasting at every retrieval,
//! and nothing asks for it. `ring_factory`, the only consumer, builds one ring
//! at a time. → `docs/decisions/readme.md` Pending 1.
//!
//! **It offers no shared retrieval.** Everything a caller can do with a ring
//! needs `&mut`, so retrieval is `get_mut` and there is no immutable `get`.
//! → `docs/api/001_the_registry_surface.md`.

#![deny(missing_docs)]

use core::fmt;
use std::collections::HashMap;
use std::collections::hash_map::Entry;

use ring_handle::Split;

/// The one failure registering can produce.
///
/// A single-variant enum rather than a unit struct, because a name conflict is
/// the only thing that can go wrong in the registry's public API *today*.
/// Retrieval and removal both return `Option`, which is not an error. An enum
/// leaves room for that to change without a breaking signature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryError {
  /// A ring is already registered under this name.
  ///
  /// Carries the name so a caller reporting the conflict does not have to have
  /// kept it, and so the message is specific without the caller formatting it.
  NameTaken {
    /// The name that was already live.
    name: String,
  },
}

impl fmt::Display for RegistryError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::NameTaken { name } => write!(f, "a ring is already registered as {name:?}"),
    }
  }
}

impl core::error::Error for RegistryError {}

/// A name-to-ring map that owns every ring in it.
///
/// Dropping the registry drops every ring, and dropping a ring drops the
/// records still in it. The test
/// `dropping_the_registry_drops_every_record_still_in_every_ring` measures this
/// rather than assuming it. Drop order across rings is unspecified and varies,
/// so a caller needing one must `remove` them in that order first.
#[derive(Debug)]
pub struct Registry<T> {
  rings: HashMap<String, Split<T>>,
}

impl<T> Default for Registry<T> {
  fn default() -> Self {
    Self::new()
  }
}

impl<T> Registry<T> {
  /// An empty registry.
  #[must_use]
  pub fn new() -> Self {
    Self { rings: HashMap::new() }
  }

  /// Register a ring under a name, taking ownership of it.
  ///
  /// **Refuses rather than replaces.** A registry that silently replaced would
  /// drop the previous ring, and with it every record still unread in that
  /// ring, as a side effect of a name collision. The refusal returns the
  /// rejected `Split` alongside the error so the caller is not forced to
  /// discard it.
  ///
  /// # Errors
  ///
  /// [`RegistryError::NameTaken`] if the name is already live, paired with the
  /// ring that was not registered.
  ///
  /// # The `result_large_err` allow
  ///
  /// `clippy::result_large_err` fires here, and correctly. The `Err` variant is
  /// **exactly 448 bytes**, fixed across every `T` because `Split< T >` is a
  /// handle, not the record. The lint itself measured that number; it is not an
  /// estimate. This method refuses both of the remedies the lint suggests:
  ///
  /// - *Shrink the payload.* The payload **is** the point. Dropping the
  ///   `Split< T >` from the error is exactly the data loss
  ///   `docs/pitfall/001` exists to prevent, one step removed. The ring would
  ///   be destroyed by a failed registration instead of by a successful one.
  /// - *Box it.* That allocates on the failure path, to fix a size the caller
  ///   already pays on the success path anyway. `register` takes the same
  ///   `Split< T >` by value, so the 448 bytes cross this boundary either way.
  ///   It would also make the error awkward to destructure.
  ///
  /// A third option, outside the lint's own suggestions, is refused too.
  /// `large-error-threshold` in `clippy.toml` would silence it crate-wide, which
  /// draws the same objection `docs/decisions/readme.md` Closed 2 raises against
  /// a crate-wide `allow`.
  ///
  /// The cost is that every caller's `Result` is that wide, including on the
  /// `Ok` path. `register` is a setup-time call, once per ring and never in a
  /// loop, so the width is paid where it does not matter.
  /// → `docs/decisions/readme.md` Closed 2.
  ///
  /// # The clone on the occupied arm
  ///
  /// The refusal path clones the name back from `OccupiedEntry`, because
  /// `OccupiedEntry` has no `into_key`. The clone measured
  /// **22–23 ns, 43–44% of the arm**. The only method that yields the key by
  /// value is `remove_entry`, which deletes the ring this method must not destroy.
  /// → `docs/workaround/001`.
  #[allow(
    clippy::result_large_err,
    reason = "the large payload is the caller's ring, handed back rather than destroyed"
  )]
  pub fn register(&mut self, name: impl Into<String>, ring: Split<T>) -> Result<(), (RegistryError, Split<T>)> {
    let name = name.into();

    match self.rings.entry(name) {
      Entry::Occupied(occupied) => {
        let name = occupied.key().clone();
        Err((RegistryError::NameTaken { name }, ring))
      }
      Entry::Vacant(vacant) => {
        vacant.insert(ring);
        Ok(())
      }
    }
  }

  /// Borrow the ring registered under a name, mutably.
  ///
  /// **There is no immutable counterpart, and that is deliberate.**
  /// `Split::ends` takes `&mut self`, so a `&Split< T >` can do nothing at all.
  /// An immutable `get` would be a method that compiles, returns something,
  /// and permits no operation. [`Self::contains`] is the immutable query that
  /// is answerable.
  pub fn get_mut(&mut self, name: &str) -> Option<&mut Split<T>> {
    self.rings.get_mut(name)
  }

  /// Take the ring registered under a name, freeing the name.
  ///
  /// This is what makes "a name that is taken" a temporary condition rather
  /// than a permanent one, and therefore what makes [`Self::register`]'s
  /// refusal recoverable rather than final.
  pub fn remove(&mut self, name: &str) -> Option<Split<T>> {
    self.rings.remove(name)
  }

  /// Whether a name is live.
  #[must_use]
  pub fn contains(&self, name: &str) -> bool {
    self.rings.contains_key(name)
  }

  /// How many rings are registered.
  #[must_use]
  pub fn len(&self) -> usize {
    self.rings.len()
  }

  /// Whether nothing is registered.
  #[must_use]
  pub fn is_empty(&self) -> bool {
    self.rings.is_empty()
  }

  /// Every live name, in no particular order.
  ///
  /// `HashMap` iteration order is unspecified and varies between one map and
  /// the next, even within a single run. A caller comparing this against an
  /// expected list must sort or collect into a set first. An ordered map
  /// would remove that hazard; it is not used because the population is not
  /// known to stay small, not because it costs more today.
  pub fn names(&self) -> impl Iterator<Item = &str> {
    self.rings.keys().map(String::as_str)
  }
}
