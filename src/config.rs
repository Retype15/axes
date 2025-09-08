// src/config.rs

use std::fs;
use std::path::PathBuf;

/// Devuelve la ruta al directorio de configuración de Axes.
/// Lo crea si no existe.
pub fn get_config_dir() -> Result<PathBuf, String> {
    let config_path = dirs::config_dir()
        .ok_or("No se pudo encontrar el directorio de configuración del sistema.")?
        .join("axes");
    log::info!("Directorio de config: {:?}", config_path);
    if !config_path.exists() {
        fs::create_dir_all(&config_path).map_err(|e| {
            format!(
                "No se pudo crear el directorio de configuración en {:?}: {}",
                config_path, e
            )
        })?;
    }

    Ok(config_path)
}

/// Devuelve la ruta al archivo index.toml.
pub fn get_index_path() -> Result<PathBuf, String> {
    get_config_dir().map(|dir| dir.join("index.toml"))
}
