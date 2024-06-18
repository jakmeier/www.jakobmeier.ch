use ed25519_compact::{PublicKey, Signature};

fn main() -> anyhow::Result<()> {
    println!("> Wasm signature verifier, featuring WASI.");
    println!("> Please enter the message text.");
    let message = read_line()?;

    println!("> Please enter the signature in hex encoding.");
    let hex_input = read_line()?;
    let binary_input = hex::decode(hex_input.clone())?;
    let signature = Signature::from_slice(&binary_input)?;

    println!("> Please enter the path to the certificate.");
    let path = read_line()?;
    let pk_vec = std::fs::read(&path)?;
    let pk_array: [u8; 32] = pk_vec.try_into().unwrap();
    let pk = PublicKey::new(pk_array);

    println!(">");
    println!("> Verifying ");
    println!(">           {message}");
    println!(">           {hex_input}");
    println!(">           {path}");
    match pk.verify(message, &signature) {
        Ok(()) => println!("valid"),
        Err(_) => println!("invalid"),
    }
    Ok(())
}

/// Read a line from stdin and remove trailing whitespace, such as \n
fn read_line() -> Result<String, anyhow::Error> {
    let mut line = String::new();
    std::io::stdin().read_line(&mut line)?;
    line.truncate(line.trim().len());
    Ok(line)
}
