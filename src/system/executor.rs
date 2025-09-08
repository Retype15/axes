// src/system/executor.rs

use std::path::PathBuf;
use std::process::{Command as StdCommand, Stdio};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ExecutionError {
    #[error("El comando '{0}' no se pudo ejecutar: {1}")]
    CommandFailed(String, std::io::Error),
    #[error("El comando '{0}' finalizó con un código de error: {1:?}")]
    NonZeroExitStatus(String, Option<i32>),
}

/// Ejecuta un comando de sistema en un directorio de trabajo específico.
pub fn execute_command(command_line: &str, cwd: &PathBuf) -> Result<(), ExecutionError> {
    log::info!("Ejecutando comando: '{}' en {:?}", command_line, cwd);

    // Determina la shell a usar. En Windows, `cmd`.
    let (shell, arg) = if cfg!(target_os = "windows") {
        ("cmd", "/C")
    } else {
        ("sh", "-c")
    };

    let mut command = StdCommand::new(shell);
    command
        .arg(arg)
        .arg(command_line)
        .current_dir(cwd)
        .stdout(Stdio::inherit()) // Redirige stdout del subproceso a nuestro stdout
        .stderr(Stdio::inherit()); // Redirige stderr del subproceso a nuestro stderr

    let status = command
        .status()
        .map_err(|e| ExecutionError::CommandFailed(command_line.to_string(), e))?;

    if !status.success() {
        return Err(ExecutionError::NonZeroExitStatus(
            command_line.to_string(),
            status.code(),
        ));
    }

    Ok(())
}
