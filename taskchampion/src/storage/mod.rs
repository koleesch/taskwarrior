/**
This module defines the backend storage used by [`Replica`](crate::Replica).
It defines a [trait](crate::storage::Storage) for storage implementations, and provides a default on-disk implementation as well as an in-memory implementation for testing.

Typical uses of this crate do not interact directly with this module; [`StorageConfig`](crate::StorageConfig) is sufficient.
However, users who wish to implement their own storage backends can implement the traits defined here and pass the result to [`Replica`](crate::Replica).
*/
use anyhow::Result;
use std::collections::HashMap;
use uuid::Uuid;

mod config;
mod inmemory;
mod kv;
mod sqlite;
mod operation;

pub use self::kv::KvStorage;
pub use config::StorageConfig;
pub use inmemory::InMemoryStorage;

pub use operation::Operation;

/// An in-memory representation of a task as a simple hashmap
pub type TaskMap = HashMap<String, String>;

#[cfg(test)]
fn taskmap_with(mut properties: Vec<(String, String)>) -> TaskMap {
    let mut rv = TaskMap::new();
    for (p, v) in properties.drain(..) {
        rv.insert(p, v);
    }
    rv
}

/// The type of VersionIds
pub use crate::server::VersionId;

/// The default for base_version.
pub(crate) const DEFAULT_BASE_VERSION: Uuid = crate::server::NO_VERSION_ID;

/// A Storage transaction, in which storage operations are performed.
///
/// # Concurrency
///
/// Serializable consistency must be maintained.  Concurrent access is unusual
/// and some implementations may simply apply a mutex to limit access to
/// one transaction at a time.
///
/// # Commiting and Aborting
///
/// A transaction is not visible to other readers until it is committed with
/// [`crate::storage::StorageTxn::commit`].  Transactions are aborted if they are dropped.
/// It is safe and performant to drop transactions that did not modify any data without committing.
pub trait StorageTxn {
    /// Get an (immutable) task, if it is in the storage
    fn get_task(&mut self, uuid: Uuid) -> Result<Option<TaskMap>>;

    /// Create an (empty) task, only if it does not already exist.  Returns true if
    /// the task was created (did not already exist).
    fn create_task(&mut self, uuid: Uuid) -> Result<bool>;

    /// Set a task, overwriting any existing task.  If the task does not exist, this implicitly
    /// creates it (use `get_task` to check first, if necessary).
    fn set_task(&mut self, uuid: Uuid, task: TaskMap) -> Result<()>;

    /// Delete a task, if it exists.  Returns true if the task was deleted (already existed)
    fn delete_task(&mut self, uuid: Uuid) -> Result<bool>;

    /// Get the uuids and bodies of all tasks in the storage, in undefined order.
    fn all_tasks(&mut self) -> Result<Vec<(Uuid, TaskMap)>>;

    /// Get the uuids of all tasks in the storage, in undefined order.
    fn all_task_uuids(&mut self) -> Result<Vec<Uuid>>;

    /// Get the current base_version for this storage -- the last version synced from the server.
    fn base_version(&mut self) -> Result<VersionId>;

    /// Set the current base_version for this storage.
    fn set_base_version(&mut self, version: VersionId) -> Result<()>;

    /// Get the current set of outstanding operations (operations that have not been sync'd to the
    /// server yet)
    fn operations(&mut self) -> Result<Vec<Operation>>;

    /// Add an operation to the end of the list of operations in the storage.  Note that this
    /// merely *stores* the operation; it is up to the TaskDb to apply it.
    fn add_operation(&mut self, op: Operation) -> Result<()>;

    /// Replace the current list of operations with a new list.
    fn set_operations(&mut self, ops: Vec<Operation>) -> Result<()>;

    /// Get the entire working set, with each task UUID at its appropriate (1-based) index.
    /// Element 0 is always None.
    fn get_working_set(&mut self) -> Result<Vec<Option<Uuid>>>;

    /// Add a task to the working set and return its (one-based) index.  This index will be one greater
    /// than the highest used index.
    fn add_to_working_set(&mut self, uuid: Uuid) -> Result<usize>;

    /// Update the working set task at the given index.  This cannot add a new item to the
    /// working set.
    fn set_working_set_item(&mut self, index: usize, uuid: Option<Uuid>) -> Result<()>;

    /// Clear all tasks from the working set in preparation for a garbage-collection operation.
    /// Note that this is the only way items are removed from the set.
    fn clear_working_set(&mut self) -> Result<()>;

    /// Commit any changes made in the transaction.  It is an error to call this more than
    /// once.
    fn commit(&mut self) -> Result<()>;
}

/// A trait for objects able to act as task storage.  Most of the interesting behavior is in the
/// [`crate::storage::StorageTxn`] trait.
pub trait Storage {
    /// Begin a transaction
    fn txn<'a>(&'a mut self) -> Result<Box<dyn StorageTxn + 'a>>;
}
