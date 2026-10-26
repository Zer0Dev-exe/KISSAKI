use std::collections::{HashSet, VecDeque};

use anyhow::{Result, bail};
use url::{Origin, Url};

use crate::{
    Config, Page,
    policy::{normalize_url, validate_destination},
};

#[derive(Clone)]
pub(crate) struct Job {
    pub url: Url,
    pub depth: usize,
    pub redirects: usize,
}

pub(crate) struct Frontier<'config> {
    config: &'config Config,
    origins: HashSet<Origin>,
    seen: HashSet<Url>,
    queue: VecDeque<Job>,
    pub page_limit_reached: bool,
}

impl<'config> Frontier<'config> {
    pub fn new(seeds: Vec<Url>, config: &'config Config) -> Result<Self> {
        if seeds.is_empty() || seeds.len() > config.max_pages {
            bail!("provide between 1 and max-pages seed URLs");
        }
        let seeds = seeds
            .iter()
            .map(|seed| {
                let seed = normalize_url(seed.as_str())?;
                validate_destination(&seed, config.allow_private)?;
                Ok(seed)
            })
            .collect::<Result<Vec<_>>>()?;
        let mut frontier = Self {
            config,
            origins: seeds.iter().map(Url::origin).collect(),
            seen: HashSet::new(),
            queue: VecDeque::new(),
            page_limit_reached: false,
        };
        for url in seeds {
            frontier.enqueue(Job {
                url,
                depth: 0,
                redirects: 0,
            });
        }
        Ok(frontier)
    }

    pub fn next(&mut self) -> Option<Job> {
        self.queue.pop_front()
    }

    pub fn complete(&mut self, job: &Job, page: &mut Page) {
        page.links.retain(|url| self.allows(url));
        if let Some(target) = &page.redirect {
            match self.redirect(job, target) {
                Ok(redirect) => self.enqueue(redirect),
                Err(error) => page.error = Some(error.to_string()),
            }
        }
        for url in &page.links {
            self.enqueue(Job {
                url: url.clone(),
                depth: job.depth + 1,
                redirects: 0,
            });
        }
    }

    fn allows(&self, url: &Url) -> bool {
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

    fn redirect(&self, job: &Job, target: &Url) -> Result<Job> {
        if !self.allows(target) {
            bail!("redirect blocked by scope or network policy");
        }
        if job.redirects >= self.config.max_redirects {
            bail!("redirect limit reached");
        }
        Ok(Job {
            url: target.clone(),
            depth: job.depth,
            redirects: job.redirects + 1,
        })
    }

    fn enqueue(&mut self, job: Job) {
        if !self.allows(&job.url) {
            return;
        }
        let mut key = job.url.clone();
        if self.config.ignore_query {
            key.set_query(None);
        }
        if self.seen.contains(&key) {
            return;
        }
        if self.seen.len() >= self.config.max_pages {
            self.page_limit_reached = true;
            return;
        }
        self.seen.insert(key);
        self.queue.push_back(job);
    }
}

#[cfg(test)]
#[path = "tests/frontier.rs"]
mod tests;
