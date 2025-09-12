// src/models.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

// --- MODELOS DE COMANDOS PÚBLICOS (PARA TOML) ---
// Estos son los que el usuario ve y usa en axes.toml

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(untagged)]
pub enum Runnable {
    Sequence(Vec<String>),
    Single(String),
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ExtendedCommand {
    pub run: Runnable,
    pub desc: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PlatformCommand {
    #[serde(default)]
    pub default: Option<Runnable>,
    pub windows: Option<Runnable>,
    pub linux: Option<Runnable>,
    pub macos: Option<Runnable>,
    pub desc: Option<String>,
}

/// Representa un comando en `axes.toml`. Usa `untagged` para una sintaxis flexible.
/// Es solo para deserializar desde TOML, no para serializar a bincode.
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(untagged)]
pub enum Command {
    Sequence(Vec<String>),
    Simple(String),
    Extended(ExtendedCommand),
    Platform(PlatformCommand),
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct OptionsConfig {
    // Campos explícitos para opciones clave
    pub at_start: Option<String>,
    pub at_exit: Option<String>,
    pub shell: Option<String>,

    // La sub-tabla `open_with`
    #[serde(default)]
    pub open_with: HashMap<String, String>,
}

// --- MODELOS DE `axes.toml` (Lo que se lee del archivo de configuración) ---

/// Representa la estructura deserializada de un archivo `axes.toml`.
/// Solo necesita `Deserialize`.
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct ProjectConfig {
    pub version: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub commands: HashMap<String, Command>,
    #[serde(default)]
    pub options: OptionsConfig,
    #[serde(default)]
    pub vars: HashMap<String, String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

impl ProjectConfig {
    pub fn new() -> Self {
        let mut commands = HashMap::new();
        commands.insert(
            "hello".to_string(),
            Command::Simple("echo 'Hello from axes!'".to_string()),
        );
        let mut open_with_defaults = HashMap::new();
        if cfg!(target_os = "windows") {
            open_with_defaults.insert("default".to_string(), "explorer .".to_string());
            open_with_defaults.insert("explorer".to_string(), "explorer .".to_string());
            open_with_defaults.insert("vsc".to_string(), "code .".to_string());
        } else if cfg!(target_os = "macos") {
            open_with_defaults.insert("default".to_string(), "open .".to_string());
            open_with_defaults.insert("finder".to_string(), "open .".to_string());
            open_with_defaults.insert("vsc".to_string(), "code .".to_string());
        } else {
            // Linux y otros
            open_with_defaults.insert("default".to_string(), "xdg-open .".to_string());
            open_with_defaults.insert("nautilus".to_string(), "nautilus .".to_string());
            open_with_defaults.insert("vsc".to_string(), "code .".to_string());
        }
        Self {
            version: Some("0.1.0".to_string()),
            description: Some("A new project managed by axes.".to_string()),
            commands: HashMap::new(), // Empezar sin comandos por defecto
            options: OptionsConfig {
                open_with: open_with_defaults,
                at_start: None,
                at_exit: None,
                shell: None,
            },
            ..Default::default()
        }
    }
}

// --- MODELOS DE ÍNDICE GLOBAL ---

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct IndexEntry {
    pub name: String,
    pub path: PathBuf,
    pub parent: Option<Uuid>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct GlobalIndex {
    #[serde(default)]
    pub projects: HashMap<Uuid, IndexEntry>,
    pub last_used: Option<Uuid>,
}

// --- MODELOS DE CACHÉ LOCAL ---

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ChildCache {
    #[serde(default)]
    pub children: HashMap<String, Uuid>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct LastUsedCache {
    pub child_uuid: Option<Uuid>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProjectRef {
    pub self_uuid: Uuid,
    pub parent_uuid: Option<Uuid>,
    pub name: String,
}

// --- MODELOS EN MEMORIA (Nuestra representación de trabajo interna) ---

/// La vista final y fusionada de la configuración.
/// No necesita `Serialize` o `Deserialize` porque NUNCA se escribe/lee directamente.
#[derive(Debug, Clone)]
pub struct ResolvedConfig {
    pub uuid: Uuid,
    pub qualified_name: String,
    pub project_root: PathBuf,
    pub version: Option<String>,
    pub description: Option<String>,
    pub commands: HashMap<String, Command>,
    pub options: OptionsConfig,
    pub vars: HashMap<String, String>,
    pub env: HashMap<String, String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ShellConfig {
    pub path: PathBuf,
    pub interactive_args: Option<Vec<String>>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct ShellsConfig {
    #[serde(default)]
    pub shells: HashMap<String, ShellConfig>,
}

// --- MODELOS SUSTITUTOS DE SERIALIZACIÓN (Para el caché binario) ---
// Estos son privados al crate y solo se usan para la conversión.

/// Un `enum` sustituto para `Command` que es explícito y serializable por `bincode`.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) enum SerializableCommand {
    Sequence(Vec<String>),
    Simple(String),
    Extended(ExtendedCommand),
    Platform(PlatformCommand),
}

/// Un wrapper para `SystemTime` que es serializable.
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub(crate) struct SerializableSystemTime(Duration);

/// El sustituto para `ResolvedConfig` que usa tipos serializables (`String` en vez de `PathBuf`).
#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct SerializableResolvedConfig {
    pub uuid: Uuid,
    pub qualified_name: String,
    pub project_root: String,
    pub version: Option<String>,
    pub description: Option<String>,
    pub commands: HashMap<String, SerializableCommand>,
    pub options: OptionsConfig,
    pub vars: HashMap<String, String>,
    pub env: HashMap<String, String>,
}

/// El contenedor principal para el caché de configuración que se escribe en disco.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct SerializableConfigCache {
    pub resolved_config: SerializableResolvedConfig,
    pub dependencies: HashMap<String, SerializableSystemTime>,
}

// --- LÓGICA DE CONVERSIÓN ENTRE MODELOS DE TRABAJO Y MODELOS SERIALIZABLES ---

// Command <-> SerializableCommand
impl From<&Command> for SerializableCommand {
    fn from(value: &Command) -> Self {
        match value {
            Command::Sequence(s) => SerializableCommand::Sequence(s.clone()),
            Command::Simple(s) => SerializableCommand::Simple(s.clone()),
            Command::Extended(e) => SerializableCommand::Extended(e.clone()),
            Command::Platform(p) => SerializableCommand::Platform(p.clone()),
        }
    }
}

impl From<SerializableCommand> for Command {
    fn from(value: SerializableCommand) -> Self {
        match value {
            SerializableCommand::Sequence(s) => Command::Sequence(s),
            SerializableCommand::Simple(s) => Command::Simple(s),
            SerializableCommand::Extended(e) => Command::Extended(e),
            SerializableCommand::Platform(p) => Command::Platform(p),
        }
    }
}

// ResolvedConfig <-> SerializableResolvedConfig
impl From<&ResolvedConfig> for SerializableResolvedConfig {
    fn from(value: &ResolvedConfig) -> Self {
        Self {
            uuid: value.uuid,
            qualified_name: value.qualified_name.clone(),
            project_root: value.project_root.to_string_lossy().into_owned(),
            version: value.version.clone(),
            description: value.description.clone(),
            commands: value
                .commands
                .iter()
                .map(|(k, v)| (k.clone(), v.into()))
                .collect(),
            options: value.options.clone(),
            vars: value.vars.clone(),
            env: value.env.clone(),
        }
    }
}

impl From<SerializableResolvedConfig> for ResolvedConfig {
    fn from(value: SerializableResolvedConfig) -> Self {
        Self {
            uuid: value.uuid,
            qualified_name: value.qualified_name,
            project_root: PathBuf::from(value.project_root),
            version: value.version,
            description: value.description,
            commands: value
                .commands
                .into_iter()
                .map(|(k, v)| (k, v.into()))
                .collect(),
            options: value.options,
            vars: value.vars,
            env: value.env,
        }
    }
}

// SystemTime <-> SerializableSystemTime
impl From<SystemTime> for SerializableSystemTime {
    fn from(time: SystemTime) -> Self {
        Self(time.duration_since(UNIX_EPOCH).unwrap_or_default())
    }
}

impl From<SerializableSystemTime> for SystemTime {
    fn from(time: SerializableSystemTime) -> Self {
        UNIX_EPOCH + time.0
    }
}
