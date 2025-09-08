// src/models/mod.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(untagged)]
pub enum Command {
    Simple(String),                                 // "deploy = '...'"
    Extended { run: String, desc: Option<String> }, // "test = { run = '...', desc = '...' }"
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct Project {
    // --- Metadatos ---
    #[serde(default)]
    pub name: String,
    pub description: Option<String>,
    pub version: Option<String>,
    // Podríamos añadir más campos como tags, created, etc.

    // --- Secciones ---
    #[serde(default)] // Si [commands] no existe, será un HashMap vacío.
    pub commands: HashMap<String, Command>,

    #[serde(default)]
    pub options: HashMap<String, String>,

    #[serde(default)]
    pub vars: HashMap<String, String>,
}

#[derive(Deserialize, Serialize, Debug, Default)]
pub struct Index {
    #[serde(default)]
    pub projects: HashMap<String, PathBuf>,
    pub last_used: Option<String>, // Nuevo campo
}
