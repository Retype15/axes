// src/cli.rs

use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about = "Axes: Un gestor de flujo de trabajo de proyecto holístico.", long_about = None)]
#[command(disable_help_subcommand = true)]
pub struct Cli {
    /// Contexto del proyecto (nombre, '.', '_', '*') o una acción global ('list', 'help').
    pub context_or_action: Option<String>,

    /// Acción a realizar en el proyecto ('run', 'start', 'init', etc.).
    pub action: Option<String>,

    /// Argumentos restantes para la acción.
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub args: Vec<String>,
}
