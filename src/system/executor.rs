// src/system/executor.rs

use dunce;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Command as StdCommand, Stdio};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ExecutionError {
    #[error("El comando no pudo ser parseado: {0}")]
    CommandParse(String),
    #[error("No se especificó ningún comando para ejecutar.")]
    EmptyCommand,
    #[error("El comando '{0}' no se pudo ejecutar: {1}")]
    CommandFailed(String, std::io::Error),
    #[error("El comando '{0}' finalizó con un código de error no nulo.")]
    NonZeroExitStatus(String),
}

/// Ejecuta un comando de sistema en un directorio de trabajo específico,
/// con un conjunto de variables de entorno adicionales.
pub fn execute_command(
    command_line: &str,
    cwd: &PathBuf,
    env_vars: &HashMap<String, String>,
) -> Result<(), ExecutionError> {
    if command_line.trim().is_empty() {
        return Err(ExecutionError::EmptyCommand);
    }

    let clean_cwd = dunce::simplified(cwd);

    log::info!("Ejecutando comando: '{}' en {:?}", command_line, clean_cwd);

    // 1. Usar `shlex` para parsear la línea de comando como lo haría un shell.
    // Esto maneja correctamente las comillas y los espacios.
    let parts = shlex::split(command_line)
        .ok_or_else(|| ExecutionError::CommandParse(command_line.to_string()))?;
    
    if parts.is_empty() {
        return Err(ExecutionError::EmptyCommand);
    }

    // 2. Separar el programa de los argumentos.
    let program = &parts[0];
    let args = &parts[1..];

    // 3. Manejar el caso especial de los comandos internos de `cmd.exe` en Windows.
    let mut command;
    if cfg!(target_os = "windows") && is_windows_shell_builtin(program) {
        // Para `start`, `cd`, `echo`, etc., necesitamos envolverlos en `cmd /C`.
        command = StdCommand::new("cmd");
        command.arg("/C");
        // Pasamos el programa y sus argumentos como tokens separados,
        // lo que permite a `cmd` reconstruirlos correctamente.
        command.arg(program);
        command.args(args);
    } else {
        // Para todos los demás ejecutables (`code.exe`, `explorer.exe`, `git`, `bash`, etc.),
        // los llamamos directamente.
        command = StdCommand::new(program);
        command.args(args);
    }

    //println!("{}", clean_cwd.to_string_lossy());
    
    // 4. Configurar el resto y ejecutar.
    command
        .current_dir(clean_cwd)
        .envs(env_vars)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    let status = command
        .status()
        .map_err(|e| ExecutionError::CommandFailed(command_line.to_string(), e))?;

    if !status.success() {
        return Err(ExecutionError::NonZeroExitStatus(
            command_line.to_string(),
        ));
    }

    Ok(())
}

/// Comprueba si un comando es un "builtin" de cmd.exe.
/// Esta es una lista simplificada pero cubre los casos más comunes.
fn is_windows_shell_builtin(program: &str) -> bool {
    matches!(
        program.to_lowercase().as_str(),
        "start" | "cd" | "dir" | "echo" | "set" | "call" | "pause" | "cls" | "copy" | "del" | "move" | "rename" | "mkdir"
    )
}