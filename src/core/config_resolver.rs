// src/core/config_resolver.rs

use crate::constants::{AXES_DIR, CONFIG_CACHE_FILENAME, PROJECT_CONFIG_FILENAME};
use crate::models::{
    GlobalIndex, IndexEntry, OptionsConfig, ProjectConfig, ResolvedConfig, SerializableConfigCache,
};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use thiserror::Error;
use uuid::Uuid;

use bincode::error::DecodeError;

#[derive(Error, Debug)]
pub enum ResolverError {
    #[error("Error de Ficheros: {0}")]
    Io(#[from] std::io::Error),
    #[error("Error al parsear TOML en '{path}': {source}")]
    TomlParse {
        path: String,
        #[source]
        source: toml::de::Error,
    },
    #[error("Error al decodificar el caché: {0}")]
    BincodeDecode(#[from] bincode::error::DecodeError),
    #[error("Error al codificar el caché: {0}")]
    BincodeEncode(#[from] bincode::error::EncodeError),
    #[error("Error de rutas: {0}")]
    Path(#[from] crate::core::paths::PathError),
    #[error("El proyecto con UUID '{uuid}' referenciado en el índice no fue encontrado.")]
    UuidNotFoundInIndex { uuid: Uuid },
    #[error("No se encontró el archivo de configuración para el proyecto '{name}' en '{path}'.")]
    ConfigFileNotFound { name: String, path: String },
}

type ResolverResult<T> = Result<T, ResolverError>;

// --- FUNCIÓN PÚBLICA PRINCIPAL ---

pub fn resolve_config_for_uuid(
    target_uuid: Uuid,
    qualified_name: String,
    index: &GlobalIndex,
) -> ResolverResult<ResolvedConfig> {
    let leaf_entry = index
        .projects
        .get(&target_uuid)
        .ok_or(ResolverError::UuidNotFoundInIndex { uuid: target_uuid })?;

    let config_cache_path = leaf_entry.path.join(AXES_DIR).join(CONFIG_CACHE_FILENAME);

    if let Some(cached_config) =
        read_and_validate_config_cache(&config_cache_path, &qualified_name)?
    {
        log::debug!(
            "Caché de configuración válido encontrado para '{}'.",
            qualified_name
        );
        return Ok(cached_config);
    }
    log::debug!(
        "Caché de config inválido o no encontrado. Resolviendo '{}'...",
        qualified_name
    );

    let inheritance_chain = build_inheritance_chain(target_uuid, index)?;

    let dependencies = inheritance_chain
        .iter()
        .map(|(entry, _)| {
            let config_path = entry.path.join(AXES_DIR).join(PROJECT_CONFIG_FILENAME);
            let metadata = fs::metadata(&config_path)?;
            Ok((config_path, metadata.modified()?))
        })
        .collect::<ResolverResult<HashMap<_, _>>>()?;

    let configs_in_chain: Vec<ProjectConfig> =
        inheritance_chain.into_iter().map(|(_, p)| p).collect();
    let mut resolved_config = merge_chain_into_config(configs_in_chain);

    resolved_config.uuid = target_uuid;
    resolved_config.qualified_name = qualified_name;
    resolved_config.project_root = leaf_entry.path.clone();

    write_config_cache(&config_cache_path, &resolved_config, dependencies)?;
    log::debug!(
        "Nuevo caché de config guardado en '{}'.",
        config_cache_path.display()
    );

    Ok(resolved_config)
}

// --- LÓGICA DE HERENCIA (ASCENDENTE) ---

fn build_inheritance_chain<'a>(
    leaf_uuid: Uuid,
    index: &'a GlobalIndex,
) -> ResolverResult<Vec<(&'a IndexEntry, ProjectConfig)>> {
    let mut chain = Vec::new();
    let mut current_uuid_opt = Some(leaf_uuid);

    while let Some(current_uuid) = current_uuid_opt {
        let entry = index
            .projects
            .get(&current_uuid)
            .ok_or(ResolverError::UuidNotFoundInIndex { uuid: current_uuid })?;

        let config = load_project_config(entry)?;
        chain.push((entry, config));

        current_uuid_opt = entry.parent;
    }

    chain.reverse();
    Ok(chain)
}

// --- LÓGICA DE FUSIÓN ---

fn merge_chain_into_config(chain: Vec<ProjectConfig>) -> ResolvedConfig {
    let mut resolved = ResolvedConfig {
        uuid: Uuid::nil(),
        qualified_name: String::new(),
        project_root: PathBuf::new(),
        version: None,
        description: None,
        commands: HashMap::new(),
        options: OptionsConfig::default(),
        vars: HashMap::new(),
        env: HashMap::new(),
    };

    for config in chain {
        resolved.version = config.version.or(resolved.version);
        resolved.description = config.description.or(resolved.description);
        resolved.options.at_start = config.options.at_start.or(resolved.options.at_start);
        resolved.options.at_exit = config.options.at_exit.or(resolved.options.at_exit);
        resolved.options.shell = config.options.shell.or(resolved.options.shell);
        resolved.options.open_with.extend(config.options.open_with);
        resolved.vars.extend(config.vars);
        resolved.env.extend(config.env);
        resolved.commands = config.commands;
    }

    resolved
}

// --- LÓGICA DE CARGA Y CACHÉ ---

fn load_project_config(entry: &IndexEntry) -> ResolverResult<ProjectConfig> {
    let config_path = entry.path.join(AXES_DIR).join(PROJECT_CONFIG_FILENAME);
    if !config_path.is_file() {
        return Err(ResolverError::ConfigFileNotFound {
            name: entry.name.clone(),
            path: config_path.display().to_string(),
        });
    }
    let content = fs::read_to_string(&config_path)?;
    toml::from_str(&content).map_err(|e| ResolverError::TomlParse {
        path: config_path.display().to_string(),
        source: e,
    })
}

fn read_and_validate_config_cache(
    cache_path: &Path,
    expected_name: &str,
) -> ResolverResult<Option<ResolvedConfig>> {
    if !cache_path.exists() {
        return Ok(None);
    }
    let cached_bytes = fs::read(cache_path)?;

    let decode_result: Result<(SerializableConfigCache, usize), _> =
        bincode::serde::decode_from_slice(&cached_bytes, bincode::config::standard());

    let serializable_cache = match decode_result {
        Ok((cache, _)) => cache, // Asigna directamente el valor que nos interesa
        Err(e) => {
            if !matches!(e, DecodeError::Io { .. }) {
                log::warn!(
                    "Caché de configuración en '{}' está corrupto o desactualizado. Se regenerará. (Error: {})",
                    cache_path.display(),
                    e
                );
                let _ = fs::remove_file(cache_path);
                return Ok(None);
            }
            return Err(ResolverError::BincodeDecode(e));
        }
    };

    if serializable_cache.resolved_config.qualified_name != expected_name {
        log::debug!("El nombre cualificado del caché no coincide. Caché inválido.");
        return Ok(None);
    }

    for (path_str, cached_mod_time_serializable) in serializable_cache.dependencies.iter() {
        let path = PathBuf::from(path_str);
        if !path.exists() {
            log::debug!(
                "Dependencia de caché '{}' no existe. Caché inválido.",
                path.display()
            );
            return Ok(None);
        }
        let current_mod_time = fs::metadata(&path)?.modified()?;
        let cached_mod_time: SystemTime = (*cached_mod_time_serializable).into();

        if current_mod_time > cached_mod_time {
            log::debug!(
                "Dependencia de caché '{}' ha sido modificada. Caché inválido.",
                path.display()
            );
            return Ok(None);
        }
    }

    Ok(Some(serializable_cache.resolved_config.into()))
}

fn write_config_cache(
    cache_path: &Path,
    config: &ResolvedConfig,
    dependencies: HashMap<PathBuf, SystemTime>,
) -> ResolverResult<()> {
    let cache_dir = cache_path.parent().unwrap();
    if !cache_dir.exists() {
        fs::create_dir_all(cache_dir)?;
    }

    let serializable_deps = dependencies
        .into_iter()
        .map(|(path, time)| (path.to_string_lossy().into_owned(), time.into()))
        .collect();

    let cache_data = SerializableConfigCache {
        resolved_config: config.into(),
        dependencies: serializable_deps,
    };

    let bytes = bincode::serde::encode_to_vec(cache_data, bincode::config::standard())?;
    fs::write(cache_path, &bytes)?;
    Ok(())
}
