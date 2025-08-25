pub mod traits;
pub mod manager;
pub mod trackers;
pub mod notifiers;

pub use manager::PluginManager;
pub use traits::{TrackerPlugin, NotifierPlugin};