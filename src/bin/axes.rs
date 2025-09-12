// src/bin/axes.rs

use anyhow::Context;
use anyhow::Result;
use anyhow::anyhow;
use clap::Parser;
use std::{env, fs};
use uuid::Uuid;

use axes::cli::Cli;
use axes::core::interpolator::Interpolator;
use axes::models::Runnable;
use axes::system::executor;
use axes::system::shell;

use axes::constants::{AXES_DIR, PROJECT_CONFIG_FILENAME};
use axes::core::graph_display;
use axes::core::{config_resolver, context_resolver, index_manager};
use axes::models::{Command as ProjectCommand, ProjectConfig, ProjectRef, ResolvedConfig};

/// El punto de entrada principal de la aplicación.
fn main() {
    // Inicializar el logger. Para ver los logs, ejecuta con `RUST_LOG=debug axes ...`
    env_logger::init();

    // Parsear los argumentos de la línea de comandos.
    let cli = Cli::parse();

    // Ejecutar la lógica principal y manejar cualquier error.
    if let Err(e) = run_cli(cli) {
        // Usamos `eprintln` para escribir en stderr, que es la práctica estándar para errores.
        // El formato `{:?}` con `anyhow` proporciona un stack trace útil.
        eprintln!("\nError: {:?}", e);
        std::process::exit(1);
    }
}

/// El despachador principal de la aplicación.
fn run_cli(cli: Cli) -> Result<()> {
    log::debug!("CLI args parsed: {:?}", cli);

    match cli.context_or_action {
        None => {
            println!("TODO: Lanzar la interfaz de usuario interactiva (TUI).");
            Ok(())
        }
        Some(context_or_action) => {
            // 1. Comprobar si es una acción que no necesita contexto.
            match context_or_action.as_str() {
                "init" => {
                    // `init` se maneja por separado.
                    return handle_init(cli.action_or_arg, cli.args);
                }
                // Si `tree` se llama sin contexto, asumimos `global tree`.
                "tree" => {
                    let index = index_manager::load_and_ensure_global_project()?;
                    let global_uuid = index_manager::GLOBAL_PROJECT_UUID;
                    let global_config = config_resolver::resolve_config_for_uuid(
                        global_uuid,
                        "global".to_string(),
                        &index,
                    )?;
                    return handle_tree(&global_config);
                }
                // Aquí se podrían añadir otros comandos globales como `register` en el futuro.

                // 2. Si no es una acción global, asumimos que es un contexto de proyecto.
                project_context => {
                    let index = index_manager::load_and_ensure_global_project()?;
                    let (uuid, qualified_name) =
                        context_resolver::resolve_context(project_context, &index)?;
                    let config =
                        config_resolver::resolve_config_for_uuid(uuid, qualified_name, &index)?;
                    log::info!("Proyecto '{}' resuelto con éxito.", config.qualified_name);

                    // El segundo argumento es la acción (o se infiere).
                    handle_project_action(config, cli.action_or_arg, cli.args)
                }
            }
        }
    }
}

fn handle_init(name_arg: Option<String>, args: Vec<String>) -> Result<()> {
    let project_name = name_arg
        .ok_or_else(|| anyhow!("El comando 'init' requiere un nombre para el nuevo proyecto."))?;

    // Parseo simple de argumentos para --parent
    let mut parent_context: Option<String> = None;
    if let Some(pos) = args.iter().position(|r| r == "--parent") {
        parent_context = args.get(pos + 1).cloned();
    }

    let current_dir = env::current_dir()?;
    println!(
        "Inicializando proyecto '{}' en {}",
        project_name,
        current_dir.display()
    );

    // 1. Validar que no exista ya un directorio .axes en el directorio actual
    let axes_dir = current_dir.join(AXES_DIR);
    if axes_dir.exists() {
        return Err(anyhow!(
            "Ya existe un directorio '.axes' en esta ubicación."
        ));
    }

    // 2. Cargar índice y resolver el padre (si se especificó)
    let mut index = index_manager::load_and_ensure_global_project()?;
    let final_parent_uuid: Uuid = match parent_context {
        Some(context) => {
            println!("Resolviendo padre '{}'...", context);
            let (uuid, qualified_name) = context_resolver::resolve_context(&context, &index)?;
            println!(
                "Proyecto padre '{}' encontrado (UUID: {}).",
                qualified_name, uuid
            );
            uuid
        }
        None => {
            println!(
                "No se especificó padre. Se enlazará al proyecto 'global'. (UUID: {})",
                index_manager::GLOBAL_PROJECT_UUID
            );
            index_manager::GLOBAL_PROJECT_UUID
        }
    };

    // 3. Añadir el nuevo proyecto al índice
    let canonical_path = current_dir.canonicalize()?;
    let new_uuid = index_manager::add_project_to_index(&mut index, project_name.clone(), canonical_path.clone(), Some(final_parent_uuid))
        .context("No se pudo añadir el proyecto al índice global. Podría haber un proyecto hermano con el mismo nombre.")?;

    // 4. Crear la estructura de archivos del proyecto en el disco
    fs::create_dir_all(&axes_dir)?;
    let config_path = axes_dir.join(PROJECT_CONFIG_FILENAME);
    let default_config = ProjectConfig::new();
    let toml_string = toml::to_string_pretty(&default_config)?;
    fs::write(&config_path, toml_string)?;

    // 5. Crear y guardar el archivo de referencia local (`project_ref.bin`)
    let project_ref = ProjectRef {
        self_uuid: new_uuid,
        parent_uuid: Some(final_parent_uuid), // El padre definitivo
        name: project_name.clone(),
    };
    index_manager::write_project_ref(&canonical_path, &project_ref)
        .context("No se pudo escribir el archivo de referencia del proyecto (project_ref.bin).")?;

    // 6. Guardar el índice global actualizado
    index_manager::save_global_index(&index)
        .context("No se pudo guardar el índice global actualizado.")?;

    println!("\n✔ ¡Éxito!");
    println!(
        "  Proyecto '{}' creado con UUID: {}",
        project_name, new_uuid
    );
    println!("  Configuración creada en: {}", config_path.display());
    println!(
        "  Identidad local guardada en: .axes/{}",
        axes::constants::PROJECT_REF_FILENAME
    );
    println!("  Registrado correctamente en el índice global.");

    Ok(())
}

/// Maneja las acciones que operan sobre una configuración de proyecto ya resuelta.
fn handle_project_action(
    config: ResolvedConfig,
    action_or_arg: Option<String>,
    args: Vec<String>,
) -> Result<()> {
    let action = action_or_arg.unwrap_or_else(|| {
        if config.qualified_name == "global" {
            "tree".to_string()
        } else {
            "start".to_string()
        }
    });

    log::debug!(
        "Manejando acción '{}' para el proyecto '{}'",
        action,
        config.qualified_name
    );

    match action.as_str() {
        "tree" => handle_tree(&config),
        "start" => handle_start(&config),
        "run" => {
            let script_name = args.get(0).cloned();
            let params = args.into_iter().skip(1).collect();
            handle_run(&config, script_name, params)
        }
        "info" => handle_info(&config),
        "open" => handle_open(&config, args),
        "rename" => handle_rename(&config, args),
        "link" => handle_link(&config, args),

        script_name if config.commands.contains_key(script_name) => {
            handle_run(&config, Some(action), args)
        }

        unknown => {
            anyhow::bail!(
                "Acción desconocida '{}' para el proyecto '{}'.",
                unknown,
                config.qualified_name
            );
        }
    }
}

// --- MANEJADORES DE ACCIONES (Implementaciones y Placeholders) ---

/// Muestra el arbol de proyectos raíz.
fn handle_tree(config: &ResolvedConfig) -> Result<()> {
    println!("\nMostrando árbol desde: '{}'", config.qualified_name);
    let index = index_manager::load_and_ensure_global_project()?;

    // Llamar a la nueva versión de `display_project_tree`
    graph_display::display_project_tree(&index, Some(config.uuid));

    Ok(())
}

fn handle_link(config: &ResolvedConfig, args: Vec<String>) -> Result<()> {
    // 1. Obtener el contexto del nuevo padre.
    let new_parent_context = args
        .get(0)
        .ok_or_else(|| anyhow!("El comando 'link' requiere el contexto del nuevo padre."))?
        .trim();

    if new_parent_context.is_empty() {
        return Err(anyhow!("El contexto del nuevo padre no puede estar vacío."));
    }
    // No validamos caracteres de ruta aquí porque es un contexto, no un nombre directo.

    println!(
        "Intentando mover '{}' a ser hijo de '{}'...",
        config.qualified_name, new_parent_context
    );

    // 2. Cargar el índice global y resolver el UUID del nuevo padre.
    let mut index = index_manager::load_and_ensure_global_project()?;
    let (new_parent_uuid, new_parent_qualified_name) =
        context_resolver::resolve_context(new_parent_context, &index).context(format!(
            "No se pudo resolver el contexto del nuevo padre '{}'.",
            new_parent_context
        ))?;

    // 3. Validaciones críticas (en el `index_manager`):
    //    a. Anti-Ciclos
    //    b. Anti-Colisión de Nombres de Hermano
    index_manager::link_project(&mut index, config.uuid, new_parent_uuid).context(format!(
        "No se pudo establecer el enlace para el proyecto '{}'.",
        config.qualified_name
    ))?;

    // 4. Guardar el índice global modificado.
    index_manager::save_global_index(&index)
        .context("No se pudo guardar el índice global actualizado.")?;

    // 5. Actualizar el `project_ref.bin` local (usando `get_or_create_project_ref`)
    let mut project_ref =
        index_manager::get_or_create_project_ref(&config.project_root, config.uuid, &index)
            .context(
                "No se pudo obtener o crear la referencia local del proyecto (`project_ref.bin`).",
            )?;

    project_ref.parent_uuid = Some(new_parent_uuid);
    if let Err(e) = index_manager::write_project_ref(&config.project_root, &project_ref) {
        eprintln!(
            "\nAdvertencia: El proyecto fue enlazado en el índice global, pero no se pudo actualizar el archivo de referencia local `project_ref.bin`: {}",
            e
        );
    }

    println!("\n✔ ¡Éxito!");
    println!(
        "El proyecto '{}' ahora es hijo de '{}'.",
        config.qualified_name, new_parent_qualified_name
    );
    println!("Nota: los cachés se regenerarán automáticamente en la próxima resolución.");

    Ok(())
}

/// Inicia una sesión de terminal interactiva para el proyecto.
fn handle_start(config: &ResolvedConfig) -> Result<()> {
    println!("\nIniciando sesión para '{}'...", config.qualified_name);

    // Simplemente llamamos a nuestra nueva función.
    // Usamos `with_context` para añadir información útil al error si ocurre.
    shell::launch_interactive_shell(config).with_context(|| {
        format!(
            "No se pudo iniciar la sesión para el proyecto '{}'",
            config.qualified_name
        )
    })
}

/// Ejecuta un comando definido en el `axes.toml` del proyecto.
fn handle_run(
    config: &ResolvedConfig,
    script_name: Option<String>,
    params: Vec<String>,
) -> Result<()> {
    let script_key = script_name
        .ok_or_else(|| anyhow!("Debe especificar un script para ejecutar con 'run'."))?;

    let command_def = config.commands.get(&script_key).ok_or_else(|| {
        anyhow!(
            "Script '{}' no encontrado en la configuración del proyecto.",
            script_key
        )
    })?;

    // 1. Obtener el `Runnable` de la definición del comando.
    let runnable_template = match command_def {
        ProjectCommand::Sequence(s) => Runnable::Sequence(s.clone()),
        ProjectCommand::Simple(s) => Runnable::Single(s.clone()),
        ProjectCommand::Extended(ext) => ext.run.clone(),
        ProjectCommand::Platform(pc) => {
            let os_specific_runnable = if cfg!(target_os = "windows") {
                pc.windows.as_ref()
            } else if cfg!(target_os = "linux") {
                pc.linux.as_ref()
            } else if cfg!(target_os = "macos") {
                pc.macos.as_ref()
            } else {
                None
            };

            os_specific_runnable.or(pc.default.as_ref())
                .ok_or_else(|| anyhow!("El script '{}' no tiene una implementación para el SO actual y no tiene un 'default'.", script_key))?
                .clone()
        }
    };

    // 2. Ejecutar el `Runnable`.
    let interpolator = axes::core::interpolator::Interpolator::new(config, &params);

    match runnable_template {
        Runnable::Single(command_template) => {
            let final_command = interpolator.interpolate(&command_template);
            println!("\n> {}", final_command);
            axes::system::executor::execute_command(
                &final_command,
                &config.project_root,
                &config.env,
            )
            .map_err(|e| anyhow!(e))?;
        }
        Runnable::Sequence(command_templates) => {
            println!(
                "\nEjecutando secuencia de comandos para '{}'...",
                script_key
            );
            for (i, command_template) in command_templates.iter().enumerate() {
                let final_command = interpolator.interpolate(command_template);
                println!(
                    "\n[{}/{}]> {}",
                    i + 1,
                    command_templates.len(),
                    final_command
                );

                // Si cualquier paso falla, `?` detendrá la ejecución y propagará el error.
                axes::system::executor::execute_command(
                    &final_command,
                    &config.project_root,
                    &config.env,
                )
                .map_err(|e| anyhow!(e))?;
            }
            println!("\n✔ Secuencia completada con éxito.");
        }
    }

    Ok(())
}

/// Muestra información detallada sobre la configuración resuelta del proyecto.
fn handle_info(config: &ResolvedConfig) -> Result<()> {
    let config_file_path = config
        .project_root
        .join(AXES_DIR)
        .join(PROJECT_CONFIG_FILENAME);

    println!("\n--- Información de '{}' ---", config.qualified_name);
    println!("  UUID:           {}", config.uuid);
    println!("  Ruta Raíz:    {}", config.project_root.display());
    println!("  Archivo Conf:   {}", config_file_path.display());

    if let Some(v) = &config.version {
        println!("  Versión:        {}", v);
    }
    if let Some(d) = &config.description {
        println!("  Descripción:    {}", d);
    }

    if !config.commands.is_empty() {
        println!("\n  Comandos Disponibles:");
        let mut cmd_names: Vec<_> = config.commands.keys().collect();
        cmd_names.sort();
        for cmd_name in cmd_names {
            if let Some(command_def) = config.commands.get(cmd_name) {
                // **CORRECCIÓN**: El match ahora extrae la struct interna `ext`.
                match command_def {
                    ProjectCommand::Sequence(_) => {
                        println!("    - {} (secuencia de comandos)", cmd_name)
                    }
                    ProjectCommand::Extended(ext) => {
                        if let Some(d) = &ext.desc {
                            println!("    - {} : {}", cmd_name, d);
                        } else {
                            println!("    - {}", cmd_name);
                        }
                    }
                    ProjectCommand::Simple(_) => {
                        println!("    - {}", cmd_name)
                    }
                    ProjectCommand::Extended(ext) => {
                        // Extraer `ext`
                        if let Some(d) = &ext.desc {
                            // Acceder a `ext.desc`
                            println!("    - {} : {}", cmd_name, d);
                        } else {
                            println!("    - {}", cmd_name);
                        }
                    }
                    ProjectCommand::Platform(pc) => {
                        if let Some(d) = &pc.desc {
                            println!("    - {} : {}", cmd_name, d);
                        } else {
                            println!("    - {} (multi-plataforma)", cmd_name);
                        }
                    }
                }
            }
        }
    } else {
        println!("\n  No hay comandos definidos.");
    }

    if !config.vars.is_empty() {
        println!("\n  Variables (fusionadas):");
        for (key, val) in &config.vars {
            println!("    - {} = \"{}\"", key, val);
        }
    }

    if !config.env.is_empty() {
        println!("\n  Variables de Entorno (fusionadas):");
        for (key, val) in &config.env {
            println!("    - {} = \"{}\"", key, val);
        }
    }

    println!("\n--------------------------");
    Ok(())
}

/// Abre el proyecto con una aplicación configurada.
fn handle_open(config: &ResolvedConfig, args: Vec<String>) -> Result<()> {
    // 1. Determinar la clave de la acción de apertura.
    let open_key = if !args.is_empty() && args[0] == "with" {
        // Caso: `axes ... open with vsc`
        args.get(1) // Tomar el nombre de la app
            .map(|s| s.as_str())
            .ok_or_else(|| anyhow!("El comando 'open with' requiere el nombre de una aplicación (ej: 'vsc', 'explorer')."))?
    } else if !args.is_empty() {
        // Caso: `axes ... open vsc` (atajo)
        args[0].as_str()
    } else {
        // Caso: `axes ... open` (usar el default)
        config.options.open_with.get("default")
            .ok_or_else(|| anyhow!("No se especificó una aplicación y no hay una clave 'default' en [options.open_with]."))?
            .as_str()
    };

    // 2. Buscar el comando en la configuración.
    // Si la clave es "default", el usuario cometió un error, ya que "default" debe apuntar a otra clave.
    if open_key == "default" {
        return Err(anyhow!(
            "La clave 'default' debe apuntar al nombre de otra acción de apertura (ej: default = \"vsc\")."
        ));
    }

    let command_template = config.options.open_with.get(open_key).ok_or_else(|| {
        anyhow!(
            "No se encontró una acción de apertura para '{}' en [options.open_with].",
            open_key
        )
    })?;

    // 3. Interpolar y ejecutar. Por ahora, {root} y {path} son iguales.
    let interpolator = axes::core::interpolator::Interpolator::new(config, &[]);
    let final_command = interpolator.interpolate(command_template);

    println!("\n> {}", final_command);

    axes::system::executor::execute_command(&final_command, &config.project_root, &config.env)
        .map_err(|e| anyhow!(e))
}

fn handle_rename(config: &ResolvedConfig, args: Vec<String>) -> Result<()> {
    let new_name = args
        .get(0)
        .ok_or_else(|| anyhow!("El comando 'rename' requiere un nuevo nombre para el proyecto."))?
        .trim();

    if new_name.is_empty() {
        return Err(anyhow!("El nuevo nombre no puede estar vacío."));
    }
    // Validar que el nuevo nombre no contenga caracteres de ruta ('/' o '\')
    if new_name.contains('/') || new_name.contains('\\') {
        return Err(anyhow!("El nuevo nombre no puede contener '/' o '\\'."));
    }
    // Validar que no sea un nombre reservado
    if ["global", ".", "..", "*", "_", "**"].contains(&new_name.to_lowercase().as_str()) {
        return Err(anyhow!(
            "El nombre '{}' es reservado y no puede usarse para un proyecto.",
            new_name
        ));
    }

    println!(
        "Renombrando '{}' a '{}'...",
        config.qualified_name, new_name
    );

    // 1. Cargar el índice global para modificarlo (operación crítica)
    let mut index = index_manager::load_and_ensure_global_project()?;

    // 2. Renombrar el proyecto en el índice en memoria (esto incluye la validación de hermanos)
    index_manager::rename_project(&mut index, config.uuid, new_name).with_context(|| {
        format!(
            "No se pudo renombrar el proyecto '{}' en el índice global.",
            config.qualified_name
        )
    })?;

    // 3. Guardar el índice global modificado en disco
    index_manager::save_global_index(&index)
        .context("No se pudo guardar el índice global actualizado.")?;

    // 4. Obtener y actualizar la referencia local del proyecto (project_ref.bin)
    //    Esta lógica está encapsulada en `get_or_create_project_ref` para auto-reparación.
    let mut project_ref = index_manager::get_or_create_project_ref(&config.project_root, config.uuid, &index)
        .with_context(|| format!("No se pudo obtener o crear la referencia local del proyecto `project_ref.bin` para '{}'.", config.qualified_name))?;

    // 5. Actualizar el nombre en la referencia y guardarla.
    project_ref.name = new_name.to_string();
    if let Err(e) = index_manager::write_project_ref(&config.project_root, &project_ref) {
        eprintln!(
            "\nAdvertencia: El proyecto fue renombrado en el índice global, pero no se pudo actualizar el archivo de referencia local `project_ref.bin` en `{}`: {}",
            config.project_root.display(),
            e
        );
    }

    println!("\n✔ ¡Éxito!");
    println!(
        "El proyecto '{}' ha sido renombrado a '{}'.",
        config.qualified_name, new_name
    );
    println!(
        "Nota: el nombre cualificado completo podría haber cambiado. Los cachés se regenerarán automáticamente en la próxima resolución."
    );

    Ok(())
}
