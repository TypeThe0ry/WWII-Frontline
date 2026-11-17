//! M0-027 acceptance 5: the synthetic input sequence is a function of `--seed` (the binary's
//! `--hash-only` prints each client's sequence fingerprint without touching the network).

use std::process::Command;

fn hashes(seed: u32, count: u32) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_headless-client"))
        .args([
            "--hash-only",
            "6000",
            "--count",
            &count.to_string(),
            "--seed",
            &seed.to_string(),
        ])
        .output()
        .expect("run headless-client");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("utf8")
}

#[test]
fn same_seed_same_input_hashes_across_runs() {
    let a = hashes(1, 5);
    assert_eq!(a.lines().count(), 5);
    assert_eq!(a, hashes(1, 5), "two runs with --seed 1 differ");
    assert_ne!(a, hashes(2, 5), "different seeds must differ");
    // clients of one run differ from each other
    let distinct: std::collections::HashSet<&str> = a
        .lines()
        .map(|l| l.split(": ").nth(1).unwrap_or(""))
        .collect();
    assert_eq!(distinct.len(), 5);
}
