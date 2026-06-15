//! Cleaning subsystems: junk detection, safe deletion, large/old files,
//! duplicates, app uninstaller, startup items, and platform path tables.
pub mod duplicates;
pub mod junk;
pub mod large_old;
pub mod paths;
pub mod startup;
pub mod trash;
pub mod uninstaller;
