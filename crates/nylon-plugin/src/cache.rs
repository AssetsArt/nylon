use dashmap::DashMap;
use flatbuffers::FlatBufferBuilder;
use nylon_sdk::fbs::plugin_generated::nylon_plugin::{
    HeaderKeyValue, HeaderKeyValueArgs, NylonHttpHeaders, NylonHttpHeadersArgs,
};
use once_cell::sync::Lazy;
use std::sync::Arc;

/// Cache key for serialized headers
#[derive(Hash, Eq, PartialEq, Clone)]
struct HeadersCacheKey {
    headers: Vec<(String, String)>,
}

/// Cached FlatBuffers data
struct CachedFlatBuffer {
    data: Vec<u8>,
    last_used: std::time::Instant,
}

/// Global cache for FlatBuffers serialization
static FLATBUFFER_CACHE: Lazy<DashMap<HeadersCacheKey, Arc<CachedFlatBuffer>>> =
    Lazy::new(|| DashMap::with_capacity(128));

const CACHE_SIZE_LIMIT: usize = 1000;
const CACHE_TTL_SECS: u64 = 300; // 5 minutes

/// Build FlatBuffer for headers with caching
pub fn build_headers_flatbuffer(headers: &[(String, String)]) -> Vec<u8> {
    // Create cache key (sorted for consistency)
    let mut sorted_headers = headers.to_vec();
    sorted_headers.sort_by(|a, b| a.0.cmp(&b.0));
    let cache_key = HeadersCacheKey {
        headers: sorted_headers,
    };

    // Try to get from cache
    if let Some(cached) = FLATBUFFER_CACHE.get(&cache_key) {
        let now = std::time::Instant::now();
        if now.duration_since(cached.last_used).as_secs() < CACHE_TTL_SECS {
            return cached.data.clone();
        }
    }

    // Build new FlatBuffer
    let mut fbs = FlatBufferBuilder::new();
    let headers_vec = headers
        .iter()
        .map(|(k, v)| {
            let key = fbs.create_string(k);
            let value = fbs.create_string(v);
            HeaderKeyValue::create(
                &mut fbs,
                &HeaderKeyValueArgs {
                    key: Some(key),
                    value: Some(value),
                },
            )
        })
        .collect::<Vec<_>>();

    let headers_vec = fbs.create_vector(&headers_vec);
    let headers = NylonHttpHeaders::create(
        &mut fbs,
        &NylonHttpHeadersArgs {
            headers: Some(headers_vec),
        },
    );
    fbs.finish(headers, None);
    let data = fbs.finished_data().to_vec();

    // Cache it
    let cached = Arc::new(CachedFlatBuffer {
        data: data.clone(),
        last_used: std::time::Instant::now(),
    });

    // Evict old entries if cache is too large
    if FLATBUFFER_CACHE.len() >= CACHE_SIZE_LIMIT {
        evict_old_entries();
    }

    FLATBUFFER_CACHE.insert(cache_key, cached);
    data
}

/// Evict old cache entries
fn evict_old_entries() {
    let now = std::time::Instant::now();
    FLATBUFFER_CACHE.retain(|_, v| now.duration_since(v.last_used).as_secs() < CACHE_TTL_SECS);
}

/// Clear the cache (for testing or memory management)
pub fn clear_cache() {
    FLATBUFFER_CACHE.clear();
}

/// Get cache statistics
pub fn cache_stats() -> (usize, usize) {
    let size = FLATBUFFER_CACHE.len();
    let capacity = FLATBUFFER_CACHE.capacity();
    (size, capacity)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_basic() {
        clear_cache();

        let headers = vec![
            ("content-type".to_string(), "application/json".to_string()),
            ("x-custom".to_string(), "value".to_string()),
        ];

        let data1 = build_headers_flatbuffer(&headers);
        let data2 = build_headers_flatbuffer(&headers);

        // Should return same data (from cache)
        assert_eq!(data1, data2);

        let (size, _) = cache_stats();
        assert_eq!(size, 1);
    }

    #[test]
    fn test_cache_order_independence() {
        clear_cache();

        let headers1 = vec![
            ("a".to_string(), "1".to_string()),
            ("b".to_string(), "2".to_string()),
        ];

        let headers2 = vec![
            ("b".to_string(), "2".to_string()),
            ("a".to_string(), "1".to_string()),
        ];

        let data1 = build_headers_flatbuffer(&headers1);
        let data2 = build_headers_flatbuffer(&headers2);

        // Should use same cache entry (order-independent)
        assert_eq!(data1, data2);

        let (size, _) = cache_stats();
        assert_eq!(size, 1);
    }
}
