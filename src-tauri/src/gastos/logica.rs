// src-tauri/src/gastos/logica.rs
//
// Lógica de negocio pura del módulo gastos: sin acceso a base de datos,
// sin async. Cubre las validaciones que hoy están repetidas entre
// sueldo_registrar y gasto_registrar en commands.rs.

/// Normaliza un campo de texto obligatorio (trim) y rechaza vacío.
/// Sirve tanto para `descripcion` (sueldo) como `categoria` (gasto).
pub fn normalizar_texto_requerido(texto: &str, nombre_campo: &str) -> Result<String, String> {
    let limpio = texto.trim().to_string();
    if limpio.is_empty() {
        return Err(format!("{} obligatoria", nombre_campo));
    }
    Ok(limpio)
}

/// Normaliza un campo de texto opcional: trim, y si queda vacío
/// se convierte en None (en vez de guardar un string vacío).
pub fn normalizar_texto_opcional(texto: Option<String>) -> Option<String> {
    texto
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Valida que un monto sea positivo (> 0). Aplica igual a sueldos y gastos.
pub fn validar_monto_positivo(monto: i64) -> Result<(), String> {
    if monto <= 0 {
        return Err("monto inválido (> 0)".to_string());
    }
    Ok(())
}

/// El sueldo requiere sí o sí un usuario destino; convierte el Option
/// en un error claro si no vino, en vez de dejarlo como un unwrap
/// disperso en el comando.
pub fn requerir_usuario_destino(id: Option<i64>) -> Result<i64, String> {
    id.ok_or_else(|| "Tenés que seleccionar el usuario destino del sueldo".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- normalizar_texto_requerido ---

    #[test]
    fn acepta_texto_valido() {
        assert_eq!(
            normalizar_texto_requerido("  Sueldo de enero  ", "descripcion").unwrap(),
            "Sueldo de enero"
        );
    }

    #[test]
    fn rechaza_texto_vacio() {
        assert!(normalizar_texto_requerido("", "descripcion").is_err());
    }

    #[test]
    fn rechaza_texto_solo_espacios() {
        assert!(normalizar_texto_requerido("    ", "categoria").is_err());
    }

    #[test]
    fn mensaje_de_error_incluye_nombre_de_campo() {
        let err = normalizar_texto_requerido("", "categoria").unwrap_err();
        assert!(err.contains("categoria"));
    }

    // --- normalizar_texto_opcional ---

    #[test]
    fn texto_opcional_con_contenido_se_mantiene() {
        assert_eq!(
            normalizar_texto_opcional(Some("  algo  ".to_string())),
            Some("algo".to_string())
        );
    }

    #[test]
    fn texto_opcional_vacio_se_convierte_en_none() {
        assert_eq!(normalizar_texto_opcional(Some("   ".to_string())), None);
    }

    #[test]
    fn texto_opcional_none_se_mantiene_none() {
        assert_eq!(normalizar_texto_opcional(None), None);
    }

    // --- validar_monto_positivo ---

    #[test]
    fn acepta_monto_positivo() {
        assert!(validar_monto_positivo(1000).is_ok());
    }

    #[test]
    fn rechaza_monto_cero() {
        assert!(validar_monto_positivo(0).is_err());
    }

    #[test]
    fn rechaza_monto_negativo() {
        assert!(validar_monto_positivo(-500).is_err());
    }

    // --- requerir_usuario_destino ---

    #[test]
    fn acepta_usuario_destino_presente() {
        assert_eq!(requerir_usuario_destino(Some(7)).unwrap(), 7);
    }

    #[test]
    fn rechaza_usuario_destino_ausente() {
        assert!(requerir_usuario_destino(None).is_err());
    }
}