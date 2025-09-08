// src/system/io.rs

use std::io::{self, Write};

/// Pide al usuario una entrada en la consola, mostrando un mensaje.
pub fn prompt(message: &str) -> io::Result<String> {
    print!("{}", message);
    io::stdout().flush()?; // Asegurarse de que el mensaje se muestre antes de leer.

    let mut buffer = String::new();
    io::stdin().read_line(&mut buffer)?;

    Ok(buffer.trim().to_string())
}
