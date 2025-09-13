# axes: El Orquestador de Flujos de Trabajo

**Tu GPS para proyectos complejos. El control remoto universal para tus herramientas de desarrollo.**

[![CI/CD Status](https://img.shields.io/badge/build-passing-brightgreen)](#)
[![Version](https://img.shields.io/badge/version-v0.1.3--alpha-blue)](#)
[![License](https://img.shields.io/badge/license-MIT-lightgrey)](#)

`axes` es una herramienta de línea de comandos diseñada para eliminar la fricción y la carga cognitiva de trabajar en entornos de desarrollo modernos. ¿Cansado de `cd ../../..`, de recordar largos comandos de `docker`, o de mantener múltiples scripts de configuración para diferentes proyectos? `axes` resuelve esto proporcionando una capa de abstracción unificada y potente sobre todo tu ecosistema de desarrollo.

---

## Índice

- [Filosofía Principal](#filosofía-principal)
- [Inicio Rápido (En 5 Minutos)](#inicio-rápido-en-5-minutos)
- [Características Principales](#características-principales)
  - [Navegación y Gestión de Proyectos](#navegación-y-gestión-de-proyectos)
  - [Ejecución de Comandos y Flujos de Trabajo](#ejecución-de-comandos-y-flujos-de-trabajo)
  - [Sesiones de Proyecto (`start`)](#sesiones-de-proyecto-start)
  - [Configuración: El `axes.toml`](#configuración-el-axestoml)
- [Referencia Completa de Comandos](#referencia-completa-de-comandos)
- [Contribuciones](#contribuciones)
- [Licencia](#licencia)

---

## Filosofía Principal

`axes` se construye sobre tres pilares fundamentales que lo diferencian de los simples gestores de scripts.

1. **El Árbol de Proyectos:** `axes` no ve tus proyectos como una lista plana, sino como un **árbol jerárquico**. Todos los proyectos son, en última instancia, descendientes del proyecto raíz `global`. Esto te permite modelar monorepos, microservicios y grupos de proyectos relacionados de una forma natural e intuitiva.

2. **Herencia de Configuración:** La característica central. Un proyecto hijo **hereda automáticamente** toda la configuración (variables, comandos, opciones) de su padre, su abuelo, y así hasta `global`. Esto fomenta la reutilización (DRY) y permite definir configuraciones comunes en un nivel superior, mientras que los proyectos hijos pueden especializarlas o sobreescribirlas.

3. **Conciencia de Contexto:** `axes` siempre sabe "dónde estás" en el árbol de proyectos. Su sintaxis inteligente de navegación te permite moverte por la jerarquía de proyectos con la misma facilidad con la que te mueves por un sistema de archivos, eliminando la necesidad de recordar rutas físicas.

## Inicio Rápido (En 5 Minutos)

Vamos a crear y gestionar un pequeño monorepo para ver el poder de `axes` en acción.

### 1. Instalación

`axes` es una aplicación de terminal única y portable. La forma más sencilla de instalarla es descargar la última versión compilada para tu sistema operativo desde la página de **Releases** de GitHub.

#### Pasos Recomendados

1. **Descarga el Binario:**
    - Ve a la página de [**Releases de `axes` en GitHub**](https://github.com/Retype15/axes/releases).
    - Busca la última versión (`v0.1.3-alpha` o superior).
    - Descarga el archivo apropiado para tu sistema:
        - Para **Windows**: `axes-x86_64-pc-windows-msvc.zip`
        - Para **Linux**: `axes-x86_64-unknown-linux-gnu.tar.gz` *(no disponible aún)*
        - Para **macOS**: `axes-x86_64-apple-darwin.zip` *(no disponible aún)*
    **Nota:** *No disponible* significa que aún no ha sido compilado para dicho SO, si quiere usarlo en su sistema deberá compilarlo desde el codigo fuente.

2. **Descomprime el Archivo:**
    - Extrae el contenido del archivo `.zip` o `.tar.gz`. Dentro encontrarás un único archivo ejecutable: `axes.exe` (en Windows) o `axes` (en Linux/macOS).

3. **Añádelo a tu PATH (¡Importante!):**
    - Para poder ejecutar `axes` desde cualquier lugar de tu terminal, debes mover el archivo ejecutable a un directorio que esté incluido en la variable de entorno `PATH` de tu sistema.
    - **Windows:** Un buen lugar es una carpeta como `C:\Program Files\axes\` o `C:\scripts\`. Luego, busca "Editar las variables de entorno del sistema" en el menú de inicio y añade esa carpeta a tu `PATH`.
    - **Linux/macOS:** Un lugar común es `/usr/local/bin/`. Puedes moverlo con el comando:

        ```sh
        sudo mv ./axes /usr/local/bin/axes
        ```

4. **Verifica la Instalación:**
    - Abre una **nueva** terminal (importante para que se cargue el `PATH` actualizado) y ejecuta:

        ```sh
        axes --version
        ```

    - Si ves la versión de `axes`, ¡la instalación ha sido un éxito!

#### Compilación desde Fuente (Para Desarrolladores)

Si tienes [Rust](https://www.rust-lang.org/tools/install) instalado, también puedes compilar `axes` desde el código fuente:

```sh
git clone https://github.com/Retype15/axes.git
cd axes
cargo build --release # O si ya tiene el proyecto en su ordenador, puede simplemente compilar con `axes build .` (puede revisar el .axes/axes.toml del propio proyecto, no tiene mucho, pero puede usarlo para testear.)
# El ejecutable estará en ./target/release/axes
```

### 2. Creando tu Primer Proyecto

Navega a la carpeta donde guardas tus proyectos y crea una nueva aplicación.

```sh
mkdir mi-super-app && cd mi-super-app
axes init mi-super-app
```

`axes` creará un directorio `.axes/` con un `axes.toml` básico y lo registrará como un hijo directo del proyecto `global`.

### 3. Creando un Sub-Proyecto (API)

Ahora, creemos un servicio de API dentro de nuestra aplicación.

```sh
mkdir services && cd services
mkdir api && cd api
axes init api --parent mi-super-app
```

`axes` ha creado un nuevo proyecto `api` y lo ha enlazado automáticamente como hijo de `mi-super-app`.

### 4. Visualizando la Estructura

Puedes ver tu nuevo árbol de proyectos en cualquier momento.

```sh
axes global tree
```

Verás una salida similar a esta:

```sh
Árbol de Proyectos Registrados:

global [/home/user/.config/axes] (**)
└─ mi-super-app [/home/user/dev/mi-super-app]
   └─ api [/home/user/dev/mi-super-app/services/api]
```

### 5. Definiendo y Ejecutando un Comando

Abre el archivo `.axes/axes.toml` dentro del proyecto `api` y añade un comando:

```toml
# ./services/api/.axes/axes.toml
version = "0.1.0"
description = "El servicio de API principal."

[commands]
dev = "cargo watch -x run"
check = ["cargo check", "cargo clippy -- -D warnings", "echo 'API verificada!'"]
```

Ahora, desde cualquier lugar de tu sistema, puedes ejecutar estos comandos usando el **contexto** del proyecto:

```sh
# Ejecutar el comando 'check'
axes mi-super-app/api run check

# O usar el atajo, que es mucho más cómodo:
axes mi-super-app/api check
```

`axes` ejecutará los tres comandos de la secuencia `check` en orden.

### 6. Iniciando una Sesión de Proyecto

La característica más potente es `start`. Te sumerge en un entorno de shell pre-configurado para ese proyecto.

```sh
# Atajo para 'start'
axes mi-super-app/api
```

Tu terminal ahora estará "dentro" del proyecto `api`. Cualquier comando que ejecutes se ejecutará desde la raíz de la API, y `axes` estará disponible en un modo de **contexto implícito**:

```sh
--- Sesión de axes para 'mi-super-app/api' iniciada. Escribe 'exit' para salir. ---

# No necesitas especificar el contexto, ¡axes ya sabe dónde estás!
axes check

# Para salir de la sesión:
exit
```

## Ejemplos de Uso Práctico

La mejor forma de entender el poder de `axes` es verlo en acción. Hemos preparado una serie de proyectos de ejemplo en el directorio [`/examples`](https://github.com/Retype15/axes/tree/main/examples) del repositorio para demostrar cómo `axes` puede orquestar diferentes tipos de flujos de trabajo.

### Ejemplo 1: API Web con Python y Flask (`python-web-api`)

Este ejemplo demuestra un caso de uso clásico para desarrolladores de Python. Muestra cómo `axes` puede gestionar:

- **Entornos Virtuales:** El hook `at_start` activa automáticamente el `.venv` del proyecto cada vez que inicias una sesión con `axes <proyecto> start`, eliminando la necesidad de recordar ejecutar `source .venv/bin/activate`.
- **Configuración Inicial:** Un comando `setup` de un solo paso (`axes ... setup`) crea el entorno virtual e instala todas las dependencias de `requirements.txt`.
- **Variables de Entorno:** La configuración de Flask (`FLASK_APP`, `FLASK_ENV`) se define en la sección `[env]`, asegurando que el servidor de desarrollo siempre se inicie con los parámetros correctos.
- **Flujos de Trabajo Complejos:** Un comando `check` encadena otros dos comandos (`lint` y `test`) para ejecutar una suite de calidad completa con una sola instrucción.

**`axes.toml` destacado:**

```toml
# .../.axes/axes.toml

[options]
# Activa el entorno virtual al iniciar una sesión
at_start = "source ./.venv/bin/activate"

[env]
# Configura Flask para el desarrollo
FLASK_APP = "app.py"
FLASK_ENV = "development"

[commands]
# Secuencia para la configuración inicial
setup = [
    "python3 -m venv .venv",
    ".venv/bin/pip install -r requirements.txt"
]

# Inicia el servidor
dev = "flask run"

# Encadena otros comandos de `axes`
check = [
    "axes run lint",
    "axes run test"
]
```

> 👉 **Explora el [código completo del ejemplo `python-web-api`](https://github.com/Retype15/axes/tree/main/examples/python-web-api) para ver todos los detalles.**

. *(A medida que se añadan más ejemplos, se listarán aquí. Por ejemplo: Monorepo con Node.js, Proyecto de Rust, Infraestructura con Docker-Compose, etc.)*

¡Felicidades! Has experimentado el flujo de trabajo básico de `axes`. Ahora exploremos todas sus características en detalle.

## Características Principales

### Navegación y Gestión de Proyectos

`axes` proporciona una sintaxis de navegación inspirada en el sistema de archivos para moverse por el árbol de proyectos.

| Contexto       | Descripción                                                                                               | Ejemplo                               |
| :------------- | :-------------------------------------------------------------------------------------------------------- | :------------------------------------ |
| `nombre`       | Resuelve a un hijo directo del proyecto `global`.                                                         | `axes mi-super-app info`              |
| `/`            | El separador de niveles en la jerarquía.                                                                  | `axes mi-super-app/api info`          |
| `.`            | Resuelve al proyecto del directorio actual, buscando hacia arriba en el sistema de archivos si es necesario. | `cd /ruta/a/api/src && axes . tree`     |
| `_`            | Resuelve al proyecto solo si el directorio actual es **exactamente** la raíz de ese proyecto.              | `cd /ruta/a/api && axes _ tree`         |
| `..`           | Navega al padre del proyecto actual en la jerarquía.                                                      | `axes mi-super-app/api/.. tree`       |
| `**`           | (Doble asterisco) Resuelve al último proyecto que hayas usado en **todo el sistema**. Útil para volver rápido. | `axes ** start`                         |
| `*`            | (Asterisco simple) Resuelve al último hijo que hayas usado **del proyecto padre actual**.                  | `axes mi-super-app/* start`           |
| `alias!`       | Expande un alias definido por el usuario a su ruta de proyecto completa.                                  | `axes api! check`                       |

#### Alias (`!`)

Los alias son atajos personalizados para contextos largos. Se gestionan con el comando `alias`.

- `g!`: Un alias por defecto que siempre apunta al proyecto `global`.
- **Crear un alias:** `axes alias set api mi-super-app/api`
- **Usar un alias:** `axes api! info`
- **Componer alias:** `axes mi-app!/api info` (si `mi-app!` es un alias)

### Ejecución de Comandos y Flujos de Trabajo

#### El Comando `run`

El comando `run` es el corazón de la ejecución de tareas.

- **Comando Simple:** `mi-comando = "echo Hola"`
- **Comando Extendido (con descripción):** `mi-comando = { run = "echo Hola", desc = "Saluda al mundo" }`
- **Secuencia de Comandos:** Define `run` como una lista de strings. `axes` los ejecutará en orden y se detendrá si alguno falla.

    ```toml
    build-and-test = { desc = "Construye y prueba", run = [
        "cargo build",
        "cargo test"
    ]}
    ```

- **Comandos Multiplataforma:** Define diferentes comandos para cada sistema operativo.

    ```toml
    [commands.open-docs.platform]
    desc = "Abre la documentación en el navegador."
    windows = "start http://localhost:3000"
    linux = "xdg-open http://localhost:3000"
    macos = "open http://localhost:3000"
    ```

#### Ignorar Errores (`-`)

Si un comando debe ejecutarse pero su código de error no debe detener la ejecución (típico de aplicaciones gráficas), puedes prefijarlo con un guion (`-`).

```toml
[options.open_with]
# explorer.exe a menudo devuelve un código de error 1. Lo ignoramos.
explorer = "-explorer ."
```

### Sesiones de Proyecto (`start`)

El comando `start` (o su atajo `axes <contexto>`) te sumerge en una sub-shell configurada para tu proyecto.

- **Configuración Silenciosa:** Antes de que obtengas el control, `axes` ejecuta en segundo plano:
    1. La inyección de todas las variables definidas en `[env]`.
    2. La ejecución del script definido en `[options].at_start`.
- **Limpieza Automática (`at_exit`):** Cuando sales de la sesión con `exit`, `axes` ejecuta el script definido en `[options].at_exit`, ideal para detener servicios o limpiar recursos.
- **Variables de Entorno de Sesión:** Dentro de la sesión, las siguientes variables están disponibles:
  - `AXES_PROJECT_UUID`: El UUID inmutable del proyecto.
  - `AXES_PROJECT_NAME`: El nombre cualificado completo (ej. `global/mi-app/api`).
  - `AXES_PROJECT_ROOT`: La ruta física a la raíz del proyecto.
- **Contexto Implícito:** Dentro de la sesión, no necesitas especificar el contexto. `axes tree` funciona directamente y se refiere al proyecto actual. El manejo de `Ctrl+C` es seguro y no dejará tu terminal en un estado inconsistente.

### Configuración: El `axes.toml`

Este es el cerebro de cada proyecto. Todos los campos son opcionales.

```toml
# --- Metadatos (Opcional) ---
name = "my-project" # Este nombre es solo un nombre local, no se actualiza con el nombre registrado, útil para su uso como variable para los scripts!
version = "1.0.0"
description = "Una descripción de mi proyecto."

# --- Comandos Personalizados ---
[commands]
test = "cargo test -- --nocapture"
lint = { run = "cargo clippy {clippy_args}", desc = "Ejecuta el linter usando la variable guardada 'clippy_args'" }
deploy = [
    "cargo build --release",
    "./deploy-script.sh"
]

# --- Variables de Interpolación ---
[vars]
# Se pueden usar en `commands`, `options`, e incluso en otras variables.
target_dir = "build/output"
final_artifact = "{target_dir}/app.exe"
clippy_args = "--all-targets"

# --- Variables de Entorno --- 
# Se inyectan en CUALQUIER comando ejecutado por `axes` (`run` o `start`).
[env]
DATABASE_URL = "postgres://user:pass@localhost/db"
RUST_LOG = "info"

# Nota --- Recomendamos definir las variables de entorno en el proyecto principal o superior y se hereden.

# Nota 2 --- A futuro se implementará [vars.private], [env.private], etc que definirán si otros heredan o no qué propiedades. 

# --- Opciones de Comportamiento y Hooks ---
[options]
# Se ejecuta al inicio de una sesión `start`.
at_start = "source ./.venv/bin/activate"
# Se ejecuta al cerrar una sesión `start`.
at_exit = "docker-compose down"
# Define la shell a usar para `start`.
shell = "bash"

# Define los comandos para `axes <contexto> open`
[options.open_with]
# `default` es un puntero a otra clave.
default = "vsc"
explorer = "-explorer '{path}'"
vsc = "code '{path}'"
shell = "cd {path}"
```

#### Interpolación de Tokens (`{...}`)

Puedes usar tokens en casi cualquier valor de string en tu `axes.toml`.

- **Tokens Reservados:**
  - `{uuid}`: El UUID del proyecto.
  - `{name}`: El nombre cualificado completo del proyecto (ej. `global/mi-app`).
  - `{path}`: La ruta física del proyecto **actual** en el que se ejecuta el comando.
  - `{root}`: La ruta física del proyecto **donde el comando fue originalmente definido**. Esto es útil para scripts heredados que necesitan acceder a recursos de su proyecto de origen. *(aún no implementado, pendiente a implementar pronto)*
  - `{version}`: La versión del proyecto.
- **Tokens de Usuario:** Cualquier clave definida en `[vars]`.
- **Parámetros de `run`:**
  - `{params}`: Se reemplaza por todos los argumentos pasados a `run`.
  - Si no se usa `{params}`, los argumentos se añaden al final del comando.

## Referencia Completa de Comandos

| Comando                                           | Descripción                                                                                                |
| :------------------------------------------------ | :--------------------------------------------------------------------------------------------------------- |
| `axes <contexto> [acción] [args...]`              | El formato principal de uso. La sintaxis de acción/contexto es flexible.                                   |
| `axes init [padre] [nombre] [--flags]`            | Crea y registra un nuevo proyecto. Si se llama sin `nombre`, inicia un asistente interactivo.              |
| `axes register [ruta] [--autosolve]`              | Registra un proyecto existente. Inicia un asistente interactivo para resolver conflictos.                    |
| `axes <contexto> tree`                            | Muestra el sub-árbol de proyectos a partir del `<contexto>`.                                               |
| `axes <contexto> info`                            | Muestra toda la configuración fusionada para un proyecto e info general.                                                  |
| `axes <contexto> start`                           | Inicia una sesión de shell interactiva en el contexto del proyecto.                                        |
| `axes <contexto> run <script> [params...]`        | Ejecuta un script definido en `[commands]`.                                                                |
| `axes <contexto> open [with] [app]`               | Abre el proyecto usando una aplicación definida en `[options.open_with]`.                                  |
| `axes <contexto> rename <nuevo-nombre>`           | Renombra un proyecto de forma segura.                                                                      |
| `axes <contexto> link <nuevo-padre>`              | Cambia el padre de un proyecto, moviéndolo en el árbol.                                                    |
| `axes <contexto> unregister [--children]`         | Elimina un proyecto (y opcionalmente sus hijos) del índice de `axes`. **No borra archivos.**              |
| `axes <contexto> delete [--children]`             | ☢️ **DESTRUCTIVO:** Desregistra un proyecto (y sus hijos) Y borra su directorio `.axes/`.                   |
| `axes alias [set\|list\|rm] [args...]`             | Gestiona los alias de proyectos.                                                                           |

## Contribuciones

¡Las contribuciones son bienvenidas! Si encuentras un error, tienes una idea para una nueva característica, o quieres mejorar la documentación, por favor abre un issue o un pull request en este repositorio de GitHub. Lo agradeceremos muchisimo!

## Licencia

Este proyecto está licenciado bajo la Licencia [MIT](https://github.com/Retype15/axes/blob/main/LICENSE).
