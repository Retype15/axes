// src/system/shell.rs

use std::env;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ShellError {
    #[error("No se pudo encontrar la shell del sistema (variable ComSpec no definida).")]
    ShellNotFound,
    #[error("Error de entrada/salida: {0}")]
    IoError(#[from] std::io::Error),
}

/// Lanza una sub-shell interactiva para un proyecto, inyectando variables de entorno de sesión.
pub fn launch_interactive_shell(
    project_root: &PathBuf,
    project_name: &str,
    at_start_script: Option<&str>,
) -> Result<(), ShellError> {
    let shell_executable = env::var("ComSpec").map_err(|_| ShellError::ShellNotFound)?;
    log::info!("Lanzando shell: {}", &shell_executable);

    let mut cmd = Command::new(&shell_executable);
    cmd.current_dir(project_root);

    // --- NUEVA LÓGICA DE INYECCIÓN DE ENTORNO ---
    // 1. Establecer nuestras variables de entorno de sesión.
    // La sub-shell heredará el entorno actual, y nosotros añadimos/sobrescribimos estas.
    cmd.env("AXES_PROJECT_ROOT", project_root.as_os_str());
    cmd.env("AXES_PROJECT_NAME", project_name);

    // 2. Construir el comando inicial para `/K`
    let mut initial_command = String::new();

    if let Some(script) = at_start_script {
        initial_command.push_str(&format!("call {}", script));
        initial_command.push_str(" && ");
    }

    let welcome_message = format!(
        "echo. && echo --- Sesion de Axes para '{}' iniciada. --- && echo Para salir, escribe 'exit'.",
        project_name
    );
    initial_command.push_str(&welcome_message);

    log::debug!("Comando de inicialización: {}", initial_command);

    cmd.arg("/K").arg(initial_command);

    // 3. Conectar I/O y esperar (sin cambios)
    let status = cmd
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;

    if !status.success() {
        log::warn!(
            "La shell interactiva terminó con un código de error: {:?}",
            status.code()
        );
    }

    Ok(())
}
