// Search indexing and query modules using Tantivy

pub mod schema;
pub mod metadata;
pub mod index_builder;
pub mod query_executor;
pub mod cli;

// Re-exports
pub use schema::{build_schema, LogEntryDocument};
pub use metadata::{IndexMetadata, LocationMetadata};
pub use index_builder::IndexBuilder;
pub use query_executor::QueryExecutor;
pub use cli::handle_search;
