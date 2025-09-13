// src/cli.rs

use clap::Parser;

/// axes: Un orquestador de flujos de trabajo de desarrollo holístico y jerárquico.
///
/// `axes` opera en dos modos principales:
///
/// 1. MODO SCRIPT (por defecto):
///    La sintaxis es flexible. `axes` determina si un argumento es una acción o un
///    contexto de proyecto basándose en una lista de acciones de sistema conocidas.
///
///    Formatos válidos:
///    - `axes <contexto> <acción> [args...]`  (ej: `axes mi-app/api info`)
///    - `axes <acción> <contexto> [args...]`  (ej: `axes info mi-app/api`)
///
///    Atajos:
///    - `axes <contexto>` -> se expande a `axes <contexto> start`
///    - `axes <contexto> <script>` -> se expande a `axes <contexto> run <script>`
///
/// 2. MODO SESIÓN (cuando `AXES_PROJECT_UUID` está definido):
///    La sintaxis es estricta, ya que el contexto del proyecto es implícito.
///
///    Formato válido:
///    - `axes <acción> [args...]` (ej: `axes tree`)
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(disable_help_subcommand = true)]
pub struct Cli {
    /// El primer argumento posicional.
    ///
    /// Su rol depende del modo y de los otros argumentos:
    /// - En MODO SCRIPT, puede ser un contexto de proyecto, una acción de sistema,
    ///   o una acción global.
    /// - En MODO SESIÓN, SIEMPRE es una acción.
    /// - Si se omite, se intentará lanzar la TUI.
    pub context_or_action: Option<String>,

    /// El segundo argumento posicional.
    ///
    /// Su rol depende del primer argumento:
    /// - Si el primer argumento fue una ACCIÓN, este es el CONTEXTO.
    /// - Si el primer argumento fue un CONTEXTO, este puede ser una ACCIÓN o el
    ///   nombre de un SCRIPT.
    /// - Para acciones globales (`init`, `register`, `alias`), este es el primer
    ///   argumento para esa acción (ej. el nombre de un alias).
    pub action_or_context_or_arg: Option<String>,

    /// Todos los argumentos restantes.
    ///
    /// Se pasan directamente a la acción que se está ejecutando. Por ejemplo, los
    /// parámetros para un script de `run`, el nuevo nombre para `rename`, o los
    /// flags para `delete`.
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub args: Vec<String>,
}
