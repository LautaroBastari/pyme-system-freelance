// src-tauri/src/stock/logica.rs
//
// Lógica de negocio pura del módulo stock: sin acceso a base de datos,
// sin async. Recibe datos ya obtenidos por repo.rs/commands.rs y
// calcula/decide. Esto permite testear las reglas de negocio (muchas
// con plata de por medio) sin levantar SQLite.

// ---------------------------------------------------------------------
// Compra de stock (MAPLE / CAJON)
// ---------------------------------------------------------------------

/// Convierte la unidad de compra a su factor de conversión a maples.
/// Antes vivía en repo.rs sin necesidad (no toca la DB).
pub fn factor_por_unidad(unidad: &str) -> Result<i64, String> {
    match unidad {
        "MAPLE" => Ok(1),
        "CAJON" => Ok(12),
        _ => Err("Unidad inválida (MAPLE|CAJON)".to_string()),
    }
}

/// Calcula cuántos maples entran y el costo unitario resultante
/// de una compra. Devuelve (cantidad_maples, costo_unitario).
pub fn calcular_compra(
    cantidad: i64,
    unidad: &str,
    costo_total: i64,
) -> Result<(i64, i64), String> {
    if cantidad <= 0 {
        return Err("Cantidad inválida".to_string());
    }
    if costo_total < 0 {
        return Err("Costo inválido".to_string());
    }

    let factor = factor_por_unidad(unidad)?;
    let cantidad_maples = cantidad * factor;

    if cantidad_maples <= 0 {
        return Err("Cantidad resultante inválida".to_string());
    }

    // división entera (floor) — igual que el código original
    let costo_unitario = costo_total / cantidad_maples;

    Ok((cantidad_maples, costo_unitario))
}

// ---------------------------------------------------------------------
// Ajuste absoluto de stock (stock_fijar_absoluto)
// ---------------------------------------------------------------------

/// Calcula el delta a aplicar y el costo económico del ajuste.
/// Devuelve (delta, costo_unitario_mov, total_costo_mov).
///
/// Regla: si el delta es negativo (se está sacando stock), se
/// registra como pérdida al costo actual. Si es positivo, se
/// trata como corrección sin costo económico (no se "gana" plata
/// por aparecer stock de la nada).
pub fn calcular_ajuste_absoluto(
    actual: i64,
    nuevo: i64,
    costo_actual: i64,
) -> Result<(i64, i64, i64), String> {
    if nuevo < 0 {
        return Err("Stock objetivo inválido (< 0)".to_string());
    }

    let delta = nuevo - actual;

    if delta == 0 {
        return Ok((0, 0, 0));
    }

    let (costo_unitario_mov, total_costo_mov) = if delta < 0 {
        let unidades = -delta;
        (costo_actual, unidades * costo_actual)
    } else {
        (0, 0)
    };

    Ok((delta, costo_unitario_mov, total_costo_mov))
}

// ---------------------------------------------------------------------
// Merma (registrar_merma)
// ---------------------------------------------------------------------

/// Calcula el delta (negativo) y el costo total de una merma.
/// Devuelve (cantidad_delta, total_costo).
pub fn calcular_merma(costo_unitario: i64, cantidad: i64) -> Result<(i64, i64), String> {
    if cantidad <= 0 {
        return Err("La cantidad debe ser mayor a cero".to_string());
    }
    Ok((-cantidad, costo_unitario * cantidad))
}

// ---------------------------------------------------------------------
// Reposición automática (producto_actualizar_reposicion)
// ---------------------------------------------------------------------

/// Valida modo + factor de reposición y decide el factor final.
/// Si el modo es "unitario", fuerza el factor a 12 (regla de negocio
/// existente). Devuelve (modo_validado, factor_final).
pub fn validar_reposicion(modo: &str, factor: i64) -> Result<(&'static str, i64), String> {
    let modo_valido = match modo {
        "unitario" => "unitario",
        "cajon" => "cajon",
        _ => return Err("reposicion_modo inválido (unitario|cajon)".to_string()),
    };

    if modo_valido == "cajon" && factor <= 0 {
        return Err("reposicion_factor debe ser > 0".to_string());
    }

    let factor_final = if modo_valido == "unitario" { 12 } else { factor };
    Ok((modo_valido, factor_final))
}

// ---------------------------------------------------------------------
// Reporte general de stock (reporte_stock_general)
// ---------------------------------------------------------------------

/// Valor total del stock de un producto (cantidad × costo unitario).
pub fn calcular_valor_total(stock_actual: i64, costo_unitario: i64) -> i64 {
    stock_actual * costo_unitario
}

/// Porcentaje que representa un producto sobre el valor total del inventario.
pub fn calcular_porcentaje_valor(valor_total: i64, total_inventario: i64) -> f64 {
    if total_inventario <= 0 {
        return 0.0;
    }
    (valor_total as f64) / (total_inventario as f64) * 100.0
}

/// Días de "rotación": cuánto tardaría el período en vender un stock
/// equivalente al actual. None si no hay ventas o no hay stock.
pub fn calcular_rotacion_dias(stock_actual: i64, ventas_30: i64, periodo_dias: f64) -> Option<f64> {
    if ventas_30 <= 0 || stock_actual <= 0 {
        return None;
    }
    Some(periodo_dias * (stock_actual as f64 / ventas_30 as f64))
}

/// Estimación de días de stock restante según ritmo de venta actual.
pub fn calcular_dias_restante(stock_actual: i64, ventas_30: i64, periodo_dias: f64) -> Option<f64> {
    if ventas_30 <= 0 {
        return None;
    }
    let ventas_diarias = ventas_30 as f64 / periodo_dias;
    if ventas_diarias <= 0.0 {
        return None;
    }
    Some(stock_actual.max(0) as f64 / ventas_diarias)
}

/// Variación porcentual de ventas vs. el período de 30 días anterior.
pub fn calcular_variacion_pct(ventas_30: i64, ventas_prev_30: i64) -> Option<f64> {
    if ventas_30 == 0 && ventas_prev_30 == 0 {
        return None;
    }
    if ventas_prev_30 == 0 {
        return Some(100.0);
    }
    Some(((ventas_30 as f64 - ventas_prev_30 as f64) / ventas_prev_30 as f64) * 100.0)
}

/// Clasificación ABC según el porcentaje de valor acumulado (curva de Pareto).
pub fn clasificar_abc(porcentaje_acumulado: f64) -> &'static str {
    if porcentaje_acumulado <= 80.0 {
        "A"
    } else if porcentaje_acumulado <= 95.0 {
        "B"
    } else {
        "C"
    }
}

/// Nivel de riesgo de quiebre de stock según cantidad actual y días restantes.
pub fn calcular_riesgo(stock_actual: i64, dias_restante: Option<f64>) -> &'static str {
    const DIAS_RIESGO_ALTO: f64 = 7.0;
    const DIAS_RIESGO_MEDIO: f64 = 30.0;

    let dias = dias_restante.unwrap_or(f64::INFINITY);

    if stock_actual <= 0 {
        "alto"
    } else if dias <= DIAS_RIESGO_ALTO {
        "alto"
    } else if dias <= DIAS_RIESGO_MEDIO {
        "medio"
    } else {
        "bajo"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- factor_por_unidad / calcular_compra ---

    #[test]
    fn factor_maple_es_uno() {
        assert_eq!(factor_por_unidad("MAPLE").unwrap(), 1);
    }

    #[test]
    fn factor_cajon_es_doce() {
        assert_eq!(factor_por_unidad("CAJON").unwrap(), 12);
    }

    #[test]
    fn factor_unidad_invalida_falla() {
        assert!(factor_por_unidad("KG").is_err());
    }

    #[test]
    fn calcula_compra_en_cajones() {
        // 2 cajones (24 maples) por 2400 -> 100 el maple
        let (maples, costo) = calcular_compra(2, "CAJON", 2400).unwrap();
        assert_eq!(maples, 24);
        assert_eq!(costo, 100);
    }

    #[test]
    fn calcula_compra_rechaza_cantidad_cero() {
        assert!(calcular_compra(0, "MAPLE", 1000).is_err());
    }

    #[test]
    fn calcula_compra_rechaza_costo_negativo() {
        assert!(calcular_compra(1, "MAPLE", -100).is_err());
    }

    #[test]
    fn calcula_compra_redondea_hacia_abajo() {
        // 10 maples por 105 -> 10 (floor), no 10.5
        let (_, costo) = calcular_compra(10, "MAPLE", 105).unwrap();
        assert_eq!(costo, 10);
    }

    // --- calcular_ajuste_absoluto ---

    #[test]
    fn ajuste_sin_cambio_no_genera_costo() {
        let (delta, costo_u, total) = calcular_ajuste_absoluto(50, 50, 100).unwrap();
        assert_eq!((delta, costo_u, total), (0, 0, 0));
    }

    #[test]
    fn ajuste_hacia_abajo_genera_perdida() {
        // de 50 a 30 -> se perdieron 20 unidades a costo 100 c/u
        let (delta, costo_u, total) = calcular_ajuste_absoluto(50, 30, 100).unwrap();
        assert_eq!(delta, -20);
        assert_eq!(costo_u, 100);
        assert_eq!(total, 2000);
    }

    #[test]
    fn ajuste_hacia_arriba_no_genera_costo() {
        // de 50 a 80 -> corrección, sin costo económico
        let (delta, costo_u, total) = calcular_ajuste_absoluto(50, 80, 100).unwrap();
        assert_eq!(delta, 30);
        assert_eq!(costo_u, 0);
        assert_eq!(total, 0);
    }

    #[test]
    fn ajuste_rechaza_stock_objetivo_negativo() {
        assert!(calcular_ajuste_absoluto(50, -1, 100).is_err());
    }

    // --- calcular_merma ---

    #[test]
    fn merma_calcula_delta_negativo_y_costo() {
        let (delta, total) = calcular_merma(150, 4).unwrap();
        assert_eq!(delta, -4);
        assert_eq!(total, 600);
    }

    #[test]
    fn merma_rechaza_cantidad_cero_o_negativa() {
        assert!(calcular_merma(150, 0).is_err());
        assert!(calcular_merma(150, -1).is_err());
    }

    // --- validar_reposicion ---

    #[test]
    fn reposicion_unitario_fuerza_factor_doce() {
        let (modo, factor) = validar_reposicion("unitario", 5).unwrap();
        assert_eq!(modo, "unitario");
        assert_eq!(factor, 12);
    }

    #[test]
    fn reposicion_cajon_respeta_factor_dado() {
        let (modo, factor) = validar_reposicion("cajon", 24).unwrap();
        assert_eq!(modo, "cajon");
        assert_eq!(factor, 24);
    }

    #[test]
    fn reposicion_cajon_rechaza_factor_invalido() {
        assert!(validar_reposicion("cajon", 0).is_err());
    }

    #[test]
    fn reposicion_modo_invalido_falla() {
        assert!(validar_reposicion("kilos", 5).is_err());
    }

    // --- reporte: valor total / porcentaje ---

    #[test]
    fn valor_total_multiplica_stock_por_costo() {
        assert_eq!(calcular_valor_total(10, 250), 2500);
    }

    #[test]
    fn porcentaje_valor_normal() {
        assert_eq!(calcular_porcentaje_valor(2500, 10000), 25.0);
    }

    #[test]
    fn porcentaje_valor_con_inventario_cero_no_divide_por_cero() {
        assert_eq!(calcular_porcentaje_valor(2500, 0), 0.0);
    }

    // --- rotación / días restantes ---

    #[test]
    fn rotacion_dias_normal() {
        // 30 vendidos en 30 días, stock actual 60 -> tarda 60 días en rotar
        let r = calcular_rotacion_dias(60, 30, 30.0).unwrap();
        assert_eq!(r, 60.0);
    }

    #[test]
    fn rotacion_dias_none_sin_ventas() {
        assert!(calcular_rotacion_dias(60, 0, 30.0).is_none());
    }

    #[test]
    fn rotacion_dias_none_sin_stock() {
        assert!(calcular_rotacion_dias(0, 30, 30.0).is_none());
    }

    #[test]
    fn dias_restante_normal() {
        // 30 vendidos en 30 días = 1/día; con 15 de stock -> 15 días restantes
        let d = calcular_dias_restante(15, 30, 30.0).unwrap();
        assert_eq!(d, 15.0);
    }

    #[test]
    fn dias_restante_none_sin_ventas() {
        assert!(calcular_dias_restante(15, 0, 30.0).is_none());
    }

    #[test]
    fn dias_restante_con_stock_negativo_no_rompe() {
        // stock negativo se trata como 0 (max(0))
        let d = calcular_dias_restante(-5, 30, 30.0).unwrap();
        assert_eq!(d, 0.0);
    }

    // --- variación % ---

    #[test]
    fn variacion_none_sin_ventas_en_ningun_periodo() {
        assert!(calcular_variacion_pct(0, 0).is_none());
    }

    #[test]
    fn variacion_cien_por_ciento_si_antes_no_vendia() {
        assert_eq!(calcular_variacion_pct(10, 0).unwrap(), 100.0);
    }

    #[test]
    fn variacion_calcula_crecimiento() {
        // de 10 a 20 -> +100%
        assert_eq!(calcular_variacion_pct(20, 10).unwrap(), 100.0);
    }

    #[test]
    fn variacion_calcula_caida() {
        // de 20 a 10 -> -50%
        assert_eq!(calcular_variacion_pct(10, 20).unwrap(), -50.0);
    }

    // --- clasificación ABC ---

    #[test]
    fn clasifica_a_hasta_80() {
        assert_eq!(clasificar_abc(80.0), "A");
        assert_eq!(clasificar_abc(50.0), "A");
    }

    #[test]
    fn clasifica_b_entre_80_y_95() {
        assert_eq!(clasificar_abc(90.0), "B");
    }

    #[test]
    fn clasifica_c_mas_de_95() {
        assert_eq!(clasificar_abc(99.0), "C");
    }

    // --- riesgo ---

    #[test]
    fn riesgo_alto_sin_stock() {
        assert_eq!(calcular_riesgo(0, Some(100.0)), "alto");
    }

    #[test]
    fn riesgo_alto_pocos_dias_restantes() {
        assert_eq!(calcular_riesgo(10, Some(5.0)), "alto");
    }

    #[test]
    fn riesgo_medio() {
        assert_eq!(calcular_riesgo(10, Some(20.0)), "medio");
    }

    #[test]
    fn riesgo_bajo_con_dias_holgados() {
        assert_eq!(calcular_riesgo(10, Some(60.0)), "bajo");
    }

    #[test]
    fn riesgo_bajo_sin_dato_de_dias_pero_con_stock() {
        // producto con stock pero sin ventas (dias_restante = None -> infinito)
        assert_eq!(calcular_riesgo(10, None), "bajo");
    }
}