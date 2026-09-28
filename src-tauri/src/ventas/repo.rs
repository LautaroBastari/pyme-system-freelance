use sqlx::{SqlitePool, Row};
use crate::error::AppError;
use super::commands::{ProductoDisponible, VentaItemDto, AgregarItemInput, SetCantidadInput, QuitarItemInput, VentaFinalizarInput, HistorialItem};
use super::model::PromoComboAplicarInput;
use super::logica::{self, ItemPromoCalc};

pub async fn obtener_productos_disponibles(pool: &SqlitePool) -> Result<Vec<ProductoDisponible>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT
          p.id_producto AS id_producto,
          p.nombre      AS nombre,
          COALESCE(
            (SELECT ph.precio
               FROM precio_historial ph
              WHERE ph.id_producto = p.id_producto
                AND ph.tipo = 'venta'
                AND ph.vigente_hasta IS NULL
              ORDER BY ph.vigente_desde DESC
              LIMIT 1),
            p.precio_venta_actual
          )                          AS precio_unitario,
          COALESCE(ps.stock_actual,0) AS stock_disponible
        FROM producto p
        LEFT JOIN producto_stock ps ON ps.id_producto = p.id_producto
        WHERE p.activo = 1
        ORDER BY p.nombre
        "#
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| ProductoDisponible {
        id_producto: r.get("id_producto"),
        nombre: r.get("nombre"),
        precio_unitario: r.get("precio_unitario"),
        stock_disponible: r.get("stock_disponible"),
    }).collect())
}

pub async fn iniciar_venta_repo(pool: &SqlitePool, uid: i64) -> Result<i64, AppError> {
    let id_caja = sqlx::query_scalar::<_, i64>(
        "SELECT id_caja FROM caja WHERE estado='abierta' LIMIT 1",
    )
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::Negocio("No hay caja abierta, abrí caja antes de vender".into()))?;

    let res = sqlx::query(
        "INSERT INTO venta(id_usuario, id_caja, fecha_hora, total, estado)
         VALUES(?, ?, DATETIME('now','localtime'), 0, 'en_curso')"
    )
    .bind(uid)
    .bind(id_caja)
    .execute(pool)
    .await?;

    Ok(res.last_insert_rowid())
}

pub async fn agregar_item_repo(pool: &SqlitePool, input: AgregarItemInput) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;

    let existente = sqlx::query(
        "SELECT id_item, cantidad, precio_unitario
        FROM venta_item
        WHERE id_venta = ? 
            AND id_producto = ?
            AND fuente_precio = 'catalogo'
        LIMIT 1",
    )
    .bind(input.id_venta)
    .bind(input.id_producto)
    .fetch_optional(&mut *tx)
    .await?;

    if let Some(row) = existente {
        let id_item: i64 = row.get("id_item");
        let cantidad_actual: i64 = row.get("cantidad");
        let precio_unitario: i64 = row.get("precio_unitario");

        let nueva_cantidad = cantidad_actual + input.cantidad;
        let nuevo_subtotal = nueva_cantidad * precio_unitario;

        sqlx::query("UPDATE venta_item SET cantidad = ?, subtotal = ? WHERE id_item = ?")
            .bind(nueva_cantidad)
            .bind(nuevo_subtotal)
            .bind(id_item)
            .execute(&mut *tx)
            .await?;
    } else {
        let row = sqlx::query(
            r#"
            SELECT
            COALESCE(
                (SELECT NULLIF(ph.precio, 0) FROM precio_historial ph WHERE ph.id_producto = ? AND ph.tipo = 'venta' AND ph.vigente_hasta IS NULL ORDER BY ph.vigente_desde DESC LIMIT 1),
                (SELECT p.precio_venta_actual FROM producto p WHERE p.id_producto = ?)
            ) AS precio_unitario,
            COALESCE(
                (SELECT NULLIF(phc.precio, 0) FROM precio_historial phc WHERE phc.id_producto = ? AND phc.tipo = 'costo' AND phc.vigente_hasta IS NULL ORDER BY phc.vigente_desde DESC LIMIT 1),
                (SELECT p.costo_actual FROM producto p WHERE p.id_producto = ?)
            ) AS costo_unitario_en_venta
            "#
        )
        .bind(input.id_producto)
        .bind(input.id_producto)
        .bind(input.id_producto)
        .bind(input.id_producto)
        .fetch_one(&mut *tx)
        .await?;

        let precio_unitario: i64 = row.get("precio_unitario");
        let costo_unitario_en_venta: i64 = row.get("costo_unitario_en_venta");
        let subtotal = precio_unitario * input.cantidad;

        sqlx::query(
            "INSERT INTO venta_item (id_venta, id_producto, cantidad, precio_unitario, costo_unitario_en_venta, fuente_precio, subtotal)
             VALUES(?, ?, ?, ?, ?, 'catalogo', ?)",
        )
        .bind(input.id_venta)
        .bind(input.id_producto)
        .bind(input.cantidad)
        .bind(precio_unitario)
        .bind(costo_unitario_en_venta)
        .bind(subtotal)
        .execute(&mut *tx)
        .await?;
    }

    sqlx::query(
        "UPDATE venta SET total = (SELECT COALESCE(SUM(subtotal),0) FROM venta_item WHERE id_venta = ?) WHERE id_venta = ?",
    )
    .bind(input.id_venta)
    .bind(input.id_venta)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}

pub async fn listar_items_venta(pool: &SqlitePool, id_venta: i64) -> Result<(Vec<VentaItemDto>, i64), AppError> {
    let items_rows = sqlx::query(
        r#"
        SELECT vi.id_item, vi.id_producto, p.nombre, vi.cantidad, vi.precio_unitario, vi.subtotal
        FROM venta_item vi
        JOIN producto p ON p.id_producto = vi.id_producto
        WHERE vi.id_venta = ? ORDER BY vi.id_item
        "#
    )
    .bind(id_venta)
    .fetch_all(pool)
    .await?;

    let items = items_rows.into_iter().map(|r| VentaItemDto {
        id_item: r.get("id_item"),
        id_producto: r.get("id_producto"),
        nombre: r.get("nombre"),
        cantidad: r.get("cantidad"),
        precio_unitario: r.get("precio_unitario"),
        subtotal: r.get("subtotal"),
    }).collect();

    let total: i64 = sqlx::query_scalar("SELECT total FROM venta WHERE id_venta = ?")
        .bind(id_venta)
        .fetch_one(pool)
        .await?;

    Ok((items, total))
}

pub async fn set_cantidad_item(pool: &SqlitePool, input: SetCantidadInput) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;

    sqlx::query("UPDATE venta_item SET cantidad = ?, subtotal = ? * precio_unitario WHERE id_item = ?")
        .bind(input.cantidad)
        .bind(input.cantidad)
        .bind(input.id_item)
        .execute(&mut *tx)
        .await?;

    sqlx::query(
        "UPDATE venta SET total = (SELECT COALESCE(SUM(subtotal),0) FROM venta_item WHERE id_venta = (SELECT id_venta FROM venta_item WHERE id_item=?))
          WHERE id_venta = (SELECT id_venta FROM venta_item WHERE id_item=?)",
    )
    .bind(input.id_item)
    .bind(input.id_item)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}

pub async fn quitar_item(pool: &SqlitePool, input: QuitarItemInput) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;
    
    let id_venta: Option<i64> = sqlx::query_scalar("SELECT id_venta FROM venta_item WHERE id_item=?")
        .bind(input.id_item).fetch_optional(&mut *tx).await?;

    sqlx::query("DELETE FROM venta_item WHERE id_item=?").bind(input.id_item).execute(&mut *tx).await?;

    if let Some(v) = id_venta {
        sqlx::query("UPDATE venta SET total = (SELECT COALESCE(SUM(subtotal),0) FROM venta_item WHERE id_venta=?) WHERE id_venta=?")
            .bind(v).bind(v).execute(&mut *tx).await?;
    }

    tx.commit().await?;
    Ok(())
}

pub async fn cancelar_venta(pool: &SqlitePool, id_venta: i64) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM venta_item WHERE id_venta=?").bind(id_venta).execute(&mut *tx).await?;
    sqlx::query("UPDATE venta SET estado='anulada', total=0 WHERE id_venta=?").bind(id_venta).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

pub async fn finalizar_venta_repo(pool: &SqlitePool, input: VentaFinalizarInput, suma_pagos: i64) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;

    let estado: String = sqlx::query_scalar("SELECT estado FROM venta WHERE id_venta = ?")
        .bind(input.id_venta).fetch_one(&mut *tx).await?;

    if estado != "en_curso" {
        return Err(AppError::Negocio("La venta no está en curso o ya fue finalizada".into()));
    }

    let items = sqlx::query("SELECT id_producto, cantidad, costo_unitario_en_venta FROM venta_item WHERE id_venta = ?")
        .bind(input.id_venta).fetch_all(&mut *tx).await?;

    if items.is_empty() { return Err(AppError::Negocio("No se puede finalizar una venta sin items".into())); }

    let total: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(subtotal), 0) FROM venta_item WHERE id_venta = ?")
        .bind(input.id_venta).fetch_one(&mut *tx).await?;

    logica::validar_pagos_venta(total, suma_pagos)?;

    sqlx::query(
        r#"INSERT INTO stock_mov (id_producto, cantidad_delta, motivo, referencia, costo_unitario, total_costo, fecha_hora)
           SELECT vi.id_producto, -vi.cantidad, 'venta', ?, vi.costo_unitario_en_venta, vi.cantidad * vi.costo_unitario_en_venta, DATETIME('now','localtime')
           FROM venta_item vi WHERE vi.id_venta = ?"#
    )
    .bind(format!("venta:{}", input.id_venta))
    .bind(input.id_venta)
    .execute(&mut *tx)
    .await?;

    sqlx::query("DELETE FROM venta_pago WHERE id_venta = ?").bind(input.id_venta).execute(&mut *tx).await?;

    for pago in &input.pagos {
        sqlx::query("INSERT INTO venta_pago (id_venta, medio, monto, referencia) VALUES (?, ?, ?, ?)")
            .bind(input.id_venta).bind(&pago.medio).bind(pago.monto).bind(&pago.referencia).execute(&mut *tx).await?;
    }

    sqlx::query("UPDATE venta SET estado = 'finalizada', total = ? WHERE id_venta = ?")
        .bind(total).bind(input.id_venta).execute(&mut *tx).await?;

    tx.commit().await?;
    Ok(())
}

pub async fn obtener_historial_hoy(pool: &SqlitePool, uid: i64) -> Result<Vec<HistorialItem>, AppError> {
    let items_rows = sqlx::query(
        r#"
        SELECT v.id_venta, time(v.fecha_hora, 'localtime') AS hora, p.codigo_producto, p.nombre AS producto,
               vi.cantidad, vi.precio_unitario, vi.subtotal, v.total AS total_venta,
               (SELECT GROUP_CONCAT(medio || ' $' || monto, ' + ') FROM venta_pago WHERE id_venta = v.id_venta) AS pagos_detalle
        FROM venta v
        JOIN venta_item vi ON vi.id_venta = v.id_venta
        JOIN producto p ON p.id_producto = vi.id_producto
        WHERE v.id_usuario = ? AND v.estado = 'finalizada'
          AND v.fecha_hora >= datetime('now','localtime','start of day')
          AND v.fecha_hora <  datetime('now','localtime','start of day','+1 day')
        ORDER BY v.fecha_hora ASC, vi.id_item
        "#
    )
    .bind(uid)
    .fetch_all(pool)
    .await?;

    Ok(items_rows.into_iter().map(|r| HistorialItem {
        id_venta: r.get("id_venta"), hora: r.get("hora"),
        codigo_producto: r.get("codigo_producto"), producto: r.get("producto"),
        cantidad: r.get("cantidad"), precio_unitario: r.get("precio_unitario"),
        subtotal: r.get("subtotal"), total_venta: r.get("total_venta"),
        pagos_detalle: r.get("pagos_detalle"),
    }).collect())
}

pub async fn aplicar_promo_combo_repo(pool: &SqlitePool, input: PromoComboAplicarInput) -> Result<(), AppError> {
    let combo = sqlx::query("SELECT id_combo, activo, precio_min_total, precio_pack FROM promo_combo WHERE id_combo = ?")
        .bind(input.id_combo).fetch_optional(pool).await?
        .ok_or_else(|| AppError::Negocio("Combo no encontrado.".into()))?;

    let activo: i64 = combo.get("activo");
    if activo != 1 { return Err(AppError::Negocio("El combo está inactivo.".into())); }

    let precio_min_total: i64 = combo.get("precio_min_total");
    let precio_pack_db: i64 = combo.get("precio_pack");
    let precio_pack_aplicar = if input.precio_total_pack > 0 { input.precio_total_pack } else { precio_pack_db };

    let items_rows = sqlx::query("SELECT id_producto, cantidad FROM promo_combo_item WHERE id_combo = ?")
        .bind(input.id_combo).fetch_all(pool).await?;

    let mut calc_input = Vec::new();
    for r in items_rows {
        let id_producto: i64 = r.get("id_producto");
        let cant: i64 = r.get("cantidad");

        let prod = sqlx::query("SELECT precio_venta_actual, costo_actual FROM producto WHERE id_producto = ? AND activo = 1")
            .bind(id_producto).fetch_optional(pool).await?
            .ok_or_else(|| AppError::Negocio(format!("Producto {id_producto} inactivo.")))?;

        calc_input.push(ItemPromoCalc {
            id_producto, cant, costo_unit: prod.get("costo_actual"), precio_catalogo: prod.get("precio_venta_actual")
        });
    }

    let prorrateo = logica::calcular_prorrateo_combo(precio_pack_aplicar, precio_min_total, &calc_input)?;
    let mut tx = pool.begin().await?;

    for res in prorrateo {
        if res.cantidad_base > 0 {
            sqlx::query(
                "INSERT INTO venta_item (id_venta, id_producto, cantidad, precio_unitario, costo_unitario_en_venta, fuente_precio, subtotal)
                 VALUES (?, ?, ?, ?, ?, 'promo', ?)"
            ).bind(input.id_venta).bind(res.id_producto).bind(res.cantidad_base).bind(res.precio_unitario_base)
             .bind(res.costo_unitario).bind(res.cantidad_base * res.precio_unitario_base).execute(&mut *tx).await?;
        }
        if res.cantidad_resto > 0 {
            sqlx::query(
                "INSERT INTO venta_item (id_venta, id_producto, cantidad, precio_unitario, costo_unitario_en_venta, fuente_precio, subtotal)
                 VALUES (?, ?, ?, ?, ?, 'promo', ?)"
            ).bind(input.id_venta).bind(res.id_producto).bind(res.cantidad_resto).bind(res.precio_unitario_resto)
             .bind(res.costo_unitario).bind(res.cantidad_resto * res.precio_unitario_resto).execute(&mut *tx).await?;
        }
    }

    tx.commit().await?;
    Ok(())
}