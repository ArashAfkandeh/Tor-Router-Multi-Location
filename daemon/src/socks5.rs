use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub async fn handle_socks5_auth(
    client: &mut TcpStream,
    expected_user: &str,
    expected_pass: &str,
) -> bool {
    let mut buf = [0u8; 512];
    
    // Read greeting
    if client.read_exact(&mut buf[0..2]).await.is_err() { return false; }
    if buf[0] != 0x05 { return false; }
    let nmethods = buf[1] as usize;
    if client.read_exact(&mut buf[0..nmethods]).await.is_err() { return false; }
    
    // Check if auth is configured
    let require_auth = !expected_user.is_empty() || !expected_pass.is_empty();

    if require_auth {
        if !buf[0..nmethods].contains(&0x02) {
            let _ = client.write_all(&[0x05, 0xFF]).await; // No acceptable methods
            return false;
        }
        
        // Reply with 0x02 (Username/Password)
        if client.write_all(&[0x05, 0x02]).await.is_err() { return false; }
        
        // Read auth request
        if client.read_exact(&mut buf[0..2]).await.is_err() { return false; }
        if buf[0] != 0x01 { return false; } // Auth version must be 1
        let ulen = buf[1] as usize;
        if client.read_exact(&mut buf[0..ulen]).await.is_err() { return false; }
        let uname = String::from_utf8_lossy(&buf[0..ulen]).to_string();
        
        if client.read_exact(&mut buf[0..1]).await.is_err() { return false; }
        let plen = buf[0] as usize;
        if client.read_exact(&mut buf[0..plen]).await.is_err() { return false; }
        let pass = String::from_utf8_lossy(&buf[0..plen]).to_string();
        
        if uname == expected_user && pass == expected_pass {
            // Success
            if client.write_all(&[0x01, 0x00]).await.is_err() { return false; }
        } else {
            // Failure
            let _ = client.write_all(&[0x01, 0x01]).await;
            return false;
        }
    } else {
        // No auth required, reply with 0x00 (No authentication required)
        if !buf[0..nmethods].contains(&0x00) {
            let _ = client.write_all(&[0x05, 0xFF]).await;
            return false;
        }
        if client.write_all(&[0x05, 0x00]).await.is_err() { return false; }
    }
    
    true
}

pub async fn fake_tor_handshake(tor: &mut TcpStream) -> bool {
    // Send NO AUTH greeting to Tor
    if tor.write_all(&[0x05, 0x01, 0x00]).await.is_err() { return false; }
    
    let mut buf = [0u8; 2];
    if tor.read_exact(&mut buf).await.is_err() { return false; }
    if buf[0] != 0x05 || buf[1] != 0x00 { return false; }
    
    true
}
