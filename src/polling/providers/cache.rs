use 

/// Policy to use when adapting the cache size based on the current cache
/// pressure
#[derive(Clone, Copy, Debug)]
enum AdaptivePolicy {
    /// Specifies that the cache should adapt (increase or decrease capacity)
    /// immediately upon a change that puts the current cache pressure at a
    /// different amount than the current capacity
    Immediate,
    /// Cache will adapt only if the current cache pressure moves outside an
    /// inclusive interval based on the current cache capacity, defined as
    /// `[(1-threshold) * capacity, (1-threshold) * capacity]`
    ImmediateThreshold,
    AverageSpan {
        threshold: f32,
    },
}

struct AdaptiveLruCache {
    adaptive_average: f32,
    adaptive_span:    u32,
    upscale_policy: AdaptivePolicy,
    downscale_policy: AdaptivePolicy,
    cache: LruCache
}
