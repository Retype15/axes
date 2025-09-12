// src/cli.rs

use clap::Parser;

/// axes: Un orquestador de flujos de trabajo de desarrollo holístico y jerárquico.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(disable_help_subcommand = true)]
pub struct Cli {
    /// El contexto del proyecto (ej: 'monorepo.api', '.', '*') o una acción global ('init' o 'tree').
    /// Si no se proporciona, se lanzará la TUI.
    pub context_or_action: Option<String>,

    /// La acción a realizar en el proyecto ('start', 'run') o el primer argumento para un comando.
    pub action_or_arg: Option<String>,

    /// Argumentos restantes para la acción (ej: los parámetros para un script de 'run').
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub args: Vec<String>,
}
