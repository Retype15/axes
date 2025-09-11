// src/core/graph_display.rs

use crate::models::{GlobalIndex, IndexEntry};
use std::collections::HashMap;
use uuid::Uuid;

/// Muestra un árbol ASCII de todos los proyectos registrados.
pub fn display_project_tree(index: &GlobalIndex) {
    if index.projects.is_empty() {
        println!("\nNo hay proyectos registrados. Usa 'axes init <nombre>' para empezar.");
        return;
    }

    // 1. Construir un mapa de relaciones padre -> lista de (UUID, &IndexEntry) de sus hijos
    let mut children_map: HashMap<Option<Uuid>, Vec<(Uuid, &IndexEntry)>> = HashMap::new();
    for (uuid, entry) in &index.projects {
        children_map.entry(entry.parent).or_default().push((*uuid, entry));
    }

    // Opcional: Ordenar los hijos alfabéticamente para una salida consistente
    for children in children_map.values_mut() {
        children.sort_by_key(|(_, entry)| &entry.name);
    }

    // 2. Obtener las raíces (proyectos sin padre)
    if let Some(roots) = children_map.get(&None) {
        println!("\nÁrbol de Proyectos Registrados:");
        for (i, (uuid, root_entry)) in roots.iter().enumerate() {
            let is_last = i == roots.len() - 1;
            print_node(*uuid, root_entry, index, &children_map, "", is_last);
        }
    } else {
        println!("\nAdvertencia: No se encontraron proyectos raíz, pero hay proyectos registrados.");
        println!("Esto puede indicar un índice corrupto.");
    }
}

/// Función recursiva para imprimir un nodo del árbol y sus descendientes.
fn print_node(
    uuid: Uuid,
    entry: &IndexEntry,
    index: &GlobalIndex,
    children_map: &HashMap<Option<Uuid>, Vec<(Uuid, &IndexEntry)>>,
    prefix: &str,
    is_last: bool,
) {
    let connector = if is_last { "└─" } else { "├─" };
    
    // Comprobar si es el último proyecto usado globalmente
    let last_used_marker = if index.last_used == Some(uuid) { " (**)" } else { "" };

    println!("{}{}{} [{}] {}", prefix, connector, entry.name, entry.path.display(), last_used_marker);

    // Preparar el prefijo para los hijos de este nodo
    let child_prefix = format!("{}{}", prefix, if is_last { "   " } else { "│  " });

    // Recursión sobre los hijos
    if let Some(children) = children_map.get(&Some(uuid)) {
        for (i, (child_uuid, child_entry)) in children.iter().enumerate() {
            let is_last_child = i == children.len() - 1;
            print_node(*child_uuid, child_entry, index, children_map, &child_prefix, is_last_child);
        }
    }
}