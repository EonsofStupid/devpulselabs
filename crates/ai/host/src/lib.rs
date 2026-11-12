//! The AI host: work that spans every provider, kept out of the app (A1c,
//! chat plan §4.3.1). The app wires it to chat; nothing here knows chat.

pub mod presence;
pub mod usage;
