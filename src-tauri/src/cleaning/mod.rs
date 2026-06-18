//! Cleaning subsystems: junk detection, safe deletion, large/old files,
//! duplicates, app uninstaller, startup items, platform path tables, and the
//! protected-path safety gate.
pub mod duplicates;
pub mod junk;
pub mod large_old;
pub mod paths;
pub mod safety;
pub mod startup;
pub mod trash;
pub mod uninstaller;
