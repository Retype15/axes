// src/core/templates.rs

use include_dir::{Dir, DirEntry, include_dir};
use std::fs;
use std::path::Path;

static TEMPLATES_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/templates");

pub fn apply_template(
    target_dir: &Path,
    template_name: &str,
    project_name: &str,
) -> Result<(), String> {
    log::info!(
        "Aplicando plantilla '{}' en {:?}",
        template_name,
        target_dir
    );

    let template_root = TEMPLATES_DIR
        .get_dir(template_name)
        .ok_or_else(|| format!("No se encontró la plantilla '{}' interna.", template_name))?;

    // El directorio base para la copia es el directorio de destino del proyecto.
    copy_dir_contents(template_root, target_dir, project_name)
}

fn copy_dir_contents(
    template_dir: &Dir,
    target_path: &Path,
    project_name: &str,
) -> Result<(), String> {
    // Asegurarse de que el directorio de destino existe
    fs::create_dir_all(target_path)
        .map_err(|e| format!("No se pudo crear el directorio {:?}: {}", target_path, e))?;

    for entry in template_dir.entries() {
        // La ruta de destino completa para esta entrada
        let final_target_path = target_path.join(entry.path().file_name().unwrap());

        match entry {
            DirEntry::Dir(d) => {
                // Si es un directorio, llamamos recursivamente
                copy_dir_contents(d, &final_target_path, project_name)?;
            }
            DirEntry::File(f) => {
                let file_name = f.path().file_name().unwrap().to_str().unwrap();

                if file_name.ends_with(".template") {
                    // Procesar archivo de plantilla
                    let final_name = file_name.strip_suffix(".template").unwrap();
                    let file_target_path = target_path.join(final_name);

                    log::debug!(
                        "Procesando plantilla {:?} a {:?}",
                        f.path(),
                        file_target_path
                    );

                    let content_utf8 = f
                        .contents_utf8()
                        .ok_or_else(|| format!("La plantilla {:?} no es UTF-8.", f.path()))?;

                    let processed_content = content_utf8.replace("{{name}}", project_name);

                    fs::write(&file_target_path, processed_content).map_err(|e| {
                        format!("No se pudo escribir {:?}: {}", file_target_path, e)
                    })?;
                } else {
                    // Copiar archivo binario/literal
                    let file_target_path = target_path.join(file_name);
                    log::debug!("Copiando archivo {:?} a {:?}", f.path(), file_target_path);

                    fs::write(&file_target_path, f.contents()).map_err(|e| {
                        format!("No se pudo escribir {:?}: {}", file_target_path, e)
                    })?;
                }
            }
        }
    }
    Ok(())
}
