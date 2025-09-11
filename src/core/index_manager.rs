// src/core/index_manager.rs

use crate::core::paths;
use crate::models::{GlobalIndex, IndexEntry, ProjectRef};
use crate::constants::PROJECT_REF_FILENAME;
use std::{fs, path::PathBuf};
use std::collections::HashSet;
use thiserror::Error;
use uuid::Uuid;

pub const GLOBAL_PROJECT_UUID: Uuid = Uuid::nil();

#[derive(Error, Debug)]
pub enum IndexError {
    #[error("Error de Ficheros: {0}")]
    Io(#[from] std::io::Error),
    #[error("Error al parsear TOML en '{path}': {source}")]
    TomlParse {
        path: String,
        #[source]
        source: toml::de::Error,
    },
    #[error("Error al serializar a formato TOML: {0}")]
    TomlSerialize(#[from] toml::ser::Error),
    #[error("Error de rutas: {0}")]
    Path(#[from] crate::core::paths::PathError),
    // CORRECCIÓN: La variante NameAlreadyExists debe tener el campo `name`.
    #[error("El nombre de proyecto '{name}' ya está en uso por otro hijo del mismo padre.")]
    NameAlreadyExists { name: String },
    #[error("Error al codificar a formato binario: {0}")]
    BincodeEncode(#[from] bincode::error::EncodeError),
    #[error("Enlace de padre roto: el proyecto '{child_uuid}' apunta a un padre inexistente '{missing_parent_uuid}'.")]
    BrokenParentLink {
        child_uuid: Uuid,
        missing_parent_uuid: Uuid,
    },
}

type IndexResult<T> = Result<T, IndexError>;

/// Carga el índice global y asegura que la entrada para el proyecto 'global' exista.
pub fn load_and_ensure_global_project() -> IndexResult<GlobalIndex> {
    let mut index = load_global_index_internal()?;
    if !index.projects.contains_key(&GLOBAL_PROJECT_UUID) {
        log::warn!("Proyecto 'global' no encontrado en el índice. Creándolo ahora.");
        let config_dir = paths::get_axes_config_dir()?;
        
        let global_entry = IndexEntry {
            name: "global".to_string(),
            path: config_dir.clone(), // Clonar para usarla después
            parent: None,
        };
        index.projects.insert(GLOBAL_PROJECT_UUID, global_entry);

        // **NUEVO**: Crear los archivos físicos para el proyecto `global`.
        // 1. Crear el `axes.toml` por defecto.
        let axes_dir = config_dir.join(crate::constants::AXES_DIR);
        fs::create_dir_all(&axes_dir)?;
        let config_path = axes_dir.join(crate::constants::PROJECT_CONFIG_FILENAME);
        if !config_path.exists() {
            let default_config = crate::models::ProjectConfig::new();
            // Añadir configuración por defecto para 'open'
            // NOTA: Esto requerirá que los modelos se actualicen primero. Lo haremos después.
            let toml_string = toml::to_string_pretty(&default_config)?;
            fs::write(config_path, toml_string)?;
        }

        // 2. Crear su `project_ref.bin`.
        let project_ref = crate::models::ProjectRef {
            self_uuid: GLOBAL_PROJECT_UUID,
            parent_uuid: None,
            name: "global".to_string(),
        };
        write_project_ref(&config_dir, &project_ref)?;

        // Guardar el índice actualizado.
        save_global_index(&index)?;
    }
    Ok(index)
}

/// Añade una nueva entrada de proyecto al índice.
pub fn add_project_to_index(
    index: &mut GlobalIndex,
    name: String,
    path: PathBuf,
    parent_uuid: Option<Uuid>,
) -> IndexResult<Uuid> {
    // Si no se especifica un padre, se asume que es hijo de `global`.
    let final_parent_uuid = parent_uuid.unwrap_or(GLOBAL_PROJECT_UUID);

    // Validar que no haya otro hijo con el mismo nombre y el mismo padre.
    let name_exists = index.projects.values().any(|entry| {
        let is_sibling = if name == "global" {
            // 'global' no puede tener hermanos
            false
        } else {
            entry.parent == Some(final_parent_uuid) && entry.name == name
        };
        is_sibling
    });

    if name_exists {
        return Err(IndexError::NameAlreadyExists { name });
    }
    
    let new_uuid = Uuid::new_v4();
    let new_entry = IndexEntry {
        name,
        path,
        parent: Some(final_parent_uuid),
    };
    
    index.projects.insert(new_uuid, new_entry);
    Ok(new_uuid)
}

fn load_global_index_internal() -> IndexResult<GlobalIndex> {
    let path = paths::get_global_index_path()?;
    if !path.exists() {
        return Ok(GlobalIndex::default());
    }
    let content = fs::read_to_string(&path)?;
    toml::from_str(&content).map_err(|e| IndexError::TomlParse {
        path: path.display().to_string(),
        source: e,
    })
}




// OLD DEFS

/// Guarda el índice global en el disco.
pub fn save_global_index(index: &GlobalIndex) -> IndexResult<()> {
    let path = paths::get_global_index_path()?;
    let toml_string = toml::to_string_pretty(index)?;
    fs::write(path, toml_string)?;
    Ok(())
}

pub fn write_project_ref(
    project_root: &PathBuf,
    project_ref: &ProjectRef,
) -> IndexResult<()> {
    let axes_dir = project_root.join(crate::constants::AXES_DIR);
    if !axes_dir.exists() {
        fs::create_dir_all(&axes_dir)?;
    }
    let ref_path = axes_dir.join(PROJECT_REF_FILENAME);
    // **CORRECCIÓN**: Usar `?` directamente, ya que `IndexError` ahora puede convertirse desde `bincode::error::EncodeError`.
    let bytes = bincode::serde::encode_to_vec(project_ref, bincode::config::standard())?;
    fs::write(ref_path, bytes)?;
    Ok(())
}

pub fn find_cycle_from_node(
    start_node_uuid: Uuid,
    index: &GlobalIndex,
) -> Result<Option<Uuid>, IndexError> {
    let mut current_uuid_opt = Some(start_node_uuid);
    let mut visited_nodes = HashSet::new();

    while let Some(current_uuid) = current_uuid_opt {
        // Si no podemos insertar el nodo, es porque ya estaba. ¡Ciclo detectado!
        if !visited_nodes.insert(current_uuid) {
            return Ok(Some(current_uuid));
        }

        // Moverse al padre
        match index.projects.get(&current_uuid) {
            Some(current_entry) => {
                current_uuid_opt = current_entry.parent;
            }
            None => {
                // El nodo actual no existe en el índice, lo que significa que el
                // nodo anterior tenía un `parent_uuid` que apunta a la nada.
                return Err(IndexError::BrokenParentLink {
                    child_uuid: visited_nodes.iter().last().unwrap().clone(), // El último nodo válido que visitamos
                    missing_parent_uuid: current_uuid,
                });
            }
        }
    }

    // Si el bucle termina, llegamos a una raíz sin repetir nodos. No hay ciclo.
    Ok(None)
}