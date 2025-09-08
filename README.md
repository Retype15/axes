# `axes`: Un Gestor de Flujo de Trabajo de Proyecto Holístico

**Estado Actual: v0.1.0-alpha** (Versión de prueba para Windows. ¡Se agradece el feedback!)

`axes` es una herramienta de línea de comandos diseñada para unificar, simplificar y automatizar la forma en que interactúas con todos tus proyectos de software, sin importar el lenguaje o la tecnología que utilicen. Actúa como una capa de abstracción universal que te permite gestionar entornos, ejecutar tareas y navegar por tus proyectos con una sintaxis coherente y potente.

¿Cansado de recordar si debes usar `npm run dev`, `cargo run`, `docker-compose up` o `source .venv/bin/activate` para cada proyecto? Con `axes`, solo necesitas recordar `axes <proyecto> start`.

## 🧭 Filosofía y Visión

La complejidad del desarrollo de software no debería estar en recordar comandos, sino en resolver problemas. `axes` nace de la necesidad de reducir la carga cognitiva del desarrollador moderno, que a menudo trabaja en múltiples proyectos, repositorios y tecnologías simultáneamente.

### 🎯 Objetivos

- **Abstracción Universal:** Proporcionar una única interfaz (`axes ...`) para interactuar con cualquier tipo de proyecto (Python, Rust, Node.js, etc.).
- **Configuración Explícita:** Cada proyecto se autodefine a través de un archivo `axes.toml` claro y legible. Sin magia oculta, sin comportamientos impredecibles.
- **Ergonomía Primero:** Ofrecer una experiencia de usuario fluida a través de una sintaxis intuitiva (`proyecto -> acción -> detalles`), atajos inteligentes y una interfaz de gestión interactiva.
- **Gestión Holística:** Ser más que un simple ejecutor de tareas o un gestor de entorno. `axes` integra la **inicialización**, la **configuración del entorno**, la **ejecución de tareas** y la **gestión multi-proyecto** en un único flujo de trabajo cohesivo.
- **Seguridad y Robustez:** Ejecutar todas las tareas en procesos aislados, sin evaluar código dinámico, y manejar los errores de forma proactiva y amigable para el usuario.

### ✨ Ventajas Clave

| Característica | ¿Qué problema resuelve? |
| :--- | :--- |
| **Interfaz Unificada (`axes.toml`)** | Elimina la necesidad de recordar comandos específicos (`npm`, `cargo`, `pip`) para cada proyecto. `axes . run build` siempre funciona. |
| **Modo Terminal (`axes start`)** | Automatiza la configuración del entorno (activación de `venv`, exportación de variables) con hooks `at_start` y `at_exit`, sumergiéndote en un contexto de desarrollo listo para usar. |
| **Gestión Multi-Proyecto Centralizada** | Te permite listar, encontrar y ejecutar comandos en cualquiera de tus proyectos registrados desde cualquier lugar de tu sistema de archivos (`axes mi-api run test`). |
| **TUI de Gestión (`axes`)** | Un lanzador interactivo para navegar, buscar y actuar sobre tus proyectos sin necesidad de memorizar nombres o comandos. Ideal para el descubrimiento. |
| **Configuración en Cascada** | Define configuraciones globales (`~/.config/axes/axes.toml`) que todos tus proyectos pueden heredar, promoviendo la consistencia y el principio DRY. |
| **Auto-reparación Inteligente** | Si mueves o eliminas un proyecto, `axes` lo detecta y te ofrece interactivamente relocalizarlo o limpiar la entrada del índice, manteniendo tu configuración sincronizada. |

## 🚀 Instalación (Versión Alfa)

Actualmente, `axes` debe ser compilado desde el código fuente.

**Requisitos:**

- [Rust](https://www.rust-lang.org/tools/install) (toolchain `stable`)

**Pasos:**

1. Clona el repositorio:

    ```sh
    git clone https://github.com/Retype15/axes.git
    cd axes
    ```

2. Compila el proyecto en modo `release` (optimizado):

    ```sh
    cargo build --release
    ```

3. El binario `axes.exe` se encontrará en `target/release/`. Se recomienda copiar este archivo a una ubicación que esté en tu `PATH` del sistema para poder llamarlo desde cualquier lugar.

## ⚙️ Conceptos Fundamentales

### 1. El Índice Global (`index.toml`)

`axes` mantiene un registro central de todos tus proyectos en `~/.config/axes/index.toml` (o el equivalente en tu SO). Este archivo mapea un nombre de proyecto único a la ruta de su directorio raíz.

### 2. La Configuración Global (`axes.toml`)

En el mismo directorio (`~/.config/axes/`), puedes crear un archivo `axes.toml`. Este archivo te permite definir `[vars]`, `[options]` y `[commands]` que estarán disponibles para **todos** tus proyectos.

### 3. El Proyecto (`.axes/axes.toml`)

Cada proyecto gestionado por `axes` debe contener un directorio `.axes` en su raíz, y dentro de él, un archivo `axes.toml`. Este archivo es el corazón del proyecto.

**Estructura de `axes.toml`:**

```toml
# --- Metadatos (Obligatorio) ---
name = "mi-api-backend"

# --- Metadatos (Opcional) ---
version = "1.2.0"
description = "El backend principal para la aplicación."

# --- Comandos Personalizados ---
# Atajos para tareas comunes.
[commands]
# Sintaxis simple
dev = "docker-compose up -d"
stop = "docker-compose down"

# Sintaxis extendida con descripción
test = { run = "pytest -v {params}", desc = "Ejecuta los tests. Acepta parámetros de pytest." }

# --- Opciones de Sesión (Hooks) ---
[options]
at_start = "source .env" # Se ejecuta al entrar en `axes start`
at_exit = "docker-compose down" # Se ejecuta al salir de la sesión
shell = "bash" # (Futuro) Especifica la shell a usar

# --- Variables Personalizadas ---
[vars]
db_user = "admin"
db_url = "postgres://{db_user}@localhost:5432/main_db"
```

### 4. Configuración en Cascada

Cuando `axes` opera sobre un proyecto, fusiona la configuración de la siguiente manera (los de abajo sobrescriben a los de arriba):

1. Lee el `axes.toml` global (`~/.config/axes/axes.toml`).
2. Lee el `axes.toml` del proyecto (`<proyecto>/.axes/axes.toml`).
3. **Resultado:**
    - Las `[vars]` se fusionan, con las del proyecto teniendo prioridad.
    - Las `[options]` se fusionan, con las del proyecto teniendo prioridad.
    - Los `[commands]` son **reemplazados** por los del proyecto. No se fusionan.

## 📖 Guía de Uso y Ejemplos

### Escenario: Configurando un Proyecto Python

Imagina que tienes un proyecto web en `C:\dev\my-web-app`.

#### 1. Inicializar el Proyecto con `axes`

Navega al directorio y ejecuta `init`.

```sh
cd C:\dev\my-web-app
axes my-web-app init
```

Esto crea el directorio `.axes` y un `axes.toml` básico, y registra "my-web-app" en tu índice global.

#### 2. Configurar `axes.toml`

Abre `.axes/axes.toml` y configúralo para tu flujo de trabajo de Python:

```toml
name = "my-web-app"
version = "0.1.0"
description = "Una aplicación web con Flask y venv."

[commands]
# Instala dependencias
install = { run = "pip install -r requirements.txt", desc = "Instala las dependencias de Python." }
# Lanza el servidor de desarrollo
dev = { run = "flask run", desc = "Inicia el servidor de desarrollo de Flask." }
# Ejecuta los tests
test = { run = "pytest", desc = "Ejecuta la suite de tests." }

[options]
# Activa el entorno virtual al iniciar una sesión
at_start = ".venv\\Scripts\\activate.bat && set FLASK_APP=src/app.py"
# Define el host de Flask
```

*(Nota: Puede usar comandos encadenados, pero para múltiples comandos muy personalizados o complejos en `at_start` o `at_exit`, se recomienda crear un script `setup_env.bat` y llamarlo)*
*(Nota 2: En un futuro cercano habrá el grupo [env] para configurar las variables de entorno antes de `at_start` simplemente declarandolas)*

```bat
# setup_env.bat
call .venv\Scripts\activate.bat
set FLASK_APP=src/app.py
```

```toml
# axes.toml actualizado
[options]
at_start = "setup_env.bat"
```

#### 3. Trabajar en el Proyecto

- **Opción A: Modo Terminal (Inmersivo)**

    Es la forma recomendada para el desarrollo diario.

    ```sh
    # Desde cualquier lugar del sistema
    axes my-web-app start
    ```

  - `axes` ejecuta `setup_env.bat`    automáticamente.
  - Aterrizas en una nueva `cmd.exe` con el   prompt `(.venv) C:\dev\my-web-app>`.
  - Tu `venv` está activado y `FLASK_APP`     está definida.
  - Ahora puedes usar tus herramientas    nativas:

    ```sh
        # Dentro de la sesión
        (.venv)> flask run
        (.venv)> pytest
        (.venv)> axes list  # Para ver los  comandos de `axes` disponibles
        (.venv)> axes test  # Atajo para `run test`
        (.venv)> exit       # Sale de la sesión
    ```

- **Opción B: Modo Script (Puntual)**

    Perfecto para tareas rápidas o  automatización.

    ```sh
    # Ejecutar tests desde cualquier lugar
    axes my-web-app run test
    
    # Ejecutar tests del proyecto en el que     estoy (estando en C:\dev\my-web-app\tests)
    axes . test
    
    # Instalar dependencias del último proyecto     en el que trabajé
    axes * install
    ```

#### 4. Usar la TUI

Si no recuerdas los nombres o simplemente quieres explorar:

```sh
axes
```

Aparecerá un menú interactivo que te guiará para seleccionar `my-web-app` y luego la acción que deseas realizar (`Start Session`, `Run Command`, etc.).

### Sintaxis de Comandos de `axes`

La estructura general es `axes [CONTEXTO] [ACCIÓN] [ARGUMENTOS...]`

- **`[CONTEXTO]`**: Sobre qué proyecto actuar.
  - `<nombre-proyecto>`: Un proyecto registrado. (Ej: `my-web-app`)
  - `.`: El proyecto encontrado en el directorio actual (o superior).
  - `_`: El proyecto en el directorio actual (sin buscar hacia arriba, útil para anidar).
  - `*`: El último proyecto con el que interactuaste.
- **`[ACCIÓN]`**: Qué hacer.
  - `start` (o nada): Inicia una sesión interactiva.
  - `run <script> [params...]`: Ejecuta un comando de `[commands]`.
  - `init`: Crea y registra un nuevo proyecto.
  - `info`, `edit`, `open`, `register`, `rename`: Acciones de gestión.
  - `<script>`: Atajo para `run <script>`.

## 🔮 Hoja de Ruta (Características Planeadas)

Esta versión alfa es solo el comienzo. Aquí hay un vistazo a lo que viene:

- **Soporte Multi-Shell Completo:** Detección y configuración automática para `bash`, `zsh`, `PowerShell`, etc., para una portabilidad total a Linux y macOS.
- **Comando `axes validate`:** Una herramienta de diagnóstico para verificar la integridad de tus proyectos y configuraciones.
- **Autocompletado de la Shell:** `Tab` para autocompletar nombres de proyectos y acciones.
- **Sistema de Plantillas Avanzado:** Plantillas que pueden pedir variables al usuario y ejecutar scripts post-inicialización.
- **Ejecución de Hooks en Modo Script (`--with-env`):** Para tareas de CI/CD que requieren configuración y limpieza de entorno.

## ❤️ Contribuciones y Feedback

¡Este es un proyecto en desarrollo activo! El feedback es increíblemente valioso en esta etapa. Si encuentras un bug, tienes una idea para una nueva característica, o simplemente quieres compartir tu experiencia, por favor [abre un Issue en GitHub](https://github.com/Retype15/axes/issues).

---
