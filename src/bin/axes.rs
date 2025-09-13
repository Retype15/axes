// src/bin/axes.rs

use anyhow::Context;
use anyhow::Result;
use anyhow::anyhow;
use clap::Parser;
use std::{env, fs, path::PathBuf};
use uuid::Uuid;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use axes::cli::Cli;
use axes::models::Runnable;
use axes::system::shell;

use axes::constants::{AXES_DIR, PROJECT_CONFIG_FILENAME};
use axes::core::graph_display;
use axes::core::{
    config_resolver, context_resolver, index_manager, onboarding_manager,
    onboarding_manager::OnboardingOptions,
};
use axes::models::{Command as ProjectCommand, ProjectConfig, ProjectRef, ResolvedConfig};

use dialoguer::{Confirm, theme::ColorfulTheme};

/// El punto de entrada principal de la aplicación.
fn main() {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    // Esto se ejecuta en un hilo separado cuando se presiona Ctrl+C.
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
        println!("\nPor favor no intente cerrar forzosamente, puede cerrar de forma segura el shell usando `exit`.");
    }).expect("Error al establecer el manejador de Ctrl-C");


    // Inicializar el logger.
    env_logger::init();

    // Parsear los argumentos de la línea de comandos.
    let cli = Cli::parse();

    // Ejecutar la lógica principal y manejar cualquier error.
    if let Err(e) = run_cli(cli) {
        // No mostrar el error si fue por una interrupción del usuario.
        if running.load(Ordering::SeqCst) {
            eprintln!("\nError: {:?}", e);
            std::process::exit(1);
        } else {
            // El error fue probablemente causado por la interrupción, así que salimos silenciosamente.
            println!("\nOperación cancelada.");
            std::process::exit(130); // Código de salida estándar para Ctrl+C
        }
    }
}

/// El despachador principal de la aplicación.
fn run_cli(cli: Cli) -> Result<()> {
    log::debug!("CLI args parsed: {:?}", cli);

    // Lista de acciones de sistema conocidas.
    const SYSTEM_ACTIONS: &[&str] = &[
        "tree", "info", "open", "rename", "link", "unregister", "delete", 
        "init", "register", "run", "start", "alias" // `alias` es futuro
    ];

    // --- Detección de Modo: Sesión vs. Script ---
    if let Ok(project_uuid_str) = std::env::var("AXES_PROJECT_UUID") {
        // --- MODO SESIÓN ---
        let project_uuid = Uuid::parse_str(&project_uuid_str)
            .context("La variable de entorno AXES_PROJECT_UUID es inválida.")?;
        
        // En modo sesión, el primer argumento SIEMPRE es la acción.
        let action = cli.context_or_action
            .ok_or_else(|| anyhow!("En modo sesión, se requiere una acción (ej: `axes tree`)."))?;
        
        // Los argumentos para la acción son todo lo que sigue.
        let mut action_args = Vec::new();
        if let Some(arg) = cli.action_or_arg {
            action_args.push(arg);
        }
        action_args.extend(cli.args);

        // Resolver el contexto implícito desde la variable de entorno.
        let index = index_manager::load_and_ensure_global_project()?;
        //let entry = index.projects.get(&project_uuid)
        //    .ok_or_else(|| anyhow!("El proyecto de la sesión actual (UUID: {}) ya no está registrado.", project_uuid))?;
        
        let qualified_name = index_manager::build_qualified_name(project_uuid, &index)
            .ok_or_else(|| anyhow!("No se pudo reconstruir el nombre del proyecto de la sesión actual (UUID: {}). Posible enlace de padre roto en el índice.", project_uuid))?;
            
        log::info!("Modo Sesión: Ejecutando en el contexto implícito de '{}'", qualified_name);
        
        let config = config_resolver::resolve_config_for_uuid(project_uuid, qualified_name, &index)?;

        return handle_project_action(config, Some(action), action_args, SYSTEM_ACTIONS);

    } else {
        // --- MODO SCRIPT ---
        
        // 1. Manejar caso sin argumentos -> TUI
        let arg1 = match cli.context_or_action {
            Some(a) => a,
            None => {
                println!("TODO: Lanzar la interfaz de usuario interactiva (TUI).");
                return Ok(());
            }
        };

        let arg2 = cli.action_or_arg;
        
        // 2. Determinar orden y atajos
        let (context_str, action_str, mut remaining_args) = 
            determine_context_and_action(&arg1, arg2.as_deref(), SYSTEM_ACTIONS)?;
        
        // Añadir el resto de los args
        remaining_args.extend(cli.args);
        
        // 3. Casos especiales que no resuelven contexto
        if action_str == "init" || action_str == "register" {
            let mut special_args = vec![context_str];
            special_args.extend(remaining_args);

            return match action_str.as_str() {
                "init" => handle_init(special_args.get(0).cloned(), special_args.into_iter().skip(1).collect()),
                "register" => handle_register(special_args.get(0).cloned(), special_args.into_iter().skip(1).collect()),
                _ => unreachable!(),
            };
        }

        // 4. Resolución y ejecución para todos los demás comandos
        let index = index_manager::load_and_ensure_global_project()?;
        let (uuid, qualified_name) = context_resolver::resolve_context(&context_str, &index)?;
        let config = config_resolver::resolve_config_for_uuid(uuid, qualified_name, &index)?;
        log::info!("Proyecto '{}' resuelto con éxito.", config.qualified_name);

        return handle_project_action(config, Some(action_str), remaining_args, SYSTEM_ACTIONS);
    }
}

/// Función auxiliar para determinar el contexto y la acción en modo script.
fn determine_context_and_action<'a>(
    arg1: &'a str, 
    arg2: Option<&'a str>,
    system_actions: &[&str]
) -> Result<(String, String, Vec<String>)> {
    
    match arg2 {
        Some(arg2_val) => {
            // Caso: 2 o más argumentos
            if system_actions.contains(&arg1) {
                // Formato: `axes <acción> <contexto> [args...]`
                Ok((arg2_val.to_string(), arg1.to_string(), Vec::new()))
            } else if system_actions.contains(&arg2_val) {
                // Formato: `axes <contexto> <acción> [args...]`
                Ok((arg1.to_string(), arg2_val.to_string(), Vec::new()))
            } else {
                // Formato atajo `run`: `axes <contexto> <script> [args...]`
                Ok((arg1.to_string(), "run".to_string(), vec![arg2_val.to_string()]))
            }
        }
        None => {
            // Caso: 1 solo argumento
            if arg1 == "tree" {
                // `axes tree` -> `axes global tree`
                Ok(("global".to_string(), "tree".to_string(), Vec::new()))
            } else {
                // `axes mi-proyecto` -> `axes mi-proyecto start`
                Ok((arg1.to_string(), "start".to_string(), Vec::new()))
            }
        }
    }
}

/// Maneja las acciones que operan sobre una configuración de proyecto ya resuelta.
fn handle_project_action(
    config: ResolvedConfig,
    action_or_arg: Option<String>,
    args: Vec<String>,
    system_actions: &[&str],
) -> Result<()> {
    // La acción ya ha sido determinada por el despachador.
    // El `action_or_arg` es la acción, y `args` son sus argumentos.
    let action = action_or_arg.expect("La acción debería estar determinada en este punto.");

    log::debug!(
        "Manejando acción '{}' para el proyecto '{}'",
        action,
        config.qualified_name
    );

    match action.as_str() {
        // Comandos de sistema
        "tree" => handle_tree(&config),
        "start" => handle_start(&config),
        "info" => handle_info(&config),
        "open" => handle_open(&config, args),
        "rename" => handle_rename(&config, args),
        "link" => handle_link(&config, args),
        "unregister" => handle_unregister(&config, args),
        "delete" => handle_delete(&config, args),
        "run" => {
            let script_name = args.first().cloned();
            let params = args.into_iter().skip(1).collect();
            handle_run(&config, script_name, params)
        }
        
        // Caso de fallback: si la "acción" no es una acción de sistema,
        // podría ser un atajo para `run` que el despachador no capturó (ej. modo sesión).
        script_name if !system_actions.contains(&script_name) => {
            handle_run(&config, Some(action), args)
        }

        // Si es una acción de sistema pero no tiene un `handle`, es un error.
        unknown => {
            anyhow::bail!(
                "La acción '{}' es reconocida pero no está implementada.",
                unknown
            );
        }
    }
}

// --- MANEJADORES DE ACCIONES (Implementaciones) ---

///Permite crear y registrar nuevos proyectos a axes.
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
    let (new_uuid, _) = index_manager::add_project_to_index(&mut index, project_name.clone(), canonical_path.clone(), Some(final_parent_uuid))
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

fn handle_link(config: &ResolvedConfig, args: Vec<String>) -> Result<()> {
    // 1. Obtener el contexto del nuevo padre.
    let new_parent_context = args
        .first()
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
        .first()
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

///Registrar proyecto existente.
fn handle_unregister(config: &ResolvedConfig, args: Vec<String>) -> Result<()> {
    let unregister_children = args.iter().any(|arg| arg == "--children");
    let mut index = index_manager::load_and_ensure_global_project()?;

    let mut uuids_to_unregister = vec![config.uuid];
    if unregister_children {
        println!(
            "Recolectando todos los descendientes de '{}'...",
            config.qualified_name
        );
        uuids_to_unregister.extend(index_manager::get_all_descendants(&index, config.uuid));
    }

    println!(
        "\nSe desregistrarán las siguientes entradas de `axes` (los archivos no serán modificados):"
    );
    for uuid in &uuids_to_unregister {
        if let Some(entry) = index.projects.get(uuid) {
            println!("  - {} (en {})", entry.name, entry.path.display());
        }
    }

    if !unregister_children
        && index
            .projects
            .values()
            .any(|e| e.parent == Some(config.uuid))
    {
        println!(
            "\nNota: los hijos directos de '{}' se convertirán en hijos de 'global'.",
            config.qualified_name
        );
    }

    if !Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("¿Continuar?")
        .default(false)
        .interact()?
    {
        println!("Operación cancelada.");
        return Ok(());
    }

    let should_reparent = !unregister_children;
    let removed_count =
        index_manager::remove_from_index(&mut index, &uuids_to_unregister, should_reparent);

    index_manager::save_global_index(&index)?;

    println!("\n✔ ¡Éxito! Se desregistraron {} proyectos.", removed_count);
    Ok(())
}

/// Elimina un proyecto del índice.
fn handle_delete(config: &ResolvedConfig, args: Vec<String>) -> Result<()> {
    let delete_children = args.iter().any(|arg| arg == "--children");
    let mut index = index_manager::load_and_ensure_global_project()?;

    let mut uuids_to_process = vec![config.uuid];
    if delete_children {
        uuids_to_process.extend(index_manager::get_all_descendants(&index, config.uuid));
    }

    println!("\n**¡ADVERTENCIA: OPERACIÓN DESTRUCTIVA!**");
    println!("Se eliminarán los directorios `.axes` Y se desregistrarán los siguientes proyectos:");

    let mut paths_to_purge = Vec::new();
    for uuid in &uuids_to_process {
        if let Some(entry) = index.projects.get(uuid) {
            println!("  - {} (en {})", entry.name, entry.path.display());
            paths_to_purge.push(entry.path.join(AXES_DIR));
        }
    }

    if !Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("¿ESTÁS SEGURO?")
        .default(false)
        .interact()?
    {
        println!("Operación cancelada.");
        return Ok(());
    }

    // 1. Purgar archivos (lo hacemos primero, por si falla, no dejamos el índice inconsistente)
    let mut purged_count = 0;
    for path in paths_to_purge {
        if path.exists() {
            if fs::remove_dir_all(&path).is_ok() {
                purged_count += 1;
            } else {
                eprintln!("Advertencia: no se pudo purgar {}", path.display());
            }
        }
    }

    // 2. Desregistrar del índice (nunca re-parentamos en un delete recursivo)
    let removed_count = index_manager::remove_from_index(&mut index, &uuids_to_process, false);

    index_manager::save_global_index(&index)?;

    println!("\n✔ ¡Éxito!");
    println!(
        "Se eliminaron {} directorios `.axes` y se desregistraron {} proyectos.",
        purged_count, removed_count
    );
    Ok(())
}

/// Registra un proyecto existente en el directorio actual o en una ruta especificada.
fn handle_register(path_arg: Option<String>, args: Vec<String>) -> Result<()> {
    // 1. Determinar la ruta objetivo
    let path = match path_arg {
        Some(ref p) if p != "--autosolve" => PathBuf::from(p),
        _ => std::env::current_dir()?,
    };

    if !path.exists() {
        return Err(anyhow!(
            "La ruta especificada no existe: {}",
            path.display()
        ));
    }

    // 2. Parsear flags
    let autosolve = args.iter().any(|arg| arg == "--autosolve")
        || (path_arg.is_some() && path_arg.unwrap() == "--autosolve");

    // 3. Cargar el índice
    let mut index = index_manager::load_and_ensure_global_project()?;

    // 4. Configurar opciones y llamar a la máquina de estados
    let options = OnboardingOptions {
        autosolve,
        suggested_parent_uuid: None,
    };

    // Pasar las opciones como referencia
    onboarding_manager::register_project(&path, &mut index, &options).context(format!(
        "No se pudo registrar el proyecto en '{}'.",
        path.display()
    ))?;

    // 5. Guardar los cambios realizados en el índice
    index_manager::save_global_index(&index)?;

    println!("\nOperación de registro finalizada.");
    Ok(())
}

fn handle_tree(config: &ResolvedConfig) -> Result<()> {
    // Si el contexto es `global`, pasamos `None` para que muestre todo.
    // Si no, pasamos el UUID del proyecto.
    let start_node = if config.uuid == index_manager::GLOBAL_PROJECT_UUID {
        None
    } else {
        Some(config.uuid)
    };

    println!("\nMostrando árbol desde: '{}'", config.qualified_name);
    let index = index_manager::load_and_ensure_global_project()?;
    graph_display::display_project_tree(&index, start_node);
    Ok(())
}