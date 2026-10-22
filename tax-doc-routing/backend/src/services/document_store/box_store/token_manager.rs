//! The token lifecycle Pega runs, in one place: sign in with the key store, save the token with its
//! expiry in a table, use the latest valid token for every Box call, and when it has expired sign
//! in again and replace the stored token.
//!
//! The expiry is whatever Box says (`expires_in`), not a fixed number of hours, so it keeps working
//! if Box or the client's app settings change the lifetime. A token is renewed a minute before it
//! expires, and immediately if Box answers 401. Because the token is stored, a restart (or a
//! second server) reuses a still-valid token instead of signing in again.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};

use crate::services::document_store::StoreError;

/// Renew this long before the token's own expiry.
pub const RENEW_MARGIN_SECS: i64 = 60;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredToken {
    pub access_token: String,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct MintedToken {
    pub access_token: String,
    pub expires_at: DateTime<Utc>,
}

/// What the HTTP layer needs: a valid bearer token, and a way to drop one Box rejected.
#[async_trait]
pub trait TokenProvider: Send + Sync {
    async fn token(&self) -> Result<String, StoreError>;
    async fn invalidate(&self);
}

/// Signs in with the vendor and returns a fresh token (Pega's `GenerateToken_Box`).
#[async_trait]
pub trait TokenMinter: Send + Sync {
    async fn mint(&self) -> Result<MintedToken, StoreError>;
}

/// Where the current token is kept between calls and restarts (the `storage_tokens` table).
#[async_trait]
pub trait TokenStore: Send + Sync {
    async fn load(&self, provider: &str) -> Result<Option<StoredToken>, StoreError>;
    /// Replace the provider's token (one row per provider).
    async fn save(&self, provider: &str, token: &StoredToken) -> Result<(), StoreError>;
    async fn clear(&self, provider: &str) -> Result<(), StoreError>;
}

type Clock = Arc<dyn Fn() -> DateTime<Utc> + Send + Sync>;

pub struct TokenManager {
    provider: String,
    minter: Arc<dyn TokenMinter>,
    store: Option<Arc<dyn TokenStore>>,
    now: Clock,
    cache: tokio::sync::Mutex<Option<StoredToken>>,
}

impl TokenManager {
    pub fn new(provider: &str, minter: Arc<dyn TokenMinter>, store: Option<Arc<dyn TokenStore>>) -> Self {
        Self::with_clock(provider, minter, store, Arc::new(Utc::now))
    }

    pub fn with_clock(provider: &str, minter: Arc<dyn TokenMinter>, store: Option<Arc<dyn TokenStore>>, now: Clock) -> Self {
        Self { provider: provider.to_string(), minter, store, now, cache: tokio::sync::Mutex::new(None) }
    }

    fn usable(&self, token: &StoredToken) -> bool {
        token.expires_at - Duration::seconds(RENEW_MARGIN_SECS) > (self.now)()
    }
}

#[async_trait]
impl TokenProvider for TokenManager {
    async fn token(&self) -> Result<String, StoreError> {
        let mut cache = self.cache.lock().await;
        if let Some(t) = cache.as_ref().filter(|t| self.usable(t)) {
            return Ok(t.access_token.clone());
        }

        // Another server (or an earlier run) may already have a valid token stored.
        if let Some(store) = &self.store {
            match store.load(&self.provider).await {
                Ok(Some(t)) if self.usable(&t) => {
                    let token = t.access_token.clone();
                    *cache = Some(t);
                    return Ok(token);
                }
                Ok(_) => {}
                Err(e) => tracing::warn!(provider = %self.provider, error = %e, "could not read the stored storage token; signing in instead"),
            }
        }

        let minted = self.minter.mint().await?;
        let fresh = StoredToken { access_token: minted.access_token, issued_at: (self.now)(), expires_at: minted.expires_at };
        if let Some(store) = &self.store {
            // A failed save is not fatal: the token still works for this process.
            if let Err(e) = store.save(&self.provider, &fresh).await {
                tracing::warn!(provider = %self.provider, error = %e, "could not save the storage token");
            }
        }
        let token = fresh.access_token.clone();
        *cache = Some(fresh);
        Ok(token)
    }

    async fn invalidate(&self) {
        *self.cache.lock().await = None;
        if let Some(store) = &self.store {
            if let Err(e) = store.clear(&self.provider).await {
                tracing::warn!(provider = %self.provider, error = %e, "could not clear the stored storage token");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    use super::*;

    struct CountingMinter {
        calls: AtomicUsize,
        lifetime: Duration,
        now: Clock,
    }
    #[async_trait]
    impl TokenMinter for CountingMinter {
        async fn mint(&self) -> Result<MintedToken, StoreError> {
            let n = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
            Ok(MintedToken { access_token: format!("token-{n}"), expires_at: (self.now)() + self.lifetime })
        }
    }

    #[derive(Default)]
    struct MemStore {
        rows: Mutex<HashMap<String, StoredToken>>,
        fail_save: bool,
        fail_load: bool,
    }
    #[async_trait]
    impl TokenStore for MemStore {
        async fn load(&self, provider: &str) -> Result<Option<StoredToken>, StoreError> {
            if self.fail_load {
                return Err(StoreError::Unavailable("db down".into()));
            }
            Ok(self.rows.lock().unwrap().get(provider).cloned())
        }
        async fn save(&self, provider: &str, token: &StoredToken) -> Result<(), StoreError> {
            if self.fail_save {
                return Err(StoreError::Unavailable("db down".into()));
            }
            self.rows.lock().unwrap().insert(provider.to_string(), token.clone());
            Ok(())
        }
        async fn clear(&self, provider: &str) -> Result<(), StoreError> {
            self.rows.lock().unwrap().remove(provider);
            Ok(())
        }
    }

    struct Fixture {
        clock: Arc<Mutex<DateTime<Utc>>>,
        minter: Arc<CountingMinter>,
        store: Arc<MemStore>,
    }

    fn fixture(lifetime_hours: i64, store: MemStore) -> Fixture {
        let clock = Arc::new(Mutex::new(DateTime::parse_from_rfc3339("2026-10-18T08:00:00Z").unwrap().with_timezone(&Utc)));
        let now: Clock = {
            let c = clock.clone();
            Arc::new(move || *c.lock().unwrap())
        };
        let minter = Arc::new(CountingMinter { calls: AtomicUsize::new(0), lifetime: Duration::hours(lifetime_hours), now });
        Fixture { clock, minter, store: Arc::new(store) }
    }

    fn manager(f: &Fixture) -> TokenManager {
        let c = f.clock.clone();
        TokenManager::with_clock("box", f.minter.clone(), Some(f.store.clone()), Arc::new(move || *c.lock().unwrap()))
    }

    fn advance(f: &Fixture, d: Duration) {
        *f.clock.lock().unwrap() += d;
    }

    #[tokio::test]
    async fn the_first_call_signs_in_and_saves_the_token_with_its_expiry() {
        let f = fixture(3, MemStore::default());
        let m = manager(&f);
        assert_eq!(m.token().await.unwrap(), "token-1");
        let saved = f.store.rows.lock().unwrap().get("box").cloned().unwrap();
        assert_eq!(saved.access_token, "token-1");
        assert_eq!(saved.expires_at - saved.issued_at, Duration::hours(3));
    }

    #[tokio::test]
    async fn a_valid_token_is_reused_without_signing_in_again() {
        let f = fixture(3, MemStore::default());
        let m = manager(&f);
        m.token().await.unwrap();
        advance(&f, Duration::hours(2));
        assert_eq!(m.token().await.unwrap(), "token-1");
        assert_eq!(f.minter.calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn an_expired_token_is_replaced_in_the_same_row() {
        let f = fixture(3, MemStore::default());
        let m = manager(&f);
        m.token().await.unwrap();
        advance(&f, Duration::hours(3));
        assert_eq!(m.token().await.unwrap(), "token-2");
        let rows = f.store.rows.lock().unwrap();
        assert_eq!(rows.len(), 1, "one row per provider, replaced not appended");
        assert_eq!(rows.get("box").unwrap().access_token, "token-2");
    }

    #[tokio::test]
    async fn the_token_is_renewed_a_minute_before_it_expires() {
        let f = fixture(3, MemStore::default());
        let m = manager(&f);
        m.token().await.unwrap();
        advance(&f, Duration::hours(3) - Duration::seconds(RENEW_MARGIN_SECS + 1));
        assert_eq!(m.token().await.unwrap(), "token-1", "still outside the margin");
        advance(&f, Duration::seconds(2));
        assert_eq!(m.token().await.unwrap(), "token-2", "inside the margin: renewed early");
    }

    #[tokio::test]
    async fn a_restart_or_second_server_reuses_the_stored_token() {
        let f = fixture(3, MemStore::default());
        manager(&f).token().await.unwrap();
        let second = manager(&f);
        assert_eq!(second.token().await.unwrap(), "token-1");
        assert_eq!(f.minter.calls.load(Ordering::SeqCst), 1, "no second sign-in");
    }

    #[tokio::test]
    async fn an_expired_stored_token_is_not_reused() {
        let f = fixture(3, MemStore::default());
        manager(&f).token().await.unwrap();
        advance(&f, Duration::hours(4));
        assert_eq!(manager(&f).token().await.unwrap(), "token-2");
    }

    #[tokio::test]
    async fn invalidate_forces_a_new_sign_in_and_clears_the_stored_token() {
        let f = fixture(3, MemStore::default());
        let m = manager(&f);
        m.token().await.unwrap();
        m.invalidate().await;
        assert!(f.store.rows.lock().unwrap().is_empty());
        assert_eq!(m.token().await.unwrap(), "token-2");
    }

    #[tokio::test]
    async fn a_failing_table_never_blocks_a_working_token() {
        let f = fixture(3, MemStore { fail_save: true, fail_load: true, ..Default::default() });
        let m = manager(&f);
        assert_eq!(m.token().await.unwrap(), "token-1");
        assert_eq!(m.token().await.unwrap(), "token-1", "still cached in memory");
    }

    #[tokio::test]
    async fn it_works_with_no_table_at_all() {
        let f = fixture(3, MemStore::default());
        let c = f.clock.clone();
        let m = TokenManager::with_clock("box", f.minter.clone(), None, Arc::new(move || *c.lock().unwrap()));
        assert_eq!(m.token().await.unwrap(), "token-1");
    }
}
