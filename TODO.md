# TODO

## Propuestas de mejora.

Si se está en modo proyecto, deshabilitar temporalmente los comandos link, advertencia para start de que se estará agregando sobre otra ejecucion de axes de forma recursiva[aceptar , cancelar].(se puede usar la variable de entorno para saberlo)

- OK - Hash de ceros predecible para el proyecto global.
- Agregar atajos con '@' tales como: `axes @micro1 tree`(de: 'proj1/dev/microservicio_1')

- Manejar de forma mas robusta la gestion de Ctrl+C para permitir el cierre forzado en el sub-shell pero no en el programa axes.

## Testing

- Comprobar que link encuentra rutas cíclicas, y proyectos con el mismo nombre registrado.


Tenemos un problema crítico de seguridad en el modo sesión! Si el usuario fuerza el cierre ej: 'Ctrl+C' la consola entra en un estado de inconsistencia absoluta y la consola deja de funcionar correctamente, así que de momento cambié main:

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