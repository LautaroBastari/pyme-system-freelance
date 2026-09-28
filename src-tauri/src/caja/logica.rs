// src-tauri/src/caja/logica.rs
//
// Lógica de negocio pura del módulo caja: sin acceso a base de datos,
// sin async. Recibe datos ya obtenidos por repo.rs y decide/calcula.
// Esto permite testear las reglas de negocio sin levantar SQLite.

/// Suma los montos de una lista de pagos. Devuelve 0 si la lista está vacía.
/// Rechaza montos negativos (un pago no puede restar).
pub fn calcular_total(montos: &[i64]) -> Result<i64, String> {
    if montos.iter().any(|&m| m < 0) {
        return Err("Un monto de pago no puede ser negativo".to_string());
    }
    Ok(montos.iter().sum())
}

/// Agrupa montos por medio de pago y devuelve el total por cada uno.
/// Rechaza montos negativos. El orden del resultado es determinístico
/// (alfabético) para que los tests sean estables.
pub fn agrupar_por_medio(pagos: &[(String, i64)]) -> Result<Vec<(String, i64)>, String> {
    use std::collections::HashMap;

    let mut mapa: HashMap<String, i64> = HashMap::new();
    for (medio, monto) in pagos {
        if *monto < 0 {
            return Err(format!("Monto inválido para medio '{}': {}", medio, monto));
        }
        *mapa.entry(medio.clone()).or_insert(0) += monto;
    }

    let mut resultado: Vec<(String, i64)> = mapa.into_iter().collect();
    resultado.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(resultado)
}

/// Decide si corresponde autocerrar una caja abierta antes de abrir una nueva.
/// Hoy la regla siempre autocierra; queda explícita y testeable acá.
pub fn debe_autocerrar(hay_caja_abierta: bool) -> bool {
    hay_caja_abierta
}

/// Regla de negocio: no se puede cerrar una caja si hay una venta en curso.
/// Extraída de repo::cerrar_caja, que antes tenía esta decisión mezclada
/// con la query SQL.
pub fn validar_cierre_caja(hay_venta_en_curso: bool) -> Result<(), String> {
    if hay_venta_en_curso {
        return Err("No se puede cerrar caja: hay una venta en curso.".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- calcular_total ---

    #[test]
    fn suma_montos_simples() {
        assert_eq!(calcular_total(&[1000, 2000, 500]).unwrap(), 3500);
    }

    #[test]
    fn lista_vacia_da_cero() {
        assert_eq!(calcular_total(&[]).unwrap(), 0);
    }

    #[test]
    fn rechaza_monto_negativo() {
        assert!(calcular_total(&[1000, -50]).is_err());
    }

    // --- agrupar_por_medio ---

    #[test]
    fn agrupa_por_medio_correctamente() {
        let pagos = vec![
            ("efectivo".to_string(), 1000),
            ("tarjeta".to_string(), 2000),
            ("efectivo".to_string(), 500),
        ];
        let resultado = agrupar_por_medio(&pagos).unwrap();
        assert_eq!(
            resultado,
            vec![
                ("efectivo".to_string(), 1500),
                ("tarjeta".to_string(), 2000),
            ]
        );
    }

    #[test]
    fn agrupar_lista_vacia_da_vacio() {
        let pagos: Vec<(String, i64)> = vec![];
        assert_eq!(agrupar_por_medio(&pagos).unwrap(), vec![]);
    }

    #[test]
    fn agrupar_rechaza_negativos() {
        let pagos = vec![("efectivo".to_string(), -100)];
        assert!(agrupar_por_medio(&pagos).is_err());
    }

    // --- debe_autocerrar ---

    #[test]
    fn autocierra_si_hay_caja_abierta() {
        assert!(debe_autocerrar(true));
        assert!(!debe_autocerrar(false));
    }

    // --- validar_cierre_caja ---

    #[test]
    fn no_permite_cerrar_con_venta_en_curso() {
        assert!(validar_cierre_caja(true).is_err());
    }

    #[test]
    fn permite_cerrar_sin_venta_en_curso() {
        assert!(validar_cierre_caja(false).is_ok());
    }
}