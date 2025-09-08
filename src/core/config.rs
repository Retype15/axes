// src/core/config.rs

use crate::config as global_paths;
use crate::models::{Command, Project};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf; // Para evitar ambigüedad

/// Representa la vista fusionada de la configuración global y la del proyecto.
/// El resto del programa interactuará con esta struct.
#[derive(Debug, Clone)]
pub struct ResolvedConfig {
    pub project_name: String,
    pub project_root: PathBuf,
    pub version: Option<String>,
    pub description: Option<String>,
    pub commands: HashMap<String, Command>,
    pub options: HashMap<String, String>,
    pub vars: HashMap<String, String>,
    pub original_project: Project,
}

/// Carga el archivo de configuración global `axes.toml`.
/// Si no existe, devuelve un `Project` vacío por defecto.
fn load_global_config() -> Result<Project, String> {
    let config_path = global_paths::get_config_dir()?.join("axes.toml");
    if !config_path.exists() {
        return Ok(Project::default());
    }
    let content = fs::read_to_string(config_path).map_err(|e| e.to_string())?;
    toml::from_str(&content).map_err(|e| format!("Error al parsear el axes.toml global: {}", e))
}

/// Crea una configuración resuelta fusionando la configuración global y la del proyecto.
pub fn resolve_config(
    project_config: Project,
    project_root: PathBuf,
) -> Result<ResolvedConfig, String> {
    let global_config = load_global_config()?;

    // Lógica de fusión: El proyecto local siempre tiene prioridad.
    let mut resolved_vars = global_config.vars;
    resolved_vars.extend(project_config.vars.clone());

    let mut resolved_options = global_config.options;
    resolved_options.extend(project_config.options.clone());

    // Para los comandos, no fusionamos, el proyecto local los define por completo.
    let resolved_commands = project_config.commands.clone();

    Ok(ResolvedConfig {
        project_name: project_config.name.clone(),
        project_root,
        version: project_config.version.clone(),
        description: project_config.description.clone(),
        commands: resolved_commands,
        options: resolved_options,
        vars: resolved_vars,
        original_project: project_config,
    })
}
