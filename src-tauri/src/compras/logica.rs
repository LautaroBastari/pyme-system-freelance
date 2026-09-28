// src-tauri/src/compras/logica.rs
//
// Lógica de negocio pura del módulo compras: sin acceso a base de datos,
// sin async.

/// Valida los datos de entrada de una compra antes de tocar la DB.
/// Antes solo se validaba `cantidad > 0`; acá sumamos `id_producto` y
/// `costo_unitario`, que no se estaban validando (podía registrarse
/// una compra con costo negativo).
pub fn validar_compra(id_producto: i64, cantidad: i64, costo_unitario: i64) -> Result<(), String> {
    if id_producto <= 0 {
        return Err("id_producto inválido".to_string());
    }
    if cantidad <= 0 {
        return Err("La cantidad debe ser positiva".to_string());
    }
    if costo_unitario < 0 {
        return Err("El costo unitario no puede ser negativo".to_string());
    }
    Ok(())
}

/// Decide qué costo unitario se usa realmente para la compra.
/// Si `mantener_costo` es true, se respeta el costo ya cargado en el
/// producto (costo_actual) en vez del que vino en la compra.
pub fn determinar_costo_efectivo(
    mantener_costo: bool,
    costo_unitario_ingresado: i64,
    costo_actual_producto: i64,
) -> i64 {
    if mantener_costo {
        costo_actual_producto
    } else {
        costo_unitario_ingresado
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- validar_compra ---

    #[test]
    fn compra_valida_pasa() {
        assert!(validar_compra(1, 10, 500).is_ok());
    }

    #[test]
    fn rechaza_id_producto_invalido() {
        assert!(validar_compra(0, 10, 500).is_err());
        assert!(validar_compra(-1, 10, 500).is_err());
    }

    #[test]
    fn rechaza_cantidad_cero_o_negativa() {
        assert!(validar_compra(1, 0, 500).is_err());
        assert!(validar_compra(1, -5, 500).is_err());
    }

    #[test]
    fn rechaza_costo_negativo() {
        assert!(validar_compra(1, 10, -1).is_err());
    }

    #[test]
    fn permite_costo_cero() {
        // costo 0 es válido (ej: mercadería donada/promocional), solo se rechaza negativo
        assert!(validar_compra(1, 10, 0).is_ok());
    }

    // --- determinar_costo_efectivo ---

    #[test]
    fn usa_costo_ingresado_si_no_mantiene_costo() {
        assert_eq!(determinar_costo_efectivo(false, 300, 250), 300);
    }

    #[test]
    fn usa_costo_actual_si_mantiene_costo() {
        assert_eq!(determinar_costo_efectivo(true, 300, 250), 250);
    }
}