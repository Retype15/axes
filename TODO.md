
# Propuestas para el futuro

- la flag `--with-entorn` para ejecutar comandos puntuales asegurando que antes del mismo se ejecute `at_start` y al finalizar `at_exit` o viceversa, que se ejecuten con at_start y at_end, a menos que se use el flag `--no-entorn` (nombres sujetos a cambiar).
- los comandos init, start no son lo suficientemente inteligentes, si se les ordena ejecutarse en la ruta `.` o '_' si encuentran un proyecto no registrado deben hacer la solicitud sobre qué hacer, si registrarlo, ejecutar de forma temporal, o cancelar.
- Herramienta `axes <proyecto|-g> checkout` para hacer un escaneo a variables de entorno, chequeo de rutas y otras cosas y reportar cada error y/o advertencias encontradas, flag: `--fix` para indicar que aplique las reparaciones a los problemas encontrados. flag: `--no-warn` para no mostar advertencias, si se usa junto a `--fix` repara solo errores, e ignora advertencias.
- Guardar los datos config en el entorno del subshell para agilizar los procesos.
- Implementar el sistema de multi-consola (multi-shell).
- Sistema de idiomas intercambiable, y definido desde config.toml y opcionalmente(como cualquier variable) desde los axes.toml
- Cambiar el nombre de axes.toml dentro de .axes por config.toml, es mas declarativo. (cancelado por propuesta de refactorizacion 1)
- Implementar de una vez el comando help (handle_help)
- Cuando se inicie un proyecto, antes de ejecutar at_start, las variables del grupo [env] deben ser incluidas en la consola, entonces va: add_env -> at_start -> shell -> at_exit.
- Guardado de estados de sesion para cada proyecto(guarda variables de entorno definidas, lista de comandos recientes usados, etc) para continuar con el flujo de trabajo cuando se prefiera.
- perfeccionar el sistema de plantillas, actualmente incompleto.
- Para options, incluir: `editor = "code"` que indica que al usar open por defecto abrirá en un proyecto de vsc, flag: `--editor <nombre|codigo del editor, ex: code, explorer, cmd>`

## Propuestas de refactorización

- Envez de que cada ruta sea un solo proyecto, permitir varios a una ruta si hace falta, por ejemplo uis.testing con env, at_start y at_exit preparado para el testing, y uis.build con los env, at_start y at_exit preparados para desarrollo, el resto de herramientas (como los scripts tambien separados para cada estado) en estos usos, la estructura sería: uis.toml para la configuracion general, uis.testing.toml y uis.build.toml para implementacionese especificas para cada etapa de desarrollo, tambien sirve para separar microservicios de esta manera.

## Error FIxes

- al usar `axes <cualquier cosa>` y `axes <cualq.cosa> start` como script NO puede crear proyecto, solamente dar error si no existe el proyecto, advertencia si existe pero tiene errores(ejemplo la ruta a la que apunta no existe o no se encuentra el proyecto, o no está registrado(si se usa '.' o '_' que son los que buscan en carpetas)(en cuyo caso lanza la advertencia para registrar, abrir temporalmente o cancelar))
