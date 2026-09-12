use base64::Engine;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let config: serde_json::Value = serde_json::from_slice(&std::fs::read(&args[3])?)?;
    let decode = |s: &str| -> Result<String, Box<dyn std::error::Error>> {
        Ok(String::from_utf8(
            base64::engine::general_purpose::STANDARD.decode(s.trim())?,
        )?)
    };
    let key = decode(
        config["plugins"]["updater"]["pubkey"]
            .as_str()
            .ok_or("missing public key")?,
    )?;
    let signature = decode(&std::fs::read_to_string(&args[2])?)?;
    minisign_verify::PublicKey::decode(&key)?.verify(
        &std::fs::read(&args[1])?,
        &minisign_verify::Signature::decode(&signature)?,
        false,
    )?;
    Ok(())
}
