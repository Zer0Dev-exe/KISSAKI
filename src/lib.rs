#![forbid(unsafe_code)]
#![doc = include_str!("../docs/LIBRARY.md")]

mod config;
pub mod crawler;
pub mod extract;
mod frontier;
pub mod policy;
mod scope;
mod transport;

pub use crawler::{Config, CrawlReport, Page, crawl};
