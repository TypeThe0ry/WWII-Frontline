//! Development certificate for WebTransport (M0-017): ECDSA P-256, self-signed, valid for at
//! most 13 days (browsers refuse `serverCertificateHashes` certificates valid for more than 14).
//!
//! `cargo run -p frontline-server --bin gen_dev_cert -- [--out dev-cert] [--days 13]`
//! writes `dev-cert.pem`, `dev-key.pem` and `dev-cert.sha256` (hex and base64 of the SHA-256,
//! the value for `serverCertificateHashes`) into the output directory (default `dev-cert/`,
//! gitignored) and prints the same.

use std::path::PathBuf;
use std::process::ExitCode;

use frontline_server::net::wt::{cert_hash, dev_identity, hash_base64, hash_hex, DEV_CERT_DAYS};

fn main() -> ExitCode {
    let mut out = PathBuf::from("dev-cert");
    let mut days = DEV_CERT_DAYS;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match (a.as_str(), args.next()) {
            ("--out", Some(v)) => out = PathBuf::from(v),
            ("--days", Some(v)) => match v.parse::<u32>() {
                Ok(d) if (1..=DEV_CERT_DAYS).contains(&d) => days = d,
                _ => {
                    eprintln!("--days must be 1..={DEV_CERT_DAYS}");
                    return ExitCode::from(2);
                }
            },
            _ => {
                eprintln!("usage: gen_dev_cert [--out DIR] [--days 1..={DEV_CERT_DAYS}]");
                return ExitCode::from(2);
            }
        }
    }
    match run(&out, days) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(out: &std::path::Path, days: u32) -> Result<(), String> {
    let id = dev_identity(days).map_err(|e| e.to_string())?;
    let hash = cert_hash(&id);
    std::fs::create_dir_all(out).map_err(|e| format!("{}: {e}", out.display()))?;
    let cert = out.join("dev-cert.pem");
    let key = out.join("dev-key.pem");
    let fp = out.join("dev-cert.sha256");
    let chain: String = id
        .certificate_chain()
        .as_slice()
        .iter()
        .map(|c| c.to_pem())
        .collect();
    std::fs::write(&cert, chain).map_err(|e| e.to_string())?;
    std::fs::write(&key, id.private_key().to_secret_pem()).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o600));
    }
    let (hex, b64) = (hash_hex(&hash), hash_base64(&hash));
    std::fs::write(&fp, format!("hex={hex}\nbase64={b64}\n")).map_err(|e| e.to_string())?;
    println!("certificate : {}", cert.display());
    println!("private key : {}", key.display());
    println!("valid for   : {days} days from now (ECDSA P-256, SAN localhost/127.0.0.1/::1)");
    println!("sha256 hex  : {hex}");
    println!("sha256 b64  : {b64}");
    println!(
        "run server  : frontline-server --cert {} --key {}",
        cert.display(),
        key.display()
    );
    println!(
        "client url  : ?certhash={hex}   (hex; the client turns it into serverCertificateHashes)"
    );
    println!(
        "smoke page  : tools/dev/wt-smoke/index.html?url=https://127.0.0.1:7001/wt&certhash={hex}&hello={}&c1={}",
        smoke_hello(),
        smoke_c1()
    );
    Ok(())
}

fn hex_of(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Encoded C0 `Hello` for the current protocol version and embedded data hash.
fn smoke_hello() -> String {
    use frontline_protocol::msg::c0::{encode_c0_client, ClientMsgC0, Hello};
    let h = Hello {
        protocol_version: frontline_protocol::PROTOCOL_VERSION,
        data_hash: frontline_sim::GameData::embedded().hash,
        ticket: vec![],
        client_build: 0,
        caps: 0,
    };
    hex_of(&encode_c0_client(&ClientMsgC0::Hello(h)).unwrap_or_default())
}

/// One valid C1 `InputBundle` payload (seq 1).
fn smoke_c1() -> String {
    use frontline_protocol::msg::input::{encode_input_bundle, InputBundle, InputCmd};
    let cmd = InputCmd {
        seq: 1,
        tick: 1,
        move_x: 0,
        move_z: 0,
        yaw: 0,
        pitch: 0,
        buttons: 0,
        weapon_slot: 0,
        aux: 0,
        view_tick: 0,
    };
    let b = InputBundle {
        ack_snapshot_tick: 0,
        pred_hash: 0,
        cmds: [cmd].into_iter().collect(),
        fire: Default::default(),
    };
    hex_of(&encode_input_bundle(&b).unwrap_or_default())
}
