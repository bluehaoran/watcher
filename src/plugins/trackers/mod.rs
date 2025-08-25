// Tracker plugin implementations
pub mod price;
pub mod version;
pub mod number;

pub use price::PriceTracker;
pub use version::VersionTracker;
pub use number::NumberTracker;