// src/bin/axes.rs

use axes::cli::Cli;
use clap::Parser;
use std::path::PathBuf;
use std::{env, fs};

// --- Imports Limpios y Ordenados ---
use axes::core::{
    config::{ResolvedConfig, resolve_config},
    index::{
        ValidatedProject, load_and_validate_project_by_name, load_index, save_index,
        update_last_used,
    },
    interpolator::Interpolator,
    resolver::{find_and_load_project, find_and_load_project_from_cwd, load_project_at},
    templates,
};
use axes::models::Command as ProjectCommand;
use axes::system::{executor::execute_command, shell::launch_interactive_shell};

use dialoguer::{Input, Select, console, console::style, theme::ColorfulTheme};

type CommandResult = Result<(), String>;

fn main() {
    env_logger::init();
    let cli = Cli::parse();

    if let Err(e) = run_cli(cli)
        && !e.is_empty()
    {
        log::error!("{}", e);
        std::process::exit(1);
    }
}

fn run_cli(cli: Cli) -> CommandResult {
    // --- MODO TERMINAL (SESIÓN ACTIVA) ---
    if let Ok(project_root_str) = env::var("AXES_PROJECT_ROOT") {
        let project_root = PathBuf::from(project_root_str);

        let (project, root) = find_and_load_project(&project_root).map_err(|e| {
            format!(
                "El proyecto de la sesión activa en {:?} no se pudo cargar: {}.",
                project_root, e
            )
        })?;

        // Creamos una configuración resuelta para el modo sesión
        let config = resolve_config(project, root)?;

        let action = cli.context_or_action;
        let mut args = cli.action.map_or(Vec::new(), |a| vec![a]);
        args.extend(cli.args);

        return handle_session_command(&config, action, args);
    }

    // --- MODO GLOBAL (SIN SESIÓN) ---
    log::debug!("No se detectó ninguna sesión activa.");
    match cli.context_or_action {
        None => handle_tui_launcher(),
        Some(context) => {
            let action = cli.action;
            let args = cli.args;

            match (context.as_str(), action.as_deref()) {
                ("list", _) => handle_list(),
                ("help", _) => handle_global_action("help", args), // Ahora `args` está en el scope

                ("init", Some(_)) | (_, Some("init")) => {
                    // Manejar tanto `axes init <nombre>` como `axes <nombre> init`
                    let project_name = if context == "init" {
                        action.unwrap() // El nombre está en la posición de `action`
                    } else {
                        context // El nombre está en la posición de `context`
                    };
                    handle_init(project_name)
                }

                // --- Casos de Acciones de Proyecto (lógica original movida y simplificada) ---
                (context_str, action_opt) => {
                    match resolve_project_context(context_str) {
                        Ok(config) => {
                            update_last_used(&config.project_name).map_err(|e| e.to_string())?;
                            handle_project_action(&config, action_opt, args)
                        }
                        Err(e) => {
                            // Atajo para `init`: `axes <nombre-nuevo>`
                            if action_opt.is_none() && ![".", "_", "*"].contains(&context_str) {
                                handle_init(context)
                            } else if !e.is_empty() {
                                Err(e)
                            } else {
                                Ok(())
                            }
                        }
                    }
                }
            }
        }
    }
}

fn handle_session_command(
    config: &ResolvedConfig,
    action: Option<String>,
    args: Vec<String>,
) -> CommandResult {
    let action_str = action.as_deref().unwrap_or("info"); // Si solo se escribe `axes`, mostramos info.

    match action_str {
        // Acciones permitidas en sesión
        "run" => handle_run(config, args),
        "info" => handle_info(config),
        "edit" => handle_edit(config),
        "open" => handle_open(config),
        "list" => {
            println!("Comandos disponibles para '{}':", config.project_name);
            for cmd_name in config.commands.keys() {
                println!("  - {}", cmd_name);
            }
            Ok(())
        }

        // Acciones prohibidas o sin sentido en sesión
        "start" => Err(
            "Ya estás en una sesión. Para cambiar de proyecto, primero sal de esta (`exit`)."
                .to_string(),
        ),
        "init" | "register" | "rename" => Err(format!(
            "La acción '{}' no está permitida dentro de una sesión activa.",
            action_str
        )),

        // Atajo para `run`
        cmd_name if config.commands.contains_key(cmd_name) => {
            let mut run_args = vec![cmd_name.to_string()];
            run_args.extend(args);
            handle_run(config, run_args)
        }

        _ => Err(format!("Acción desconocida en sesión: '{}'.", action_str)),
    }
}

// --- DISPATCHERS ---

fn handle_global_action(action: &str, _args: Vec<String>) -> CommandResult {
    match action {
        //"list" => handle_list(),
        "help" => {
            println!("Mostrando ayuda... (TODO: implementar `clap` con `print_help`)");
            Ok(())
        }
        _ => Err(format!("Acción global desconocida: {}", action)),
    }
}

fn handle_project_action(
    config: &ResolvedConfig,
    action: Option<&str>,
    args: Vec<String>,
) -> CommandResult {
    let final_action = action.unwrap_or("start");

    match final_action {
        "start" => handle_start(config),
        "run" => handle_run(config, args),
        "info" => handle_info(config),
        "edit" => handle_edit(config),
        "open" => handle_open(config),
        "register" => handle_register(config),
        "rename" => handle_rename(config, args),
        cmd_name if config.commands.contains_key(cmd_name) => {
            let mut run_args = vec![cmd_name.to_string()];
            run_args.extend(args);
            handle_run(config, run_args)
        }
        _ => Err(format!(
            "Acción desconocida '{}' para el proyecto '{}'.",
            final_action, config.project_name
        )),
    }
}

fn resolve_project_context(context: &str) -> Result<ResolvedConfig, String> {
    match context {
        "." => find_and_load_project_from_cwd()
            .map_err(|e| e.to_string())
            .and_then(|(p, r)| resolve_config(p, r)),

        "_" => {
            let cwd = env::current_dir().map_err(|e| e.to_string())?;
            load_project_at(&cwd)
                .map_err(|_| {
                    "No se encontró un proyecto de Axes directamente en este directorio."
                        .to_string()
                })
                .and_then(|(p, r)| resolve_config(p, r))
        }

        "*" => {
            let index = load_index().map_err(|e| e.to_string())?;
            let last_used_name = index
                .last_used
                .as_ref()
                .ok_or("No hay ningún proyecto usado recientemente.")?;
            load_and_validate_project_by_name(last_used_name).and_then(|vp| match vp {
                ValidatedProject::Success(p, r) => resolve_config(*p, r),
                ValidatedProject::Cancelled(msg) => {
                    println!("{}", msg);
                    std::process::exit(0);
                }
            })
        }

        project_name => load_and_validate_project_by_name(project_name).and_then(|vp| match vp {
            ValidatedProject::Success(p, r) => resolve_config(*p, r),
            ValidatedProject::Cancelled(msg) => {
                println!("{}", msg);
                std::process::exit(0);
            }
        }),
    }
}

// --- MANEJADORES DE ACCIONES ---

fn handle_list() -> CommandResult {
    println!("Proyectos registrados en Axes:");
    let index = load_index().map_err(|e| e.to_string())?;

    if index.projects.is_empty() {
        println!("  (No hay proyectos registrados. Usa `axes <nombre> init` para añadir uno.)");
        return Ok(());
    }

    let max_len = index.projects.keys().map(|k| k.len()).max().unwrap_or(0);

    for (name, path) in &index.projects {
        let is_last_used = index.last_used.as_deref() == Some(name);
        let marker = if is_last_used { "*" } else { " " };
        println!(
            " {} {:<width$} -> {}",
            marker,
            name,
            path.display(),
            width = max_len
        );
    }
    Ok(())
}

fn handle_init(project_name: String) -> CommandResult {
    let current_dir = env::current_dir().map_err(|e| e.to_string())?;
    log::info!(
        "Intentando inicializar el proyecto '{}' en {:?}",
        project_name,
        current_dir
    );

    // 1. Validar que no haya ya un proyecto en el CWD
    if current_dir.join(".axes").join("axes.toml").is_file() {
        return Err("Ya existe un proyecto de Axes en este directorio.".to_string());
    }

    // 2. Validar que el nombre no esté ya en uso en el índice
    let mut index = load_index().map_err(|e| e.to_string())?;
    if index.projects.contains_key(&project_name) {
        return Err(format!(
            "El nombre de proyecto '{}' ya está registrado. Por favor, elige otro.",
            project_name
        ));
    }

    println!("Inicializando proyecto '{}'...", project_name);

    // 3. Aplicar la plantilla por defecto al directorio actual
    templates::apply_template(&current_dir, "default", &project_name)?;

    // 4. Registrar el nuevo proyecto
    let project_root = current_dir.canonicalize().map_err(|e| e.to_string())?;
    index.projects.insert(project_name.clone(), project_root);
    index.last_used = Some(project_name.clone());

    save_index(&index).map_err(|e| e.to_string())?;

    let config_path = current_dir.join(".axes").join("axes.toml");
    println!(
        "\n{}",
        style(format!(
            "✔ ¡Proyecto '{}' inicializado y registrado con éxito!",
            project_name
        ))
        .green()
    );
    println!(
        "  Puedes empezar editando: {}",
        style(config_path.display()).cyan()
    );

    Ok(())
}

fn handle_start(config: &ResolvedConfig) -> CommandResult {
    let at_start_script = config.options.get("at_start").map(|s| s.as_str());

    let shell_result =
        launch_interactive_shell(&config.project_root, &config.project_name, at_start_script);

    if let Some(at_exit_script) = config.options.get("at_exit") {
        println!("\nEjecutando script de salida (at_exit)...");
        let interpolator = Interpolator::new(config, &[]);
        let interpolated_script = interpolator.interpolate(at_exit_script);
        if let Err(e) = execute_command(&interpolated_script, &config.project_root) {
            log::error!("El script at_exit falló: {}", e);
        }
    }

    shell_result.map_err(|e| e.to_string())
}

fn handle_run(config: &ResolvedConfig, mut args: Vec<String>) -> CommandResult {
    if args.is_empty() {
        return Err("Debes especificar un comando de script para ejecutar.\n  Ejemplo: axes . run mi-comando".to_string());
    }
    let command_name = args.remove(0);
    let params = args;
    execute_project_command(config, &command_name, &params)
}

fn handle_info(config: &ResolvedConfig) -> CommandResult {
    println!(
        "\n--- Información de '{}' ---",
        style(&config.project_name).bold().cyan()
    );

    // Metadatos
    println!("  Ruta: {}", config.project_root.display());
    if let Some(v) = &config.version {
        println!("  Versión: {}", v);
    }
    if let Some(d) = &config.description {
        println!("  Descripción: {}", d);
    }

    // Comandos
    if !config.commands.is_empty() {
        println!("\n  Comandos Disponibles:");
        let mut cmd_names: Vec<_> = config.commands.keys().collect();
        cmd_names.sort();
        for cmd_name in cmd_names {
            if let Some(command_def) = config.commands.get(cmd_name) {
                match command_def {
                    ProjectCommand::Extended { desc: Some(d), .. } => {
                        println!("    - {} : {}", style(cmd_name).bold(), d);
                    }
                    _ => {
                        println!("    - {}", style(cmd_name).bold());
                    }
                }
            }
        }
    } else {
        println!("\n  No hay comandos definidos.");
    }

    // Opciones
    if !config.options.is_empty() {
        println!("\n  Opciones Configuradas:");
        for (key, val) in &config.options {
            println!("    - {}: {}", style(key).bold(), val);
        }
    }

    println!("\n--------------------------");
    Ok(())
}

fn handle_edit(config: &ResolvedConfig) -> CommandResult {
    let config_path = config.project_root.join(".axes").join("axes.toml");
    println!(
        "Abriendo archivo de configuración: {}",
        config_path.display()
    );

    opener::open(&config_path)
        .map_err(|e| format!("No se pudo abrir el archivo de configuración: {}", e))?;

    Ok(())
}

fn handle_open(config: &ResolvedConfig) -> CommandResult {
    println!(
        "Abriendo directorio del proyecto: {}",
        config.project_root.display()
    );

    opener::open(&config.project_root)
        .map_err(|e| format!("No se pudo abrir el directorio del proyecto: {}", e))?;

    Ok(())
}

fn handle_register(config: &ResolvedConfig) -> CommandResult {
    let mut index = load_index().map_err(|e| e.to_string())?;

    if index.projects.contains_key(&config.project_name) {
        return Err(format!(
            "El proyecto '{}' ya está registrado.",
            config.project_name
        ));
    }

    index
        .projects
        .insert(config.project_name.clone(), config.project_root.clone());
    save_index(&index).map_err(|e| e.to_string())?;

    println!(
        "{}",
        style(format!(
            "✔ Proyecto '{}' registrado con éxito.",
            config.project_name
        ))
        .green()
    );
    Ok(())
}

fn handle_rename(config: &ResolvedConfig, args: Vec<String>) -> CommandResult {
    let new_name = args.first()
        .ok_or("Debes proporcionar un nuevo nombre para el proyecto.\n  Ejemplo: axes . rename <nuevo-nombre>")?;

    if new_name == &config.project_name {
        return Err("El nuevo nombre es igual al actual.".to_string());
    }

    let mut index = load_index().map_err(|e| e.to_string())?;

    if index.projects.contains_key(new_name) {
        return Err(format!(
            "El nombre '{}' ya está en uso por otro proyecto.",
            new_name
        ));
    }

    if let Some(path) = index.projects.remove(&config.project_name) {
        index.projects.insert(new_name.clone(), path);

        if index.last_used.as_deref() == Some(&config.project_name) {
            index.last_used = Some(new_name.clone());
        }

        save_index(&index).map_err(|e| e.to_string())?;
        println!(
            "{}",
            style(format!(
                "✔ Proyecto '{}' renombrado a '{}' con éxito.",
                config.project_name, new_name
            ))
            .green()
        );

        let config_path = config.project_root.join(".axes").join("axes.toml");
        let mut project_to_save = config.original_project.clone();
        project_to_save.name = new_name.clone();

        let toml_content = toml::to_string_pretty(&project_to_save)
            .map_err(|e| format!("No se pudo serializar el axes.toml: {}", e))?;

        fs::write(config_path, toml_content)
            .map_err(|e| format!("No se pudo actualizar el archivo axes.toml: {}", e))?;
    } else {
        return Err(
            "Error inesperado: no se encontró el proyecto en el índice para renombrarlo."
                .to_string(),
        );
    }

    Ok(())
}

fn handle_tui_launcher() -> CommandResult {
    let mut index = load_index().map_err(|e| e.to_string())?;
    loop {
        if index.projects.is_empty() {
            println!("{}", style("No hay proyectos registrados.").yellow());
            println!(
                "Usa {} para empezar.",
                style("axes <nombre-proyecto> init").cyan()
            );

            let create_new = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("¿Deseas inicializar un nuevo proyecto ahora?")
                .item("Sí")
                .item("No")
                .default(0)
                .interact_opt()
                .map_err(|e| e.to_string())?;

            match create_new {
                Some(0) => {
                    let project_name = Input::<String>::with_theme(&ColorfulTheme::default())
                        .with_prompt("Nombre del nuevo proyecto")
                        .interact_text()
                        .map_err(|e| e.to_string())?;

                    if project_name.is_empty() {
                        eprintln!(
                            "{}",
                            style("El nombre del proyecto no puede estar vacío. Volviendo...")
                                .red()
                        );
                        continue;
                    }
                    if let Err(e) = handle_init(project_name) {
                        eprintln!(
                            "{}",
                            style(format!("Error al inicializar el proyecto: {}", e)).red()
                        );
                    }
                    index = load_index().map_err(|e| e.to_string())?;
                    continue;
                }
                _ => {
                    println!("Saliendo de Axes. ¡Hasta luego!");
                    return Ok(());
                }
            }
        }

        let mut project_names: Vec<&String> = index.projects.keys().collect();
        project_names.sort();

        let last_used_pos = index
            .last_used
            .as_ref()
            .and_then(|name| project_names.iter().position(|&n| n == name))
            .unwrap_or(0);

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Selecciona un proyecto (Esc o q para salir)")
            .items(&project_names)
            .default(last_used_pos)
            .interact_opt()
            .map_err(|e| e.to_string())?;

        let project_name = match selection {
            Some(index) => project_names[index].clone(),
            None => {
                println!("Saliendo de Axes. ¡Hasta luego!");
                return Ok(());
            }
        };

        let config = match resolve_project_context(&project_name) {
            Ok(cfg) => cfg,
            Err(e) => {
                println!("{}", e);
                index = load_index().map_err(|e| e.to_string())?;
                continue;
            }
        };

        update_last_used(&config.project_name).map_err(|e| e.to_string())?;

        let mut last_selected_action = 0;

        'action_loop: loop {
            let actions = &[
                "▶️  Start Session",
                "🚀 Run Command",
                "ℹ️  Show Info",
                "📝 Edit Config",
                "📂 Open Folder",
                "↩️  Volver a la lista de proyectos",
            ];

            let action_selection = Select::with_theme(&ColorfulTheme::default())
                .with_prompt(format!(
                    "Acción para '{}'",
                    style(&config.project_name).cyan()
                ))
                .items(actions)
                .default(last_selected_action)
                .interact_opt()
                .map_err(|e| e.to_string())?;

            let selected_action_index = match action_selection {
                Some(index) => index,
                None => break 'action_loop,
            };

            last_selected_action = selected_action_index;
            let mut needs_pause = true;

            let action_result = match selected_action_index {
                0 => {
                    needs_pause = false;
                    handle_start(&config)
                }
                1 => {
                    if config.commands.is_empty() {
                        Err(
                            "El proyecto no tiene ningún comando definido para ejecutar."
                                .to_string(),
                        )
                    } else {
                        let script_to_run: String = Input::with_theme(&ColorfulTheme::default())
                            .with_prompt("Comando a ejecutar (ej: build --fast)")
                            .interact_text()
                            .map_err(|e| e.to_string())?;

                        let args: Vec<String> =
                            script_to_run.split_whitespace().map(String::from).collect();

                        if args.is_empty() {
                            Err("No se especificó ningún script.".to_string())
                        } else {
                            handle_run(&config, args)
                        }
                    }
                }
                2 => handle_info(&config),
                3 => handle_edit(&config),
                4 => handle_open(&config),
                5 => break 'action_loop,
                _ => unreachable!(),
            };

            if let Err(e) = action_result {
                eprintln!("\n{}", style(format!("✖ {}", e)).red());
            }

            if needs_pause {
                let _ = Input::<String>::with_theme(&ColorfulTheme::default())
                    .with_prompt("\n--- Presiona Enter para continuar ---")
                    .allow_empty(true)
                    .interact();

                let _ = console::Term::stdout().clear_screen();
            }
        }
    }
}

// --- FUNCIONES AUXILIARES ---

fn execute_project_command(
    config: &ResolvedConfig,
    command_name: &str,
    params: &[String],
) -> CommandResult {
    log::debug!(
        "Ejecutando comando '{}' en proyecto '{}' con params: {:?}",
        command_name,
        config.project_name,
        params
    );

    let command_def = config
        .commands
        .get(command_name)
        .ok_or_else(|| format!("Comando '{}' no encontrado en axes.toml", command_name))?;

    let raw_command_line = match command_def {
        ProjectCommand::Simple(s) => s.clone(),
        ProjectCommand::Extended { run, .. } => run.clone(),
    };

    let interpolator = Interpolator::new(config, params);
    let mut final_command_line = interpolator.interpolate(&raw_command_line);

    if !raw_command_line.contains("{params}") && !params.is_empty() {
        final_command_line.push(' ');
        final_command_line.push_str(&params.join(" "));
    }

    log::debug!("Línea de comando final: '{}'", final_command_line);

    execute_command(&final_command_line, &config.project_root).map_err(|e| e.to_string())
}
