//! 数据库模块
//!
//! 提供 SQLite 数据库访问功能

pub mod pool;
pub mod credentials;
pub mod stats;
pub mod balance;
pub mod audit;
pub mod migration;

pub use pool::{init_pool, run_migrations};
