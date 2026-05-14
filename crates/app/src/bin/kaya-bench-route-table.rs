use kaya_mesh::{RouteEntry, RouteEntrySpec, RouteSource, RoutingTable};
use std::time::Instant;

fn main() {
    let routes = std::env::args()
        .nth(1)
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(10_000);
    let mut table = RoutingTable::new(120_000);
    let started = Instant::now();

    for index in 0..routes {
        let destination = format!("KY-{index:06X}");
        let next_hop = format!("KY-{:06X}", index % 128);
        table.upsert(RouteEntry::from_spec(RouteEntrySpec {
            destination_node: destination,
            destination_callsign: Some(format!("peer-{index}")),
            next_hop,
            hop_count: 1 + (index % 5) as u8,
            trusted: index % 3 == 0,
            encrypted_capable: index % 2 == 0,
            source: RouteSource::Announce,
            latency_ms: Some((index % 80) as u64),
        }));
    }

    let elapsed = started.elapsed();
    let per_second = routes as f64 / elapsed.as_secs_f64().max(0.001);
    println!(
        "route_table: routes={} stored={} elapsed_ms={} routes_per_sec={:.0}",
        routes,
        table.len(),
        elapsed.as_millis(),
        per_second
    );
}
