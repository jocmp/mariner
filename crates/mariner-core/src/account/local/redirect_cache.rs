//! When to trust a feed's permanent redirect.
//!
//! A redirect is trusted once an all-permanent chain (301 or 308 at every
//! hop) to the same target has been seen [`TRUST_AFTER`] times in a row.
//! From then on the feed is fetched from the target. The feed's URL never
//! changes, so its article ids stay the same.

use mariner_feeds::fetch::Redirect;

/// Sightings before a redirect is trusted. (Feedbin uses 576, tuned for a
/// server polling every 15 minutes; a client refreshes far less often.)
pub(super) const TRUST_AFTER: i64 = 5;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct RedirectCache {
    /// The pending or trusted target.
    pub redirected_to: Option<String>,
    /// Consecutive sightings of `redirected_to`.
    pub redirect_count: i64,
}

impl RedirectCache {
    /// Where to fetch the feed subscribed at `feed_url`.
    pub fn url<'a>(&'a self, feed_url: &'a str) -> &'a str {
        match &self.redirected_to {
            Some(target) if self.is_trusted() => target,
            _ => feed_url,
        }
    }

    pub fn is_trusted(&self) -> bool {
        self.redirected_to.is_some() && self.redirect_count >= TRUST_AFTER
    }

    /// Counts the redirects a successful fetch followed.
    pub fn save(&mut self, redirects: &[Redirect]) {
        let permanent = !redirects.is_empty() && redirects.iter().all(Redirect::is_permanent);
        if permanent {
            let target = &redirects.last().expect("not empty").to;
            if self.redirected_to.as_ref() == Some(target) {
                self.redirect_count += 1;
            } else {
                self.redirected_to = Some(target.clone());
                self.redirect_count = 1;
            }
        } else if !self.is_trusted() {
            // A trusted target fetched directly has no redirects; anything
            // else breaks the streak.
            self.redirected_to = None;
            self.redirect_count = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FEED: &str = "http://example.com/feed";
    const TARGET: &str = "https://example.com/feed";

    #[test]
    fn trusts_a_permanent_redirect_after_repeated_sightings() {
        let mut cache = RedirectCache::default();
        for _ in 1..TRUST_AFTER {
            cache.save(&[redirect(301, FEED, TARGET)]);
            assert_eq!(cache.url(FEED), FEED);
        }
        cache.save(&[redirect(308, FEED, TARGET)]);
        assert_eq!(cache.url(FEED), TARGET);

        // Fetched from the target directly, there are no redirects.
        cache.save(&[]);
        assert_eq!(cache.url(FEED), TARGET);
    }

    #[test]
    fn a_temporary_hop_or_a_new_target_restarts_the_count() {
        let mut cache = RedirectCache::default();
        cache.save(&[redirect(301, FEED, TARGET)]);
        cache.save(&[
            redirect(301, FEED, "https://example.com/a"),
            redirect(302, "https://example.com/a", TARGET),
        ]);
        assert_eq!(cache, RedirectCache::default());

        cache.save(&[redirect(301, FEED, TARGET)]);
        cache.save(&[redirect(301, FEED, "https://elsewhere.example/feed")]);
        assert_eq!(cache.redirect_count, 1);
        assert_eq!(
            cache.redirected_to.as_deref(),
            Some("https://elsewhere.example/feed")
        );

        cache.save(&[]);
        assert_eq!(cache, RedirectCache::default());
    }

    #[test]
    fn a_trusted_target_that_moves_again_is_no_longer_trusted() {
        let mut cache = RedirectCache {
            redirected_to: Some(TARGET.into()),
            redirect_count: TRUST_AFTER,
        };
        cache.save(&[redirect(302, TARGET, "https://cdn.example.com/feed")]);
        assert!(cache.is_trusted(), "temporary hops leave it trusted");

        cache.save(&[redirect(301, TARGET, "https://new.example.com/feed")]);
        assert_eq!(cache.url(FEED), FEED);
    }

    fn redirect(status: u16, from: &str, to: &str) -> Redirect {
        Redirect {
            status,
            from: from.into(),
            to: to.into(),
        }
    }
}
