//! The seeded item sequence every candidate write path is driven with.

/// The shape of the payload each item carries.
///
/// Both variants produce a fixed eight-byte record, because a table of
/// fixed-extent cells is what the oracle compares. What varies is how much of
/// that record is significant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PayloadArchetype {
    /// All eight bytes significant, every item the same width.
    #[default]
    Uniform,
    /// A variable-width record padded to eight bytes, the significant width
    /// cycling with the item index.
    ///
    /// Exists so a candidate that quietly assumes a constant width is measured
    /// against one that does not, rather than never meeting the case.
    Mixed,
}

/// One generated item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Item {
    /// Position in the full sequence, counted from zero.
    pub index: u64,
    /// The eight-byte payload, shaped by the workload's archetype.
    pub payload: [u8; 8],
}

/// A deterministic item sequence, reproducible from its seed alone.
///
/// **The sequence does not depend on `producer_count`.** Each item is a pure
/// function of `( seed, index )`, and the producer count only decides how the
/// same items are handed out. That is what makes determinism structural rather
/// than a property the generator has to be careful to preserve: there is no
/// interleaving for a thread count to perturb, because nothing is generated in
/// thread order in the first place.
///
/// ```
/// use bench_harness::{PayloadArchetype, Workload};
///
/// let a = Workload::new(7, 4, 256, 64, PayloadArchetype::Uniform);
/// let b = Workload::new(7, 8, 256, 64, PayloadArchetype::Uniform);
///
/// // Different producer counts, same items.
/// assert_eq!(a.items(), b.items());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Workload {
    seed: u64,
    producer_count: usize,
    item_count: u64,
    batch_size: u64,
    archetype: PayloadArchetype,
}

impl Workload {
    /// A workload of `item_count` items, handed to `producer_count` producers in
    /// batches of `batch_size`.
    ///
    /// `producer_count` and `batch_size` are each clamped to a minimum of one.
    /// Zero of either describes no workload at all rather than an error worth
    /// returning, and clamping keeps the constructor total — a harness that can
    /// panic while being configured is a harness that cannot grade a bad config.
    #[must_use]
    pub const fn new(
        seed: u64,
        producer_count: usize,
        item_count: u64,
        batch_size: u64,
        archetype: PayloadArchetype,
    ) -> Self {
        Self {
            seed,
            producer_count: if producer_count == 0 { 1 } else { producer_count },
            item_count,
            batch_size: if batch_size == 0 { 1 } else { batch_size },
            archetype,
        }
    }

    /// The seed this workload reproduces from.
    #[must_use]
    pub const fn seed(self) -> u64 {
        self.seed
    }

    /// How many producers the items are divided among.
    #[must_use]
    pub const fn producer_count(self) -> usize {
        self.producer_count
    }

    /// How many items the full sequence holds.
    #[must_use]
    pub const fn item_count(self) -> u64 {
        self.item_count
    }

    /// How many consecutive items one producer takes before the next producer
    /// takes over.
    #[must_use]
    pub const fn batch_size(self) -> u64 {
        self.batch_size
    }

    /// The payload shape each item carries.
    #[must_use]
    pub const fn archetype(self) -> PayloadArchetype {
        self.archetype
    }

    /// The item at `index`, independent of every other item and of the producer
    /// count.
    ///
    /// ```
    /// use bench_harness::{PayloadArchetype, Workload};
    ///
    /// let w = Workload::new(1, 4, 10, 2, PayloadArchetype::Uniform);
    /// assert_eq!(w.item(3).index, 3);
    /// assert_eq!(w.item(3), w.item(3));
    /// ```
    #[must_use]
    pub const fn item(self, index: u64) -> Item {
        // The family's established mixing constant — a neighbouring index differs
        // in every byte rather than in one, so a fold landing one slot over
        // produces an obviously wrong payload instead of a nearly-right one.
        let mixed = self.seed ^ index.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        let bytes = mixed.to_le_bytes();

        let significant = match self.archetype {
            PayloadArchetype::Uniform => 8,
            PayloadArchetype::Mixed => 1 + (index % 8) as usize,
        };

        let mut payload = [0_u8; 8];
        let mut at = 0;

        while at < significant {
            payload[at] = bytes[at];
            at += 1;
        }

        Item { index, payload }
    }

    /// The full sequence, in index order.
    #[must_use]
    pub fn items(self) -> Vec<Item> {
        (0..self.item_count).map(|index| self.item(index)).collect()
    }

    /// The items `producer` is responsible for, in index order.
    ///
    /// Batches are dealt round-robin, so producer `p` takes batch `p`, then
    /// batch `p + producer_count`, and so on. Every item belongs to exactly one
    /// producer, which is what makes the union over all producers equal to
    /// [`Self::items`] as a multiset at any producer count.
    ///
    /// A `producer` at or beyond [`Self::producer_count`] owns nothing.
    ///
    /// ```
    /// use bench_harness::{PayloadArchetype, Workload};
    ///
    /// let w = Workload::new(1, 2, 8, 2, PayloadArchetype::Uniform);
    /// let indices: Vec<u64> = w.items_for(0).iter().map(|i| i.index).collect();
    /// assert_eq!(indices, vec![0, 1, 4, 5]);
    /// ```
    #[must_use]
    pub fn items_for(self, producer: usize) -> Vec<Item> {
        if producer >= self.producer_count {
            return Vec::new();
        }

        (0..self.item_count)
            .filter(|index| self.owner_of(*index) == producer)
            .map(|index| self.item(index))
            .collect()
    }

    /// Which producer owns the item at `index`.
    #[must_use]
    pub const fn owner_of(self, index: u64) -> usize {
        ((index / self.batch_size) % self.producer_count as u64) as usize
    }
}
