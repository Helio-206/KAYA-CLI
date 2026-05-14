pub fn score_route(
    hop_count: u8,
    trusted: bool,
    encrypted_capable: bool,
    latency_ms: Option<u64>,
    failure_count: u32,
    age_ms: u64,
) -> i64 {
    let trust_bonus = if trusted { 250 } else { 0 };
    let encryption_bonus = if encrypted_capable { 75 } else { 0 };
    let latency_penalty = latency_ms.unwrap_or(250) as i64 / 5;
    let age_penalty = (age_ms / 10_000) as i64;
    10_000 - hop_count as i64 * 700 + trust_bonus + encryption_bonus
        - latency_penalty
        - failure_count as i64 * 300
        - age_penalty
}
