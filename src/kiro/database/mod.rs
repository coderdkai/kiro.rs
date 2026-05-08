//! 数据库模块
//!
//! 提供 SQLite 数据库访问功能

pub mod pool;
pub mod credentials;
#[allow(dead_code)]
pub mod stats;
#[allow(dead_code)]
pub mod balance;

pub use pool::{init_pool, run_migrations};
