//! Local fixture sender. This does not establish a session or expect a production reply.
use std::{net::SocketAddrV4, path::Path};
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let endpoint: SocketAddrV4 = args
        .next()
        .unwrap_or_else(|| "127.0.0.1:8765".into())
        .parse()?;
    if !endpoint.ip().is_loopback() || endpoint.port() == 0 {
        return Err("fixture sender requires a loopback IPv4 address and nonzero port".into());
    }
    if args.next().is_some() {
        return Err("usage: cargo run --example send_fixtures -- [127.0.0.1:8765]".into());
    }
    let socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await?;
    for name in ["hello.bin", "bad_crc.bin"] {
        let bytes = std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../protocol/fixtures/v1")
                .join(name),
        )?;
        let sent = socket.send_to(&bytes, endpoint).await?;
        println!("Sent {name}: {sent} bytes to {endpoint}; delivery is not acknowledged");
    }
    Ok(())
}
