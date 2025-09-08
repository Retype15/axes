// src/core/interpolator.rs

// Corregimos el import. `config` es un módulo hermano dentro de `core`.
use super::config::ResolvedConfig;

pub struct Interpolator<'a> {
    config: &'a ResolvedConfig,
    params: &'a [String],
}

impl<'a> Interpolator<'a> {
    pub fn new(config: &'a ResolvedConfig, params: &'a [String]) -> Self {
        Self { config, params }
    }

    /// Interpola una cadena de texto, reemplazando todos los tokens conocidos.
    pub fn interpolate(&self, input: &str) -> String {
        let pass1 = self.interpolate_reserved(input);
        let pass2 = self.interpolate_vars(&pass1);
        self.interpolate_params(&pass2)
    }

    fn interpolate_params(&self, input: &str) -> String {
        let params_str = self.params.join(" ");
        input.replace("{params}", &params_str)
    }

    /// Reemplaza tokens reservados y metadatos del proyecto.
    fn interpolate_reserved(&self, input: &str) -> String {
        let mut result = input.to_string();

        // {root} - Ahora accedemos a través de `self.config`
        if let Some(root_str) = self.config.project_root.to_str() {
            result = result.replace("{root}", root_str);
        }

        // {name} - Ahora accedemos a través de `self.config`
        result = result.replace("{name}", &self.config.project_name);

        // {version} - Ahora accedemos a través de `self.config`
        let version = self.config.version.as_deref().unwrap_or("");
        result = result.replace("{version}", version);

        // {description} - Ahora accedemos a través de `self.config`
        let description = self.config.description.as_deref().unwrap_or("");
        result = result.replace("{description}", description);

        result
    }

    /// Reemplaza tokens personalizados de la sección [vars].
    fn interpolate_vars(&self, input: &str) -> String {
        let mut result = input.to_string();
        // Ahora iteramos sobre `self.config.vars`
        for (key, value) in &self.config.vars {
            let token = format!("{{{}}}", key);
            let interpolated_value = self.interpolate_reserved(value);
            result = result.replace(&token, &interpolated_value);
        }
        result
    }
}
