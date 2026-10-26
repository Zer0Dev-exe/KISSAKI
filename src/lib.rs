#![forbid(unsafe_code)]

pub mod crawler;
pub mod extract;
pub mod policy;

pub use crawler::{Config, CrawlReport, Page, crawl};
