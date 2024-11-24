use criterion::{criterion_group, criterion_main, Criterion};
use reqwest::Client;
use tokio_test::block_on;

fn benchmark_status_handler(c: &mut Criterion) {
  let client = Client::new();
  let url = "http://127.0.0.1:3000/status";

  c.bench_function("status_bench", |b| {
    b.iter(|| {
      block_on(async {
        let response = client.get(url).send().await.unwrap();
        assert_eq!(response.status(), 200);
      });
    });
  });
}

criterion_group!(benches, benchmark_status_handler);
criterion_main!(benches);
