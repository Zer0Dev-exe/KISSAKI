use std::io::Read;

use anyhow::{Result, bail};
use kissaki::policy::{normalize_url, validate_destination};
use url::Url;

const MAX_INPUT_BYTES: usize = 1024 * 1024;

pub(crate) struct Targets {
    urls: Vec<Url>,
    bytes: usize,
    limit: usize,
    allow_private: bool,
}

impl Targets {
    pub fn new(limit: usize, allow_private: bool) -> Self {
        Self {
            urls: Vec::new(),
            bytes: 0,
            limit,
            allow_private,
        }
    }

    pub fn argument(&mut self, value: &str) -> Result<()> {
        self.account(value.len())?;
        self.append(value)
    }

    pub fn read(&mut self, reader: impl Read) -> Result<()> {
        let remaining = MAX_INPUT_BYTES - self.bytes;
        let mut text = String::new();
        reader
            .take((remaining + 1) as u64)
            .read_to_string(&mut text)?;
        self.account(text.len())?;
        for line in text.lines().map(str::trim) {
            if !line.is_empty() && !line.starts_with('#') {
                self.append(line)?;
            }
        }
        Ok(())
    }

    pub fn finish(self) -> Result<Vec<Url>> {
        if self.urls.is_empty() {
            bail!("provide at least one seed URL");
        }
        Ok(self.urls)
    }

    fn account(&mut self, bytes: usize) -> Result<()> {
        if bytes > MAX_INPUT_BYTES - self.bytes {
            bail!("combined target input exceeds 1 MiB");
        }
        self.bytes += bytes;
        Ok(())
    }

    fn append(&mut self, value: &str) -> Result<()> {
        if self.urls.len() >= self.limit {
            bail!("seed count exceeds max-pages ({})", self.limit);
        }
        let url = normalize_url(value)?;
        validate_destination(&url, self.allow_private)?;
        self.urls.push(url);
        Ok(())
    }
}

#[cfg(test)]
#[path = "tests/targets.rs"]
mod tests;
