// src/core/resolver.rs

use crate::models::Project;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ResolverError {
    #[error(
        "No se encontró un proyecto (.axes/axes.toml) en este directorio ni en los superiores."
    )]
    ProjectNotFound,
    #[error("No se pudo leer el archivo del proyecto: {0}")]
    FileReadError(#[from] std::io::Error),
    #[error("El archivo project.toml está mal formado: {0}")]
    TomlParseError(#[from] toml::de::Error),
}

/// Busca y carga el proyecto `project.toml` a partir de una ruta dada.
/// Devuelve el `Project` y la ruta raíz del proyecto.
pub fn find_and_load_project(start_path: &Path) -> Result<(Project, PathBuf), ResolverError> {
    let project_root = find_project_root(start_path).ok_or(ResolverError::ProjectNotFound)?;
    let project_toml_path = project_root.join(".axes").join("axes.toml");

    log::info!("Cargando proyecto desde: {:?}", project_toml_path);

    let content = fs::read_to_string(&project_toml_path)?;
    let project: Project = toml::from_str(&content)?;

    Ok((project, project_root))
}

/// Busca el directorio raíz del proyecto (el que contiene `.project/`)
/// comenzando desde `start_path` y subiendo en el árbol de directorios.
fn find_project_root(start_path: &Path) -> Option<PathBuf> {
    let mut current_path = start_path.to_path_buf();

    loop {
        let project_dir = current_path.join(".axes");
        if project_dir.is_dir() && project_dir.join("axes.toml").is_file() {
            return Some(current_path);
        }

        if !current_path.pop() {
            // Si no podemos subir más (llegamos a la raíz), paramos.
            return None;
        }
    }
}

/// Una función de conveniencia para buscar desde el directorio de trabajo actual.
pub fn find_and_load_project_from_cwd() -> Result<(Project, PathBuf), ResolverError> {
    let cwd = env::current_dir().expect("No se pudo obtener el directorio actual.");
    find_and_load_project(&cwd)
}

pub fn load_project_at(path: &Path) -> Result<(Project, PathBuf), ResolverError> {
    let config_path = path.join(".axes").join("axes.toml");
    if !config_path.is_file() {
        return Err(ResolverError::ProjectNotFound);
    }
    log::info!("Cargando proyecto desde la ruta exacta: {:?}", config_path);
    let content = fs::read_to_string(&config_path)?;
    let project: Project = toml::from_str(&content)?;
    // La raíz del proyecto es el path que nos pasaron.
    Ok((project, path.to_path_buf()))
}
