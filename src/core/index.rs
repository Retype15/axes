// src/core/index.rs

use crate::config::get_index_path;
use crate::core::resolver::find_and_load_project;
use crate::models::Index;
use crate::models::Project;
use dialoguer::{Input, Select, console, theme::ColorfulTheme};
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum IndexError {
    #[error("Error al acceder a la ruta del índice: {0}")]
    PathError(String),
    #[error("Error al leer el archivo de índice: {0}")]
    FileReadError(#[from] std::io::Error),
    #[error("El archivo index.toml está mal formado: {0}")]
    TomlParseError(#[from] toml::de::Error),
    #[error("Error al serializar el índice a TOML: {0}")]
    TomlSerializeError(#[from] toml::ser::Error),
    #[error("Error al escribir en el archivo de índice: {0}")]
    FileWriteError(std::io::Error),
}

pub enum ValidatedProject {
    Success(Box<Project>, PathBuf),
    Cancelled(String), // El usuario canceló la operación, con un mensaje informativo.
}

pub fn load_index() -> Result<Index, IndexError> {
    let index_path = get_index_path().map_err(IndexError::PathError)?;

    if !index_path.exists() {
        log::info!("No se encontró index.toml, se usará un índice vacío.");
        return Ok(Index::default());
    }

    log::info!("Cargando índice desde: {:?}", index_path);
    let content = fs::read_to_string(index_path)?;
    let index: Index = toml::from_str(&content)?;

    Ok(index)
}

pub fn save_index(index: &Index) -> Result<(), IndexError> {
    let index_path = get_index_path().map_err(IndexError::PathError)?;
    log::info!("Guardando índice en: {:?}", index_path);

    let toml_content = toml::to_string_pretty(index)?;

    fs::write(&index_path, toml_content).map_err(IndexError::FileWriteError)?;

    Ok(())
}

/// Actualiza el campo `last_used` en el índice y lo guarda.
pub fn update_last_used(project_name: &str) -> Result<(), IndexError> {
    let mut index = load_index()?;
    index.last_used = Some(project_name.to_string());
    save_index(&index)
}

pub fn load_and_validate_project_by_name(project_name: &str) -> Result<ValidatedProject, String> {
    let mut index = load_index().map_err(|e| e.to_string())?;

    let project_root = match index.projects.get(project_name) {
        Some(path) => path.clone(),
        None => {
            return Err(format!(
                "Proyecto '{}' no encontrado en el índice.",
                project_name
            ));
        }
    };

    // --- LÓGICA DE VALIDACIÓN ---
    if project_root.join(".axes").join("axes.toml").is_file() {
        let (project, root) = find_and_load_project(&project_root).map_err(|e| e.to_string())?;
        return Ok(ValidatedProject::Success(Box::new(project), root));
    }

    // --- BUCLE DE REPARACIÓN ---
    loop {
        eprintln!(
            "\n{}",
            console::style(format!(
                "! Advertencia: El proyecto '{}' apunta a una ruta inválida.",
                project_name
            ))
            .yellow()
        );
        eprintln!("  Ruta registrada: {}", project_root.display());

        let theme = ColorfulTheme::default();
        let options = &["Relocalizar el proyecto", "Eliminar del índice", "Cancelar"];

        let choice = Select::with_theme(&theme)
            .with_prompt("La configuración del proyecto no se encontró. ¿Qué deseas hacer?")
            .items(options)
            .default(0)
            .interact()
            .map_err(|e| e.to_string())?;

        match choice {
            0 => {
                // Relocalizar
                let new_path_str = Input::<String>::with_theme(&theme)
                    .with_prompt("Introduce la nueva ruta raíz del proyecto")
                    .interact_text()
                    .map_err(|e| e.to_string())?;

                // Si el usuario no introduce nada, volvemos al menú.
                if new_path_str.is_empty() {
                    continue;
                }

                // Usamos `fs::canonicalize` que es más fiable que `PathBuf::canonicalize`
                let new_path = match std::fs::canonicalize(&new_path_str) {
                    Ok(p) => p,
                    Err(_) => {
                        eprintln!("{}", console::style("  ✖ La ruta proporcionada no es válida o no existe. Por favor, inténtalo de nuevo.").red());
                        let _ = console::Term::stdout().clear_screen();
                        continue; // Volver al inicio del bucle
                    }
                };

                if !new_path.join(".axes").join("axes.toml").is_file() {
                    eprintln!("{}", console::style("  ✖ La nueva ruta no contiene un proyecto de Axes válido. Por favor, inténtalo de nuevo.").red());
                    continue; // Volver al inicio del bucle
                }

                // ¡Éxito! Actualizamos, guardamos y salimos del bucle devolviendo el proyecto.
                index
                    .projects
                    .insert(project_name.to_string(), new_path.clone());
                save_index(&index).map_err(|e| e.to_string())?;
                eprintln!(
                    "{}",
                    console::style("✔ Ruta del proyecto actualizada.").green()
                );

                let (project, root) =
                    find_and_load_project(&new_path).map_err(|e| e.to_string())?;
                return Ok(ValidatedProject::Success(Box::new(project), root));
            }
            1 => {
                // Eliminar
                index.projects.remove(project_name);
                index.last_used = None; // También limpiar el último usado si era este
                save_index(&index).map_err(|e| e.to_string())?;
                return Ok(ValidatedProject::Cancelled(format!(
                    "Proyecto '{}' eliminado del índice.",
                    project_name
                )));
            }
            _ => {
                // Cancelar
                return Ok(ValidatedProject::Cancelled(
                    "Operación cancelada por el usuario.".to_string(),
                ));
            }
        }
    }
}
