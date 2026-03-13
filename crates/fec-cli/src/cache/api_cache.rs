use super::open_connection;
use anyhow::{Context, Result};
use fec_api::{ApiCache, ApiCacheEntry};
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use std::path::Path;
use url::Url;

/// Placeholder used in URLs when the API key is not DEMO_KEY.
const API_KEY_PLACEHOLDER: &str = "$LIBFEC_API_KEY";

/// SQLite-backed implementation of the ApiCache trait.
pub struct SqliteApiCache {
    conn: Connection,
}

/// Normalize a URL for caching by replacing non-public API keys with a placeholder.
/// Returns (normalized_url, api_key_hash) where api_key_hash is Some if the key was replaced.
fn normalize_url_for_cache(url: &Url) -> (String, Option<String>) {
    let mut normalized = url.clone();
    let mut api_key_hash: Option<String> = None;

    // Find and potentially replace the api_key parameter
    let pairs: Vec<(String, String)> = url
        .query_pairs()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

    let has_non_demo_key = pairs.iter().any(|(k, v)| k == "api_key" && v != "DEMO_KEY");

    if has_non_demo_key {
        // Rebuild query string with placeholder
        normalized.query_pairs_mut().clear();
        for (key, value) in &pairs {
            if key == "api_key" && value != "DEMO_KEY" {
                // Hash the actual API key
                let mut hasher = Sha256::new();
                hasher.update(value.as_bytes());
                let hash = format!("{:x}", hasher.finalize());
                api_key_hash = Some(hash);
                // Use placeholder in URL
                normalized
                    .query_pairs_mut()
                    .append_pair(key, API_KEY_PLACEHOLDER);
            } else {
                normalized.query_pairs_mut().append_pair(key, value);
            }
        }
    }

    (normalized.to_string(), api_key_hash)
}

impl SqliteApiCache {
    /// Create a new SqliteApiCache, opening or creating the database at the given path.
    pub fn new(cache_directory: &Path) -> Result<Self> {
        let db_path = cache_directory.join(".api-cache.db");
        let conn = open_connection(&db_path)
            .with_context(|| format!("Could not open API cache database at {:?}", db_path))?;

        // Create table with auto-increment primary key
        // api_key_hash is empty string for DEMO_KEY entries
        conn.execute(
            "CREATE TABLE IF NOT EXISTS api_cache (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                url TEXT NOT NULL,
                api_key_hash TEXT NOT NULL DEFAULT '',
                body TEXT NOT NULL,
                max_age_secs INTEGER NOT NULL,
                cached_at INTEGER NOT NULL
            )",
            [],
        )
        .context("Failed to create api_cache table")?;

        // Index for fast lookups by (url, api_key_hash)
        conn.execute(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_api_cache_url_hash 
             ON api_cache (url, api_key_hash)",
            [],
        )
        .context("Failed to create api_cache index")?;

        Ok(Self { conn })
    }

    /// Get the current Unix timestamp in seconds.
    fn now_secs() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs() as i64
    }

    /// Clean up expired entries from the cache.
    /// This is called periodically to prevent the cache from growing indefinitely.
    #[allow(dead_code)]
    pub fn cleanup_expired(&self) -> Result<usize> {
        let now = Self::now_secs();
        let deleted = self.conn.execute(
            "DELETE FROM api_cache WHERE cached_at + max_age_secs < ?1",
            [now],
        )?;
        Ok(deleted)
    }
}

impl ApiCache for SqliteApiCache {
    fn get(&self, url: &Url) -> Option<ApiCacheEntry> {
        let (normalized_url, api_key_hash) = normalize_url_for_cache(url);
        let hash_value = api_key_hash.unwrap_or_default(); // Use empty string for DEMO_KEY
        let now = Self::now_secs();

        let result: Result<(String, i64), _> = self.conn.query_row(
            "SELECT body, max_age_secs FROM api_cache
             WHERE url = ?1 AND api_key_hash = ?2 AND cached_at + max_age_secs > ?3",
            rusqlite::params![normalized_url, hash_value, now],
            |row| Ok((row.get(0)?, row.get(1)?)),
        );

        match result {
            Ok((body_str, max_age_secs)) => {
                match serde_json::from_str(&body_str) {
                    Ok(body) => Some(ApiCacheEntry {
                        body,
                        max_age_secs: max_age_secs as u64,
                    }),
                    Err(_) => None, // Invalid JSON in cache, treat as miss
                }
            }
            Err(_) => None, // Not found or expired
        }
    }

    fn get_stale(&self, url: &Url) -> Option<ApiCacheEntry> {
        let (normalized_url, api_key_hash) = normalize_url_for_cache(url);
        let hash_value = api_key_hash.unwrap_or_default();

        let result: Result<(String, i64), _> = self.conn.query_row(
            "SELECT body, max_age_secs FROM api_cache
             WHERE url = ?1 AND api_key_hash = ?2",
            rusqlite::params![normalized_url, hash_value],
            |row| Ok((row.get(0)?, row.get(1)?)),
        );

        match result {
            Ok((body_str, max_age_secs)) => match serde_json::from_str(&body_str) {
                Ok(body) => Some(ApiCacheEntry {
                    body,
                    max_age_secs: max_age_secs as u64,
                }),
                Err(_) => None,
            },
            Err(_) => None,
        }
    }

    fn set(&mut self, url: &Url, entry: &ApiCacheEntry) -> Result<()> {
        let (normalized_url, api_key_hash) = normalize_url_for_cache(url);
        let hash_value = api_key_hash.unwrap_or_default(); // Use empty string for DEMO_KEY
        let body_str = serde_json::to_string(&entry.body)?;
        let now = Self::now_secs();

        self.conn.execute(
            "INSERT OR REPLACE INTO api_cache (url, api_key_hash, body, max_age_secs, cached_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                normalized_url,
                hash_value,
                body_str,
                entry.max_age_secs as i64,
                now
            ],
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_url_demo_key_unchanged() {
        let url =
            Url::parse("https://api.open.fec.gov/v1/filings?api_key=DEMO_KEY&cycle=2026").unwrap();
        let (normalized, hash) = normalize_url_for_cache(&url);

        assert_eq!(normalized, url.as_str());
        assert!(hash.is_none());
    }

    #[test]
    fn test_normalize_url_real_key_replaced() {
        let url =
            Url::parse("https://api.open.fec.gov/v1/filings?api_key=MY_SECRET_KEY&cycle=2026")
                .unwrap();
        let (normalized, hash) = normalize_url_for_cache(&url);

        // Note: $ gets URL-encoded to %24
        assert!(normalized.contains("api_key=%24LIBFEC_API_KEY"));
        assert!(!normalized.contains("MY_SECRET_KEY"));
        assert!(hash.is_some());
        // Hash should be consistent
        assert_eq!(hash.unwrap().len(), 64); // SHA256 hex is 64 chars
    }

    #[test]
    fn test_normalize_url_same_key_same_hash() {
        let url1 =
            Url::parse("https://api.open.fec.gov/v1/filings?api_key=MY_KEY&cycle=2026").unwrap();
        let url2 =
            Url::parse("https://api.open.fec.gov/v1/filings?api_key=MY_KEY&cycle=2024").unwrap();

        let (_, hash1) = normalize_url_for_cache(&url1);
        let (_, hash2) = normalize_url_for_cache(&url2);

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_normalize_url_different_keys_different_hashes() {
        let url1 =
            Url::parse("https://api.open.fec.gov/v1/filings?api_key=KEY_A&cycle=2026").unwrap();
        let url2 =
            Url::parse("https://api.open.fec.gov/v1/filings?api_key=KEY_B&cycle=2026").unwrap();

        let (_, hash1) = normalize_url_for_cache(&url1);
        let (_, hash2) = normalize_url_for_cache(&url2);

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_normalize_url_preserves_other_params() {
        let url = Url::parse(
            "https://api.open.fec.gov/v1/filings?api_key=SECRET&per_page=100&cycle=2026",
        )
        .unwrap();
        let (normalized, _) = normalize_url_for_cache(&url);

        assert!(normalized.contains("per_page=100"));
        assert!(normalized.contains("cycle=2026"));
    }

    #[test]
    fn test_cache_roundtrip_with_real_key() {
        let dir = tempfile::tempdir().unwrap();
        let mut cache = SqliteApiCache::new(dir.path()).unwrap();

        let url =
            Url::parse("https://api.open.fec.gov/v1/filings?api_key=MY_SECRET&cycle=2026").unwrap();
        let entry = ApiCacheEntry {
            body: serde_json::json!({"test": "data"}),
            max_age_secs: 3600,
        };

        // Set and get should work
        cache.set(&url, &entry).unwrap();
        let result = cache.get(&url);

        assert!(result.is_some());
        assert_eq!(result.unwrap().body, entry.body);
    }

    #[test]
    fn test_cache_different_keys_isolated() {
        let dir = tempfile::tempdir().unwrap();
        let mut cache = SqliteApiCache::new(dir.path()).unwrap();

        let url1 =
            Url::parse("https://api.open.fec.gov/v1/filings?api_key=KEY_A&cycle=2026").unwrap();
        let url2 =
            Url::parse("https://api.open.fec.gov/v1/filings?api_key=KEY_B&cycle=2026").unwrap();

        let entry1 = ApiCacheEntry {
            body: serde_json::json!({"user": "A"}),
            max_age_secs: 3600,
        };
        let entry2 = ApiCacheEntry {
            body: serde_json::json!({"user": "B"}),
            max_age_secs: 3600,
        };

        cache.set(&url1, &entry1).unwrap();
        cache.set(&url2, &entry2).unwrap();

        // Each key should get its own cached data
        let result1 = cache.get(&url1).unwrap();
        let result2 = cache.get(&url2).unwrap();

        assert_eq!(result1.body["user"], "A");
        assert_eq!(result2.body["user"], "B");
    }

    #[test]
    fn test_cache_demo_key_shared() {
        let dir = tempfile::tempdir().unwrap();
        let mut cache = SqliteApiCache::new(dir.path()).unwrap();

        let url =
            Url::parse("https://api.open.fec.gov/v1/filings?api_key=DEMO_KEY&cycle=2026").unwrap();
        let entry = ApiCacheEntry {
            body: serde_json::json!({"shared": true}),
            max_age_secs: 3600,
        };

        cache.set(&url, &entry).unwrap();

        // Same URL with DEMO_KEY should hit cache
        let result = cache.get(&url);
        assert!(result.is_some());
    }
}
