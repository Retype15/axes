// src/core/context_resolver.rs

use crate::models::{GlobalIndex, IndexEntry, LastUsedCache};
use dialoguer::{Error as DialoguerError, Select, theme::ColorfulTheme};
use std::{env, fs, path::Path};
use thiserror::Error;
use uuid::Uuid;

use crate::constants::{AXES_DIR, PROJECT_CONFIG_FILENAME};
use crate::core::index_manager::{self, GLOBAL_PROJECT_UUID};

use bincode::error::DecodeError;

#[derive(Error, Debug)]
pub enum ContextError {
    #[error("Error de Ficheros: {0}")]
    Io(#[from] std::io::Error),
    #[error("Error de Índice: {0}")]
    Index(#[from] crate::core::index_manager::IndexError),
    #[error("Error al decodificar el caché: {0}")]
    BincodeDecode(#[from] bincode::error::DecodeError),
    #[error("Error al codificar el caché: {0}")]
    BincodeEncode(#[from] bincode::error::EncodeError),
    #[error("Error de Interfaz de Usuario: {0}")]
    Dialoguer(#[from] DialoguerError),
    #[error("Contexto vacío no proporcionado.")]
    EmptyContext,
    #[error("El contexto '**' solo puede ser usado al inicio de la ruta.")]
    GlobalRecentNotAtStart,
    #[error("El contexto '.' o '_' solo puede ser usado al inicio de la ruta.")]
    LocalPathNotAtStart,
    #[error("No se puede subir más en la jerarquía. Ya se está en un proyecto raíz.")]
    AlreadyAtRoot,
    #[error("No se ha utilizado ningún proyecto recientemente. No se puede resolver '**'.")]
    NoLastUsedProject,
    #[error(
        "El proyecto padre '{parent_name}' no ha utilizado ningún hijo recientemente. No se puede resolver '*'."
    )]
    NoLastUsedChild { parent_name: String },
    #[error("No se encontró ningún proyecto de axes en el directorio actual ni en los superiores.")]
    ProjectNotFoundFromPath,
    #[error("No se encontró ningún proyecto de axes en el directorio actual.")]
    ProjectNotFoundInCwd,
    #[error("No se encontró el proyecto raíz con el nombre '{name}'.")]
    RootProjectNotFound { name: String },
    #[error("El proyecto hijo '{child_name}' no se encontró para el padre '{parent_name}'.")]
    ChildProjectNotFound {
        child_name: String,
        parent_name: String,
    },
    #[error("Operación cancelada por el usuario.")]
    Cancelled,
}

type ContextResult<T> = Result<T, ContextError>;

/// Resuelve una ruta de proyecto a un UUID y un nombre cualificado.
pub fn resolve_context(context: &str, index: &GlobalIndex) -> ContextResult<(Uuid, String)> {
    let parts: Vec<&str> = context.split('/').filter(|s| !s.is_empty()).collect();
    if parts.is_empty() {
        return Err(ContextError::EmptyContext);
    }

    let mut resolved_parts: Vec<String> = Vec::new();

    // 1. Resolver la primera parte para obtener el estado inicial
    let (mut current_uuid, mut current_parent_uuid) = resolve_first_part(parts[0], index)?;
    resolved_parts.push(index.projects.get(&current_uuid).unwrap().name.clone());

    // 2. Iterar sobre el resto de las partes
    for part in parts.iter().skip(1) {
        let (next_uuid, next_parent_uuid) = match *part {
            "**" => return Err(ContextError::GlobalRecentNotAtStart),
            "." | "_" => return Err(ContextError::LocalPathNotAtStart),
            ".." => {
                let parent_uuid = current_parent_uuid.ok_or(ContextError::AlreadyAtRoot)?;
                let parent_entry = index.projects.get(&parent_uuid).unwrap(); // Seguro
                resolved_parts.pop(); // Quitar el nombre del hijo actual
                (parent_uuid, parent_entry.parent)
            }
            "*" => {
                let parent_entry = index.projects.get(&current_uuid).unwrap(); // Seguro
                let child_uuid = resolve_last_used_child(current_uuid, parent_entry, index)?;
                let child_entry = index.projects.get(&child_uuid).unwrap(); // Seguro
                resolved_parts.push(child_entry.name.clone());
                (child_uuid, Some(current_uuid))
            }
            name => {
                let parent_entry = index.projects.get(&current_uuid).unwrap(); // Seguro
                let child_uuid = find_child_by_name(current_uuid, parent_entry, name, index)?;
                let child_entry = index.projects.get(&child_uuid).unwrap(); // Seguro
                resolved_parts.push(child_entry.name.clone());
                (child_uuid, Some(current_uuid))
            }
        };
        current_uuid = next_uuid;
        current_parent_uuid = next_parent_uuid;
    }

    // FIXME: Update last used caches for the entire resolved path.
    update_last_used_caches(current_uuid, index)?;

    Ok((current_uuid, resolved_parts.join("/")))
}

/// Resuelve la primera parte de la ruta, que tiene reglas especiales.
fn resolve_first_part(part: &str, index: &GlobalIndex) -> ContextResult<(Uuid, Option<Uuid>)> {
    let uuid = match part {
        "**" => index.last_used.ok_or(ContextError::NoLastUsedProject)?,
        "." => find_project_from_path(&env::current_dir()?, true, index)?,
        "_" => find_project_from_path(&env::current_dir()?, false, index)?,
        // **NUEVA LÓGICA**: "global" es un nombre explícito, el resto son hijos implícitos de `global`.
        "global" => GLOBAL_PROJECT_UUID,
        name => {
            // Es una ruta implícita, buscar como hijo de `global`.
            let global_entry = index.projects.get(&GLOBAL_PROJECT_UUID).unwrap(); // Es seguro.
            find_child_by_name(GLOBAL_PROJECT_UUID, global_entry, name, index)?
        }
    };
    let entry = index.projects.get(&uuid).unwrap();
    Ok((uuid, entry.parent))
}

/// Resuelve '*' para un hijo, con fallback interactivo.
fn resolve_last_used_child(
    parent_uuid: Uuid,
    parent_entry: &IndexEntry,
    index: &GlobalIndex,
) -> ContextResult<Uuid> {
    let cache_path = parent_entry.path.join(AXES_DIR).join("last_used.cache.bin");
    if let Ok(Some(cache)) = read_last_used_cache(&cache_path)
        && let Some(uuid) = cache.child_uuid
    {
        log::debug!(
            "Último hijo usado '{}' encontrado en caché para '{}'.",
            uuid,
            parent_entry.name
        );
        return Ok(uuid);
    }

    // Fallback: no hay caché o está vacío. Preguntar al usuario.
    log::warn!(
        "No se encontró caché de último hijo usado para '{}'. Iniciando fallback interactivo.",
        parent_entry.name
    );
    let children: Vec<_> = index
        .projects
        .values()
        .filter(|e| e.parent == Some(parent_uuid))
        .collect();

    if children.is_empty() {
        return Err(ContextError::NoLastUsedChild {
            parent_name: parent_entry.name.clone(),
        });
    }

    let child_names: Vec<_> = children.iter().map(|e| e.name.as_str()).collect();
    println!(
        "El proyecto '{}' no tiene un hijo usado recientemente.",
        parent_entry.name
    );
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Por favor, selecciona un hijo para continuar:")
        .items(&child_names)
        .default(0)
        .interact_opt()?
        .ok_or(ContextError::Cancelled)?;

    let selected_name = child_names[selection];
    find_child_by_name(parent_uuid, parent_entry, selected_name, index)
}

/// Encuentra el UUID de un proyecto buscando desde una ruta del sistema de archivos.
fn find_project_from_path(
    path: &Path,
    search_up: bool,
    index: &GlobalIndex,
) -> ContextResult<Uuid> {
    let mut current_path_opt = Some(path);
    while let Some(p) = current_path_opt {
        let config_path = p.join(AXES_DIR).join(PROJECT_CONFIG_FILENAME);
        if config_path.is_file() {
            let canonical_p = p.canonicalize()?;
            if let Some((uuid, _)) = index.projects.iter().find(|(_, e)| e.path == canonical_p) {
                return Ok(*uuid);
            }
        }
        if !search_up {
            break;
        }
        current_path_opt = p.parent();
    }
    if search_up {
        Err(ContextError::ProjectNotFoundFromPath)
    } else {
        Err(ContextError::ProjectNotFoundInCwd)
    }
}

/// Encuentra el UUID de un hijo por su nombre (lógica movida de config_resolver).
fn find_child_by_name(
    parent_uuid: Uuid,
    parent_entry: &IndexEntry,
    child_name: &str,
    index: &GlobalIndex,
) -> ContextResult<Uuid> {
    index
        .projects
        .iter()
        .find(|(_, e)| e.parent == Some(parent_uuid) && e.name == child_name)
        .map(|(uuid, _)| *uuid)
        .ok_or_else(|| ContextError::ChildProjectNotFound {
            child_name: child_name.to_string(),
            parent_name: parent_entry.name.clone(),
        })
}

/// Lee el caché de "último usado" de un proyecto padre.
fn read_last_used_cache(path: &Path) -> ContextResult<Option<LastUsedCache>> {
    if !path.exists() {
        return Ok(None);
    }
    let bytes = fs::read(path)?;

    let decode_result: Result<(LastUsedCache, usize), _> =
        bincode::serde::decode_from_slice(&bytes, bincode::config::standard());

    match decode_result {
        Ok((cache, _)) => Ok(Some(cache)),
        Err(e) => {
            if !matches!(e, DecodeError::Io { .. }) {
                log::warn!(
                    "Caché de 'último usado' en '{}' está corrupto. Se regenerará. (Error: {})",
                    path.display(),
                    e
                );
                let _ = fs::remove_file(path);
                Ok(None)
            } else {
                Err(ContextError::BincodeDecode(e))
            }
        }
    }
}

/// Escribe el caché de "último usado" de un proyecto padre.
fn write_last_used_cache(path: &Path, cache: &LastUsedCache) -> ContextResult<()> {
    let cache_dir = path.parent().unwrap(); // Asegura que el directorio existe
    if !cache_dir.exists() {
        fs::create_dir_all(cache_dir)?;
    }
    let bytes = bincode::serde::encode_to_vec(cache, bincode::config::standard())?;
    fs::write(path, bytes)?;
    Ok(())
}

fn update_last_used_caches(final_uuid: Uuid, index: &GlobalIndex) -> ContextResult<()> {
    // 1. Actualizar el `last_used` global.
    let mut global_index = index_manager::load_and_ensure_global_project()?;
    global_index.last_used = Some(final_uuid);
    index_manager::save_global_index(&global_index)?;

    // 2. **NUEVO**: Actualizar los cachés de hijos (`*`) subiendo por el árbol.
    let mut current_entry = index.projects.get(&final_uuid).unwrap();
    let mut child_uuid_to_save = final_uuid;

    // Subir por la cadena de herencia
    while let Some(parent_uuid) = current_entry.parent {
        if let Some(parent_entry) = index.projects.get(&parent_uuid) {
            log::debug!(
                "Actualizando el 'último usado' para el padre '{}' a '{}'",
                parent_entry.name,
                child_uuid_to_save
            );
            let cache = LastUsedCache {
                child_uuid: Some(child_uuid_to_save),
            };
            let cache_path = parent_entry.path.join(AXES_DIR).join("last_used.cache.bin");

            // Llamar a la función que antes no se usaba
            write_last_used_cache(&cache_path, &cache)?;

            // Preparar para la siguiente iteración
            child_uuid_to_save = parent_uuid;
            current_entry = parent_entry;
        } else {
            // Si el padre no se encuentra en el índice (enlace roto), nos detenemos.
            break;
        }
    }

    Ok(())
}
