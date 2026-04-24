//! 数据库模块
//!
//! 提供 SQLite 数据库访问功能

#[allow(dead_code)]
pub mod pool;
#[allow(dead_code)]
pub mod credentials;
#[allow(dead_code)]
pub mod stats;
#[allow(dead_code)]
pub mod balance;
#[allow(dead_code)]
pub mod audit;
#[allow(dead_code)]
pub mod migration;

pub use pool::{init_pool, run_migrations};
