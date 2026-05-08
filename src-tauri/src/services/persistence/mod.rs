// Phase: 2
// Persistence service layer.

pub mod atomic_writer;
pub mod builtin_data;
pub mod corruption;
pub mod metadata_store;
pub mod migration;
pub mod migration_log;
pub mod profile_repo;
pub mod settings_repo;
pub mod tag_repo;

#[cfg(test)]
mod tests;
