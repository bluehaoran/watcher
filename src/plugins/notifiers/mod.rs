// Notifier plugin implementations
pub mod email;
pub mod discord;

pub use email::EmailNotifier;
pub use discord::DiscordNotifier;