// src/constants.rs

/// El nombre del directorio que contiene la configuración de axes para un proyecto.
pub const AXES_DIR: &str = ".axes";

/// El nombre del archivo de configuración principal de un proyecto (dentro de .axes/).
pub const PROJECT_CONFIG_FILENAME: &str = "axes.toml";

/// El nombre del archivo de caché para la configuración resuelta de un proyecto (dentro de .axes/).
pub const CONFIG_CACHE_FILENAME: &str = "config.cache.bin";

/// El nombre del archivo de caché para los hijos de un proyecto (dentro de .axes/).
pub const CHILDREN_CACHE_FILENAME: &str = "children.cache.bin";

/// El nombre del archivo del índice global (en ~/.config/axes/).
pub const GLOBAL_INDEX_FILENAME: &str = "index.toml";

/// El nombre del archivo que contiene la identidad y referencias de un proyecto.
pub const PROJECT_REF_FILENAME: &str = "project_ref.bin";
