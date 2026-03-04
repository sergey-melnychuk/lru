use criterion::{
    BenchmarkGroup, Criterion, criterion_group, criterion_main, measurement::WallTime,
};
use lru::{dll, map};

const SIZES: &[usize] = &[1_000, 10_000, 100_000, 1_000_000];

/// Sequential puts that always evict (worst-case for both implementations).
fn bench_put_evict<C: Cache>(group: &mut BenchmarkGroup<WallTime>, name: &str, size: usize) {
    group.bench_function(name, |b| {
        let mut cache = C::new(size);
        // Pre-fill.
        for i in 0..size {
            cache.put(i, i);
        }
        let mut key = size;
        b.iter(|| {
            cache.put(key, key);
            key += 1;
        });
    });
}

/// Gets that always hit (promotes to MRU each time).
fn bench_get_hit<C: Cache>(group: &mut BenchmarkGroup<WallTime>, name: &str, size: usize) {
    group.bench_function(name, |b| {
        let mut cache = C::new(size);
        for i in 0..size {
            cache.put(i, i);
        }
        let mut key = 0;
        b.iter(|| {
            cache.get(&key);
            key = (key + 1) % size;
        });
    });
}

fn benchmarks(c: &mut Criterion) {
    for &size in SIZES {
        let label = format!("put_evict/size={size}");
        let mut group = c.benchmark_group(&label);
        bench_put_evict::<DllCache>(&mut group, "dll", size);
        bench_put_evict::<MapCache>(&mut group, "map", size);
        group.finish();

        let label = format!("get_hit/size={size}");
        let mut group = c.benchmark_group(&label);
        bench_get_hit::<DllCache>(&mut group, "dll", size);
        bench_get_hit::<MapCache>(&mut group, "map", size);
        group.finish();
    }
}

criterion_group!(benches, benchmarks);
criterion_main!(benches);

trait Cache {
    fn new(limit: usize) -> Self;
    fn get(&mut self, key: &usize) -> Option<usize>;
    fn put(&mut self, key: usize, value: usize);
}

struct DllCache(dll::Lru<usize, usize>);
struct MapCache(map::Lru<usize, usize>);

impl Cache for DllCache {
    fn new(limit: usize) -> Self {
        Self(dll::Lru::new(limit))
    }
    fn get(&mut self, key: &usize) -> Option<usize> {
        self.0.get(key)
    }
    fn put(&mut self, key: usize, value: usize) {
        self.0.put(key, value);
    }
}

impl Cache for MapCache {
    fn new(limit: usize) -> Self {
        Self(map::Lru::new(limit))
    }
    fn get(&mut self, key: &usize) -> Option<usize> {
        self.0.get(key)
    }
    fn put(&mut self, key: usize, value: usize) {
        self.0.put(key, value);
    }
}
