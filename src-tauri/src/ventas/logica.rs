use crate::error::AppError;

pub fn validar_cantidad_item(cantidad: i64) -> Result<(), AppError> {
    if cantidad <= 0 {
        return Err(AppError::Negocio("Cantidad inválida, debe ser mayor a 0".to_string()));
    }
    Ok(())
}

pub fn validar_pagos_venta(total_venta: i64, pagos_ingresados: i64) -> Result<(), AppError> {
    if total_venta <= 0 {
        return Err(AppError::Negocio("No se puede finalizar una venta con total 0".to_string()));
    }
    if pagos_ingresados != total_venta {
        return Err(AppError::Negocio(format!(
            "La suma de los pagos (${}) no coincide con el total de la venta (${})",
            pagos_ingresados, total_venta
        )));
    }
    Ok(())
}

#[derive(Clone, Debug)]
pub struct ItemPromoCalc {
    pub id_producto: i64,
    pub cant: i64,
    pub costo_unit: i64,
    pub precio_catalogo: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ItemPromoResultado {
    pub id_producto: i64,
    pub cantidad_base: i64,
    pub precio_unitario_base: i64,
    pub cantidad_resto: i64,
    pub precio_unitario_resto: i64,
    pub costo_unitario: i64,
}

pub fn calcular_prorrateo_combo(
    precio_pack_aplicar: i64,
    precio_min_total: i64,
    items: &[ItemPromoCalc]
) -> Result<Vec<ItemPromoResultado>, AppError> {
    
    if precio_pack_aplicar <= 0 {
        return Err(AppError::Negocio("El precio del pack no está definido o es 0.".to_string()));
    }
    if precio_pack_aplicar < precio_min_total {
        return Err(AppError::Negocio(format!(
            "El precio del pack (${}) no puede ser menor al mínimo (${}).",
            precio_pack_aplicar, precio_min_total
        )));
    }
    if items.is_empty() {
        return Err(AppError::Negocio("El combo no tiene productos asignados.".to_string()));
    }

    let mut calc_interna: Vec<(i64, i64, i64, i64, i64, i64)> = items.iter().map(|it| {
        let base_total = it.precio_catalogo.checked_mul(it.cant).unwrap_or(0);
        (it.id_producto, it.cant, it.costo_unit, base_total, 0, 0)
    }).collect();

    let base_sum: i64 = calc_interna.iter().map(|x| x.3).sum();
    if base_sum <= 0 {
        return Err(AppError::Negocio("No se puede prorratear: suma base de catálogo es 0.".to_string()));
    }

    let mut asign_sum: i64 = 0;
    for it in calc_interna.iter_mut() {
        let num = it.3.checked_mul(precio_pack_aplicar).unwrap_or(0);
        it.4 = num / base_sum; // asign_total
        it.5 = num % base_sum; // resto para sort
        asign_sum += it.4;
    }

    let mut faltante = precio_pack_aplicar - asign_sum;
    if faltante > 0 {
        calc_interna.sort_by(|a, b| b.5.cmp(&a.5));
        for it in calc_interna.iter_mut() {
            if faltante == 0 { break; }
            it.4 += 1;
            faltante -= 1;
        }
    }

    let mut resultados = Vec::new();
    for it in calc_interna {
        let (id_prod, cant, costo_unit, _, asign_total, _) = it;
        let base_unit = asign_total / cant;
        let rem_units = asign_total % cant;
        let cant_base = cant - rem_units;

        resultados.push(ItemPromoResultado {
            id_producto: id_prod,
            cantidad_base: cant_base,
            precio_unitario_base: base_unit,
            cantidad_resto: rem_units,
            precio_unitario_resto: base_unit + 1,
            costo_unitario: costo_unit,
        });
    }

    Ok(resultados)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rechaza_cantidad_negativa() {
        assert!(validar_cantidad_item(0).is_err());
        assert!(validar_cantidad_item(-5).is_err());
    }

    #[test]
    fn rechaza_venta_sin_pagos_completos() {
        assert!(validar_pagos_venta(1000, 500).is_err());
        assert!(validar_pagos_venta(1000, 1000).is_ok());
    }
}