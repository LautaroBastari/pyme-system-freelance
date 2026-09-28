use tauri::State;
use serde::{Serialize, Deserialize};

use crate::AppState;
use crate::stock::repo::{self, TipoPrecio};
use crate::stock::logica;
use crate::stock::model::ReposicionModo;
use super::model::{StockMermaInput, CompraStockInput};
use crate::error::AppError;

/*  Listar / Buscar  */
#[derive(Serialize)]
pub struct StockResumen {
    pub id_producto: i64,
    pub codigo: String,
    pub nombre: String,
    pub stock_actual: i64,
    pub precio_venta_actual: i64,
    pub costo_actual: i64,
    pub activo: i64,
}

#[derive(Debug, Serialize)]
pub struct StockReporteProducto {
    pub id_producto: i64,
    pub codigo_producto: String,
    pub nombre: String,
    pub stock_actual: i64,
    pub costo_unitario: i64,
    pub valor_total: i64,
    pub porcentaje_valor: f64,

    pub rotacion_dias: Option<f64>,
    pub dias_stock_restante: Option<f64>,
    pub clasificacion_abc: Option<String>,
    pub variacion_pct: Option<f64>,
    pub riesgo: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct StockReporteResultado {
    pub total_inventario: i64,
    pub cantidad_productos: i64,
    pub productos: Vec<StockReporteProducto>,
}

#[derive(serde::Deserialize)]
pub struct FixAbsInput {
    pub id_producto: i64,
    pub nuevo: i64,
    pub motivo: Option<String>,
    pub referencia: Option<String>,
}

#[tauri::command]
pub fn stock_listar(
    state: tauri::State<'_, crate::app_state::AppState>,
    q: Option<String>,
    solo_activos: Option<bool>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<StockResumen>, AppError> {
    let q = q.unwrap_or_default();
    let solo_activos = solo_activos.unwrap_or(true);
    let limit = limit.unwrap_or(200).max(1);
    let offset = offset.unwrap_or(0).max(0);

    let rows = tauri::async_runtime::block_on(async move {
        sqlx::query(
            r#"
            SELECT p.id_producto,
                p.codigo_producto,
                p.nombre,
                COALESCE(ps.stock_actual, 0) AS stock_actual,
                p.precio_venta_actual,
                p.costo_actual,
                p.activo
            FROM producto p
            LEFT JOIN producto_stock ps USING(id_producto)
            WHERE (?1 = '' OR p.nombre LIKE '%'||?1||'%' OR p.codigo_producto LIKE '%'||?1||'%')
            ORDER BY p.id_producto ASC
            LIMIT ?2 OFFSET ?3
            "#
        )
        .bind(&q)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.pool)
        .await
    })?;

    let out = rows
        .into_iter()
        .map(|r| {
            use sqlx::Row;
            StockResumen {
                id_producto: r.get(0),
                codigo: r.get(1),
                nombre: r.get(2),
                stock_actual: r.get(3),
                precio_venta_actual: r.get(4),
                costo_actual: r.get(5),
                activo: r.get(6),
            }
        })
        .collect();

    Ok(out)
}

/* Crear producto  */
#[derive(Deserialize)]
pub struct ProductoCrearIn {
    pub codigo: String,
    pub nombre: String,
    pub precio_venta: i64,
    pub costo: i64,
    pub reposicion_modo: ReposicionModo,
    pub reposicion_factor: i64,
}

#[derive(Serialize)]
pub struct ProductoIdOut { pub id_producto: i64 }

#[tauri::command]
pub async fn producto_crear(
    state: State<'_, AppState>,
    input: ProductoCrearIn
) -> Result<ProductoIdOut, AppError> {
    // La detección de código duplicado ahora está encapsulada en el repo
    let id = repo::producto_crear(
        &state.pool,
        &input.codigo,
        &input.nombre,
        input.precio_venta,
        input.costo,
        input.reposicion_modo,
        input.reposicion_factor,
    ).await?;

    Ok(ProductoIdOut { id_producto: id })
}

/* Editar básicos  */
#[derive(Deserialize)]
pub struct ProductoActualizarIn {
    pub id_producto: i64,
    pub codigo: Option<String>,
    pub nombre: Option<String>,
    pub activo: Option<i64>, 
}

#[tauri::command]
pub async fn producto_actualizar(state: State<'_, AppState>, input: ProductoActualizarIn)
-> Result<(), AppError> {
    if input.id_producto <= 0 { return Err(AppError::Negocio("id_producto inválido".into())); }
    
    repo::producto_actualizar(
        &state.pool,
        input.id_producto,
        input.codigo.as_deref(),
        input.nombre.as_deref(),
        input.activo
    ).await?;
    Ok(())
}

/* Eliminar/Restaurar (soft delete)  */
#[derive(Deserialize)]
pub struct ProductoSetActivoIn { pub id_producto: i64, pub activo: bool }

#[tauri::command]
pub async fn producto_set_activo(state: State<'_, AppState>, input: ProductoSetActivoIn)
-> Result<(), AppError> {
    if input.id_producto <= 0 { return Err(AppError::Negocio("id_producto inválido".into())); }
    repo::producto_set_activo(&state.pool, input.id_producto, input.activo).await?;
    Ok(())
}

/* Ajustar stock  */
#[derive(Deserialize)]
pub struct StockAjusteIn { pub id_producto: i64, pub delta: i64, pub motivo: String, pub referencia: Option<String> }
#[derive(Serialize)]   
pub struct StockAjusteOut { pub id_movimiento: i64, pub stock_nuevo: i64 }

#[tauri::command]
pub async fn stock_ajustar(state: State<'_, AppState>, input: StockAjusteIn)
-> Result<StockAjusteOut, AppError> {
    if input.id_producto <= 0 { return Err(AppError::Negocio("id_producto inválido".into())); }
    if input.delta == 0 { return Err(AppError::Negocio("El delta no puede ser 0".into())); }
    if input.motivo.trim().is_empty() { return Err(AppError::Negocio("El motivo es requerido".into())); }

    let id_mov = repo::stock_ajustar(
        &state.pool, input.id_producto, input.delta, &input.motivo, input.referencia.as_deref()
    ).await?;

    let stock_nuevo: i64 = sqlx::query_scalar(
        "SELECT stock_actual FROM producto_stock WHERE id_producto=?1"
    )
    .bind(input.id_producto)
    .fetch_one(&state.pool)
    .await?;

    Ok(StockAjusteOut { id_movimiento: id_mov, stock_nuevo })
}

/* Actualizar precio  */
#[derive(Deserialize)]
pub struct PrecioActualizarIn { pub id_producto: i64, pub tipo: String, pub nuevo: i64 }

#[tauri::command]
pub async fn precio_actualizar(
    state: tauri::State<'_, crate::app_state::AppState>,
    input: PrecioActualizarIn,
) -> Result<(), AppError> {
    if input.id_producto <= 0 { return Err(AppError::Negocio("id_producto inválido".into())); }
    if input.nuevo < 0 { return Err(AppError::Negocio("El precio no puede ser negativo".into())); }

    let t = match input.tipo.as_str() {
        "venta" => TipoPrecio::Venta,
        "costo" => TipoPrecio::Costo,
        _ => return Err(AppError::Negocio("Tipo inválido (usa 'venta' o 'costo')".into()))
    };

    repo::precio_actualizar(&state.pool, input.id_producto, t, input.nuevo).await?;
    Ok(())
}

/* Historiales  */
#[derive(Deserialize)]
pub struct HistStockIn { pub id_producto: i64, pub limit: i64 }
#[derive(Deserialize)]
pub struct HistPrecioIn { pub id_producto: i64, pub tipo: Option<String>, pub limit: i64 }

#[derive(Serialize)]
pub struct StockMovOut { pub id_movimiento: i64, pub cantidad_delta: i64, pub motivo: String, pub referencia: Option<String>, pub fecha_hora: String }
#[derive(Serialize)]
pub struct PrecioHistOut { pub id_precio: i64, pub tipo: String, pub precio: i64, pub vigente_desde: String, pub vigente_hasta: Option<String> }

#[tauri::command]
pub async fn stock_mov_listar(state: State<'_, AppState>, input: HistStockIn)
-> Result<Vec<StockMovOut>, AppError> {
    let rows = repo::stock_mov_listar(&state.pool, input.id_producto, input.limit).await?;
    Ok(rows.into_iter().map(|r| StockMovOut {
        id_movimiento: r.id_movimiento, cantidad_delta: r.cantidad_delta,
        motivo: r.motivo, referencia: r.referencia, fecha_hora: r.fecha_hora
    }).collect())
}

#[tauri::command]
pub async fn precio_hist_listar(state: State<'_, AppState>, input: HistPrecioIn)
-> Result<Vec<PrecioHistOut>, AppError> {
    let rows = repo::precio_hist_listar(
        &state.pool, input.id_producto, input.tipo.as_deref(), input.limit
    ).await?;
    Ok(rows.into_iter().map(|r| PrecioHistOut {
        id_precio: r.id_precio, tipo: r.tipo, precio: r.precio,
        vigente_desde: r.vigente_desde, vigente_hasta: r.vigente_hasta
    }).collect())
}

#[tauri::command]
pub async fn stock_fijar_absoluto(
    state: tauri::State<'_, AppState>,
    input: FixAbsInput,
) -> Result<(), AppError> {
    let pool = &state.pool;
    let mut tx = pool.begin().await?;

    let actual: i64 = sqlx::query_scalar(
        "SELECT stock_actual FROM producto_stock WHERE id_producto = ?1"
    )
    .bind(input.id_producto)
    .fetch_one(&mut *tx)
    .await?;

    let costo_actual: i64 = sqlx::query_scalar(
        "SELECT costo_actual FROM producto WHERE id_producto = ?1"
    )
    .bind(input.id_producto)
    .fetch_one(&mut *tx)
    .await?;

    // Mapeamos el Result<T, String> de logica.rs a nuestro AppError::Negocio
    let (delta, costo_unitario_mov, total_costo_mov) =
        logica::calcular_ajuste_absoluto(actual, input.nuevo, costo_actual)
            .map_err(AppError::Negocio)?;

    if delta != 0 {
        sqlx::query(
            "INSERT INTO stock_mov (
                 id_producto,
                 cantidad_delta,
                 motivo,
                 referencia,
                 costo_unitario,
                 total_costo,
                 fecha_hora
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, CURRENT_TIMESTAMP)"
        )
        .bind(input.id_producto)
        .bind(delta)
        .bind(input.motivo.unwrap_or_else(|| "ajuste_absoluto_ui".to_string()))
        .bind(input.referencia.unwrap_or_default())
        .bind(costo_unitario_mov)
        .bind(total_costo_mov)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

#[tauri::command]
pub fn reporte_stock_general(
    state: State<'_, AppState>,
    solo_activos: Option<bool>,
) -> Result<StockReporteResultado, AppError> {
    use std::collections::HashMap;
    use sqlx::Row;

    let pool = &state.pool;
    let solo_activos = solo_activos.unwrap_or(true);

    let filas = tauri::async_runtime::block_on(async {
        sqlx::query(
            r#"
            SELECT
                p.id_producto,
                p.codigo_producto,
                p.nombre,
                COALESCE(ps.stock_actual, 0) AS stock_actual,
                p.costo_actual               AS costo_unitario
            FROM producto p
            LEFT JOIN producto_stock ps
                ON ps.id_producto = p.id_producto
            WHERE (p.activo = 1 OR ?1 = 0)
            ORDER BY p.nombre ASC
            "#
        )
        .bind(if solo_activos { 1 } else { 0 })
        .fetch_all(pool)
        .await
    })?;

    let ventas_rows = tauri::async_runtime::block_on(async {
        sqlx::query(
            r#"
            SELECT
                vi.id_producto                           AS id_producto,
                SUM(
                  CASE
                    WHEN v.fecha_hora >= DATETIME('now','localtime','-30 day')
                    THEN vi.cantidad
                    ELSE 0
                  END
                ) AS cant_30,
                SUM(
                  CASE
                    WHEN v.fecha_hora >= DATETIME('now','localtime','-60 day')
                     AND v.fecha_hora < DATETIME('now','localtime','-30 day')
                    THEN vi.cantidad
                    ELSE 0
                  END
                ) AS cant_prev_30
            FROM venta v
            JOIN venta_item vi ON vi.id_venta = v.id_venta
            WHERE v.estado = 'finalizada'
            GROUP BY vi.id_producto
            "#
        )
        .fetch_all(pool)
        .await
    })?;

    let mut ventas_map: HashMap<i64, (i64, i64)> = HashMap::new();
    for r in ventas_rows {
        let id: i64 = r.get("id_producto");
        let c30: i64 = r.get("cant_30");
        let cprev: i64 = r.get("cant_prev_30");
        ventas_map.insert(id, (c30, cprev));
    }

    let periodo_dias = 30.0_f64;
    let mut total_inventario: i64 = 0;
    let mut productos: Vec<StockReporteProducto> = Vec::new();

    for r in filas {
        let id_producto: i64 = r.get("id_producto");
        let codigo_producto: String = r.get("codigo_producto");
        let nombre: String = r.get("nombre");
        let stock_actual: i64 = r.get("stock_actual");
        let costo_unitario: i64 = r.get("costo_unitario");

        let valor_total = logica::calcular_valor_total(stock_actual, costo_unitario);
        total_inventario += valor_total;

        let (ventas_30, ventas_prev_30) = ventas_map.get(&id_producto).cloned().unwrap_or((0, 0));

        let rotacion_dias = logica::calcular_rotacion_dias(stock_actual, ventas_30, periodo_dias);
        let dias_stock_restante = logica::calcular_dias_restante(stock_actual, ventas_30, periodo_dias);
        let variacion_pct = logica::calcular_variacion_pct(ventas_30, ventas_prev_30);

        productos.push(StockReporteProducto {
            id_producto,
            codigo_producto,
            nombre,
            stock_actual,
            costo_unitario,
            valor_total,
            porcentaje_valor: 0.0,
            rotacion_dias,
            dias_stock_restante,
            clasificacion_abc: None,
            variacion_pct,
            riesgo: None,
        });
    }

    for p in &mut productos {
        p.porcentaje_valor = logica::calcular_porcentaje_valor(p.valor_total, total_inventario);
    }

    let mut indices: Vec<usize> = (0..productos.len()).collect();
    indices.sort_by(|&i, &j| productos[j].valor_total.cmp(&productos[i].valor_total));

    let mut acumulado = 0.0_f64;

    for idx in indices {
        let p = &mut productos[idx];
        acumulado += p.porcentaje_valor;
        p.clasificacion_abc = Some(logica::clasificar_abc(acumulado).to_string());
        p.riesgo = Some(logica::calcular_riesgo(p.stock_actual, p.dias_stock_restante).to_string());
    }

    Ok(StockReporteResultado {
        total_inventario,
        cantidad_productos: productos.len() as i64,
        productos,
    })
}

#[tauri::command(rename = "stock_registrar_merma")]
pub async fn stock_registrar_merma(
    state: State<'_, AppState>,
    input: StockMermaInput,
) -> Result<(), AppError> {
    if input.cantidad <= 0 {
        return Err(AppError::Negocio("La cantidad debe ser mayor a cero".to_string()));
    }

    repo::registrar_merma(&state.pool, input).await?;
    Ok(())
}

#[tauri::command(rename = "stock_compra")]
pub async fn stock_compra(
    state: State<'_, AppState>,
    input: CompraStockInput,
) -> Result<(), AppError> {
    let (cantidad_maples, costo_unitario) =
        logica::calcular_compra(input.cantidad, input.unidad.as_str(), input.costo_total)
            .map_err(AppError::Negocio)?;

    let referencia = format!(
        "{} {} → {} maples",
        input.cantidad, input.unidad, cantidad_maples
    );

    let pool = &state.pool;
    let mut tx = pool.begin().await?;

    repo::stock_compra_tx(
        &mut tx,
        input.id_producto,
        cantidad_maples,
        costo_unitario,
        input.costo_total,
        referencia,
    ).await?;
    
    tx.commit().await?;
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct ReposicionRangoIn {
    pub desde: String,                 
    pub hasta: String,                 
    pub solo_activos: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct StockReposicionRowOut {
    pub id_producto: i64,
    pub codigo_producto: String,
    pub nombre: String,
    pub vendidos: i64,
    pub reposicion_modo: String,
    pub reposicion_factor: i64,
}

#[tauri::command(rename = "reporte_stock_reposicion")]
pub async fn reporte_stock_reposicion(
    state: State<'_, AppState>,
    input: ReposicionRangoIn,
) -> Result<Vec<StockReposicionRowOut>, AppError> {
    let solo_activos = input.solo_activos.unwrap_or(true);

    let rows = repo::reporte_stock_reposicion_rango(
        &state.pool,
        &input.desde,
        &input.hasta,
        solo_activos,
    ).await?;

    Ok(rows
        .into_iter()
        .map(|r| StockReposicionRowOut {
            id_producto: r.id_producto,
            codigo_producto: r.codigo_producto,
            nombre: r.nombre,
            vendidos: r.vendidos,
            reposicion_modo: r.reposicion_modo,
            reposicion_factor: r.reposicion_factor,
        })
        .collect())
}

#[derive(Debug, Deserialize)]
pub struct ProductoActualizarReposicionIn {
    pub id_producto: i64,
    pub reposicion_modo: String,
    pub reposicion_factor: i64,
}

#[tauri::command(rename = "producto_actualizar_reposicion")]
pub async fn producto_actualizar_reposicion(
    state: State<'_, AppState>,
    input: ProductoActualizarReposicionIn,
) -> Result<(), AppError> {
    repo::producto_actualizar_reposicion(
        &state.pool,
        input.id_producto,
        &input.reposicion_modo,
        input.reposicion_factor,
    ).await?;
    Ok(())
}