// src/bin/axes.rs

use anyhow::Result;
use clap::Parser;
use anyhow::anyhow;
use std::{env, fs};
use uuid::Uuid;
use anyhow::Context;

use axes::cli::Cli;
use axes::core::interpolator::Interpolator;
use axes::system::executor;
use axes::system::shell;

use axes::core::graph_display;
use axes::core::{config_resolver, context_resolver, index_manager};
use axes::models::{Command as ProjectCommand, ProjectConfig, ResolvedConfig, ProjectRef};
use axes::constants::{AXES_DIR, PROJECT_CONFIG_FILENAME};


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
        // Caso 1: No se proporcionaron argumentos.
        None => {
            println!("TODO: Lanzar la interfaz de usuario interactiva (TUI).");
            // handle_tui_launcher()
            Ok(())
        }
        // Caso 2: Se proporcionó un argumento.
        Some(context_or_action) => {
            match context_or_action.as_str() {
                // Acciones globales que no requieren un contexto de proyecto.
                "list" => handle_list(),
                "init" => handle_init(cli.action_or_arg, cli.args),

                // Cualquier otra cosa se asume que es un contexto de proyecto.
                project_context => {
                    log::info!("Resolviendo contexto de proyecto: '{}'", project_context);
                    // 1. Cargar el índice una vez.
                    let index = index_manager::load_and_ensure_global_project()?;
                    // 2. Resolver el contexto a un UUID y nombre.
                    let (uuid, qualified_name) = context_resolver::resolve_context(project_context, &index)?;
                    // 3. Resolver la configuración para ese UUID.
                    let config = config_resolver::resolve_config_for_uuid(uuid, qualified_name, &index)?;
                    log::info!("Proyecto '{}' resuelto con éxito.", config.qualified_name);
                    
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
    println!("Inicializando proyecto '{}' en {}", project_name, current_dir.display());

    // 1. Validar que no exista ya un proyecto en el directorio actual
    let axes_dir = current_dir.join(AXES_DIR);
    if axes_dir.exists() {
        return Err(anyhow!("Ya existe un directorio '.axes' en esta ubicación."));
    }
    
    // 2. Cargar índice y resolver el padre (si se especificó)
    let mut index = index_manager::load_and_ensure_global_project()?;
        let parent_uuid: Option<Uuid> = match parent_context {
        Some(context) => {
            println!("Resolviendo padre '{}'...", context);
            // La llamada aquí no cambia, porque `resolve_context` ahora maneja la nueva sintaxis.
            let (uuid, qualified_name) = context_resolver::resolve_context(&context, &index)?;
            println!("Proyecto padre '{}' encontrado (UUID: {}).", qualified_name, uuid);
            Some(uuid)
        },
        None => {
            println!("No se especificó padre. Se enlazará al proyecto 'global'.");
            Some(index_manager::GLOBAL_PROJECT_UUID)
        }
    };

    // 3. Añadir el nuevo proyecto al índice
    let canonical_path = current_dir.canonicalize()?;
    let new_uuid = index_manager::add_project_to_index(&mut index, project_name.clone(), canonical_path.clone(), parent_uuid)
        .context("No se pudo añadir el proyecto al índice.")?;
    
    // 4. Crear la estructura de archivos del proyecto
    fs::create_dir_all(&axes_dir)?;
    let config_path = axes_dir.join(PROJECT_CONFIG_FILENAME);
    let default_config = ProjectConfig::new();
    let toml_string = toml::to_string_pretty(&default_config)?;
    fs::write(&config_path, toml_string)?;

    let project_ref = ProjectRef {
        self_uuid: new_uuid,
        parent_uuid,
        name: project_name.clone(),
    };
    index_manager::write_project_ref(&canonical_path, &project_ref)
        .context("No se pudo escribir el archivo de referencia del proyecto (project_ref.bin).")?;

    println!("\n✔ ¡Éxito!");
    println!("  Proyecto '{}' creado con UUID: {}", project_name, new_uuid);
    println!("  Configuración creada en: {}", config_path.display());
    println!("  Identidad local guardada en: .axes/{}", axes::constants::PROJECT_REF_FILENAME);
    println!("  Registrado correctamente en el índice global.");

    Ok(())
}

/// Maneja las acciones que operan sobre una configuración de proyecto ya resuelta.
fn handle_project_action(
    config: ResolvedConfig,
    action_or_arg: Option<String>,
    args: Vec<String>,
) -> Result<()> {
    // Determinar la acción a realizar. Si no se especifica, el predeterminado es 'start'.
    let action = action_or_arg.as_deref().unwrap_or("start");

    log::debug!(
        "Manejando acción '{}' para el proyecto '{}'",
        action,
        config.qualified_name
    );

    match action {
        "start" => handle_start(&config),
        "run" => {
            // Para 'run', el primer argumento después de 'run' es el nombre del script.
            // Ej: `axes . run build --fast` -> `args` es ["build", "--fast"]
            let script_name = args.get(0).cloned();
            handle_run(&config, script_name, args.into_iter().skip(1).collect())
        }
        "info" => handle_info(&config),
        "open" => handle_open(&config, args),
        // ... otros comandos como edit, open, etc. irían aquí.

        // Atajo: `axes <proyecto> <script>` es equivalente a `axes <proyecto> run <script>`
        script_name if config.commands.contains_key(script_name) => {
            log::debug!("Detectado atajo para 'run'. Script: '{}'", script_name);
            // Reconstruimos los argumentos para `handle_run`.
            // `action_or_arg` era el nombre del script, y `args` son sus parámetros.
            let script_name = action_or_arg.expect("Script name should exist here");
            handle_run(&config, Some(script_name), args)
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

/// Muestra la lista de proyectos raíz.
fn handle_list() -> Result<()> {
    // El mensaje se mueve a `display_project_tree` para un mejor control
    let index = index_manager::load_and_ensure_global_project()?;
    graph_display::display_project_tree(&index);
    Ok(())
}
fn handle_link(_project_to_move_uuid: Uuid, _new_parent_uuid: Uuid) -> Result<()> { //Ignore warn...
    //let index = index_manager::load_and_ensure_global_project()?;
    
    // **VALIDACIÓN DE CICLOS**
    //if index_manager::find_cycle_from_node(project_to_move_uuid, &index) {
    //    return Err(anyhow!("Operación ilegal: crearía una referencia circular."));
    //}
    
    // **VALIDACIÓN DE COLISIÓN DE NOMBRES**
    // ... (comprobar que el nuevo padre no tenga ya un hijo con el mismo nombre)
    
    // Si todo es válido, proceder a modificar el índice...
    // ...
    
    Ok(())
}

/// Inicia una sesión de terminal interactiva para el proyecto.
fn handle_start(config: &ResolvedConfig) -> Result<()> {
    println!(
        "\nIniciando sesión para '{}'...",
        config.qualified_name
    );
    
    // Simplemente llamamos a nuestra nueva función.
    // Usamos `with_context` para añadir información útil al error si ocurre.
    shell::launch_interactive_shell(config)
        .with_context(|| format!("No se pudo iniciar la sesión para el proyecto '{}'", config.qualified_name))
}

/// Ejecuta un comando definido en el `axes.toml` del proyecto.
fn handle_run(
    config: &ResolvedConfig,
    script_name: Option<String>,
    params: Vec<String>,
) -> Result<()> {
    let script_key = script_name
        .ok_or_else(|| anyhow!("Debe especificar un script para ejecutar con 'run'."))?;
        
    let command_def = config.commands.get(&script_key)
        .ok_or_else(|| anyhow!("Script '{}' no encontrado en la configuración del proyecto.", script_key))?;

    let command_line_template = match command_def {
        ProjectCommand::Simple(s) => s.clone(),
        ProjectCommand::Extended(ext) => ext.run.clone(),
        ProjectCommand::Platform(pc) => {
            // Resolver el comando específico para la plataforma actual
            let os_specific_command = if cfg!(target_os = "windows") {
                pc.windows.as_ref()
            } else if cfg!(target_os = "linux") {
                pc.linux.as_ref()
            } else if cfg!(target_os = "macos") {
                pc.macos.as_ref()
            } else {
                None
            };
            
            // Usar el comando específico del SO, o el `default` como fallback.
            os_specific_command.or(pc.default.as_ref())
                .ok_or_else(|| anyhow!("El script '{}' no tiene una implementación para el sistema operativo actual y no tiene un 'default'.", script_key))?
                .clone()
        }
    };
    
    // Interpolar los tokens en la línea de comando
    let interpolator = Interpolator::new(config, &params);
    let final_command_line = interpolator.interpolate(&command_line_template);
    
    println!("\n> {}", final_command_line);
    
    // Ejecutar el comando final
    executor::execute_command(&final_command_line, &config.project_root, &config.env)
        .map_err(|e| anyhow!(e))
}

/// Muestra información detallada sobre la configuración resuelta del proyecto.
fn handle_info(config: &ResolvedConfig) -> Result<()> {
    let config_file_path = config.project_root.join(AXES_DIR).join(PROJECT_CONFIG_FILENAME);
    
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
                    ProjectCommand::Simple(_) => {
                        println!("    - {}", cmd_name)
                    },
                    ProjectCommand::Extended(ext) => { // Extraer `ext`
                        if let Some(d) = &ext.desc { // Acceder a `ext.desc`
                            println!("    - {} : {}", cmd_name, d);
                        } else {
                            println!("    - {}", cmd_name);
                        }
                    },
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
        return Err(anyhow!("La clave 'default' debe apuntar al nombre de otra acción de apertura (ej: default = \"vsc\")."));
    }

    let command_template = config.options.open_with.get(open_key)
        .ok_or_else(|| anyhow!("No se encontró una acción de apertura para '{}' en [options.open_with].", open_key))?;

    // 3. Interpolar y ejecutar. Por ahora, {root} y {path} son iguales.
    let interpolator = axes::core::interpolator::Interpolator::new(config, &[]);
    let final_command = interpolator.interpolate(command_template);

    println!("\n> {}", final_command);

    axes::system::executor::execute_command(&final_command, &config.project_root, &config.env)
        .map_err(|e| anyhow!(e))
}