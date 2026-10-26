use std::collections::HashSet;

use url::{Origin, Url};

use crate::{Config, policy::validate_destination};

pub(crate) struct Scope<'config> {
    origins: HashSet<Origin>,
    config: &'config Config,
}

impl<'config> Scope<'config> {
    pub fn new(seeds: &[Url], config: &'config Config) -> Self {
        Self {
            origins: seeds.iter().map(Url::origin).collect(),
            config,
        }
    }

    pub fn allows(&self, url: &Url) -> bool {
        self.origins.contains(&url.origin())
            && validate_destination(url, self.config.allow_private).is_ok()
            && self
                .config
                .include
                .as_ref()
                .is_none_or(|pattern| pattern.is_match(url.as_str()))
            && self
                .config
                .exclude
                .as_ref()
                .is_none_or(|pattern| !pattern.is_match(url.as_str()))
    }
}
