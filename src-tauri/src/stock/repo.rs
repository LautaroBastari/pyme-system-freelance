use sqlx::{SqlitePool, Row, sqlite::SqliteQueryResult};
use time::OffsetDateTime;
use super::model::StockMermaInput;
use sqlx::{Sqlite, Transaction};
use crate::stock::model::ReposicionModo;
use crate::stock::logica;
use crate::error::AppError;

#[derive(Copy, Clone, Debug)]
pub enum TipoPrecio { Venta, Costo }

impl TipoPrecio {
    pub fn as_str(&self) -> &'static str {
        match self {
            TipoPrecio::Venta => "venta",
            TipoPrecio::Costo => "costo",
        }
    }
}

/*  Crear producto  */
pub async fn producto_crear(
    pool: &SqlitePool,
    codigo: &str,
    nombre: &str,
    precio_venta: i64,
    costo: i64,
    reposicion_modo: ReposicionModo,
    reposicion_factor: i64,
) -> Result<i64, AppError> {
    if precio_venta < 0 || costo < 0 { return Err(AppError::Negocio("Los precios y costos no pueden ser negativos".into())); }
    if reposicion_factor <= 0 { return Err(AppError::Negocio("El factor de reposición debe ser mayor a cero".into())); }

    let mut tx = pool.begin().await?;

    let query_res = sqlx::query(
        "INSERT INTO producto(
            codigo_producto,
            nombre,
            precio_venta_actual,
            costo_actual,
            activo,
            reposicion_modo,
            reposicion_factor
         )
         VALUES (?1,?2,?3,?4,1,?5,?6)"
    )
    .bind(codigo)
    .bind(nombre)
    .bind(precio_venta)
    .bind(costo)
    .bind(reposicion_modo.as_str())
    .bind(reposicion_factor)
    .execute(&mut *tx)
    .await;

    // Manejo limpio del UNIQUE constraint desde la base de datos
    let res = match query_res {
        Ok(r) => r,
        Err(sqlx::Error::Database(db_err)) if db_err.code().as_deref() == Some("2067") => {
            return Err(AppError::Negocio("Este código ya existe. Elige otro código por favor.".into()));
        }
        Err(e) => return Err(e.into()),
    };

    let id = res.last_insert_rowid();

    sqlx::query(
        "INSERT INTO precio_historial(id_producto,tipo,precio,vigente_desde,vigente_hasta)
         VALUES (?1,'venta',?2, CURRENT_TIMESTAMP, NULL)"
    )
    .bind(id)
    .bind(precio_venta)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO precio_historial(id_producto,tipo,precio,vigente_desde,vigente_hasta)
         VALUES (?1,'costo',?2, CURRENT_TIMESTAMP, NULL)"
    )
    .bind(id)
    .bind(costo)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(id)
}

/*  Actualizar básicos  */
pub async fn producto_actualizar(
    pool: &SqlitePool,
    id_producto: i64,
    codigo: Option<&str>,
    nombre: Option<&str>,
    activo: Option<i64>,
) -> Result<(), AppError> {
    let mut sets = Vec::<(&str, String)>::new();
    if let Some(c) = codigo { sets.push(("codigo_producto", c.to_string())); }
    if let Some(n) = nombre { sets.push(("nombre", n.to_string())); }
    if let Some(a) = activo { sets.push(("activo", a.to_string())); }

    if sets.is_empty() { return Ok(()); }

    let mut sql = String::from("UPDATE producto SET ");
    for (i, (col, _)) in sets.iter().enumerate() {
        if i > 0 { sql.push_str(", "); }
        sql.push_str(col); sql.push_str(" = ?"); sql.push_str(&(i+1).to_string());
    }
    sql.push_str(" WHERE id_producto = ?"); sql.push_str(&(sets.len()+1).to_string());

    let mut q = sqlx::query(&sql);
    for (_, v) in &sets { q = q.bind(v); }
    q = q.bind(id_producto);
    
    let res = q.execute(pool).await;
    match res {
        Ok(_) => Ok(()),
        Err(sqlx::Error::Database(db_err)) if db_err.code().as_deref() == Some("2067") => {
            Err(AppError::Negocio("Este código ya existe. Elige otro código por favor.".into()))
        }
        Err(e) => Err(e.into()),
    }
}

pub async fn producto_set_activo(pool: &SqlitePool, id_producto: i64, activo: bool) -> Result<(), AppError> {
    sqlx::query("UPDATE producto SET activo=?1 WHERE id_producto=?2")
        .bind(if activo {1} else {0})
        .bind(id_producto)
        .execute(pool).await?;
    Ok(())
}

/* Ajuste de stock (delta)  */
pub async fn stock_ajustar(
    pool: &SqlitePool,
    id_producto: i64,
    delta: i64,
    motivo: &str,
    referencia: Option<&str>,
) -> Result<i64, AppError> {
    let mut tx = pool.begin().await?;

    let res = sqlx::query(
        "INSERT INTO stock_mov(id_producto,cantidad_delta,motivo,referencia,fecha_hora)
         VALUES (?1,?2,?3,?4,CURRENT_TIMESTAMP)"
    )
    .bind(id_producto)
    .bind(delta)
    .bind(motivo)
    .bind(referencia.unwrap_or(""))
    .execute(&mut *tx)
    .await;

    match res {
        Ok(r) => {
            let id = r.last_insert_rowid();
            tx.commit().await?;
            Ok(id)
        }
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("Stock negativo no permitido") {
                Err(AppError::Negocio("No podés descontar stock por debajo de 0".into()))
            } else {
                Err(e.into())
            }
        }
    }
}

/*  Actualizar precio (historial)  */
pub async fn precio_actualizar(
    pool: &SqlitePool,
    id_producto: i64,
    tipo: TipoPrecio,
    nuevo: i64,
) -> Result<i64, AppError> {
    let res = sqlx::query(
        "INSERT INTO precio_historial(id_producto,tipo,precio,vigente_desde,vigente_hasta)
         VALUES (?1,?2,?3,CURRENT_TIMESTAMP,NULL)"
    )
    .bind(id_producto)
    .bind(tipo.as_str())
    .bind(nuevo)
    .execute(pool)
    .await?;

    Ok(res.last_insert_rowid())
}

/*  Historias  */
pub struct StockMovRow {
    pub id_movimiento: i64,
    pub cantidad_delta: i64,
    pub motivo: String,
    pub referencia: Option<String>,
    pub fecha_hora: String,
}

pub async fn stock_mov_listar(
    pool: &SqlitePool,
    id_producto: i64,
    limit: i64,
) -> Result<Vec<StockMovRow>, AppError> {
    let rows = sqlx::query(
        "SELECT id_movimiento,cantidad_delta,motivo,referencia,fecha_hora
         FROM stock_mov WHERE id_producto=?1
         ORDER BY fecha_hora DESC
         LIMIT ?2"
    )
    .bind(id_producto)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| StockMovRow {
        id_movimiento: r.get(0),
        cantidad_delta: r.get(1),
        motivo: r.get(2),
        referencia: r.get(3),
        fecha_hora: r.get(4),
    }).collect())
}

pub struct PrecioHistRow {
    pub id_precio: i64,
    pub tipo: String,
    pub precio: i64,
    pub vigente_desde: String,
    pub vigente_hasta: Option<String>,
}

pub async fn precio_hist_listar(
    pool: &SqlitePool,
    id_producto: i64,
    tipo: Option<&str>,
    limit: i64,
) -> Result<Vec<PrecioHistRow>, AppError> {
    let (sql, bind_tipo) = if tipo.is_some() {
        (
            "SELECT id_precio,tipo,precio,vigente_desde,vigente_hasta
             FROM precio_historial
             WHERE id_producto=?1 AND tipo=?2
             ORDER BY vigente_desde DESC
             LIMIT ?3",
            true
        )
    } else {
        (
            "SELECT id_precio,tipo,precio,vigente_desde,vigente_hasta
             FROM precio_historial
             WHERE id_producto=?1
             ORDER BY vigente_desde DESC
             LIMIT ?2",
            false
        )
    };

    let mut q = sqlx::query(sql).bind(id_producto);
    if bind_tipo { q = q.bind(tipo.unwrap()); }
    q = q.bind(limit);

    let rows = q.fetch_all(pool).await?;
    Ok(rows.into_iter().map(|r| PrecioHistRow {
        id_precio: r.get(0),
        tipo: r.get(1),
        precio: r.get(2),
        vigente_desde: r.get(3),
        vigente_hasta: r.get(4),
    }).collect())
}

pub async fn registrar_merma(
    pool: &SqlitePool,
    input: StockMermaInput,
) -> Result<(), AppError> {
    let cantidad_delta = -input.cantidad;

    let mut tx = pool.begin().await?;

    let costo_unitario: i64 = sqlx::query_scalar(
        r#"
        SELECT costo_actual
          FROM producto
         WHERE id_producto = ?
        "#,
    )
    .bind(input.id_producto)
    .fetch_one(&mut *tx)
    .await?;

    let total_costo = costo_unitario * input.cantidad;

    sqlx::query(
        r#"
        INSERT INTO stock_mov (
            id_producto,
            cantidad_delta,
            motivo,
            referencia,
            costo_unitario,
            total_costo,
            fecha_hora
        )
        VALUES (?, ?, ?, ?, ?, ?, DATETIME('now','localtime'))
        "#,
    )
    .bind(input.id_producto)
    .bind(cantidad_delta)
    .bind(&input.motivo)
    .bind(&input.observacion)
    .bind(costo_unitario)
    .bind(total_costo)
    .execute(&mut *tx)
    .await?;

    let descripcion = format!(
        "Merma: {} (prod #{}) x{}",
        input.motivo, input.id_producto, input.cantidad
    );

    sqlx::query(
        r#"
        INSERT INTO gasto_rentabilidad (
          fecha, categoria, tipo, origen, descripcion, monto, referencia
        )
        VALUES (
          DATE('now','localtime'),
          'otros',
          'egreso',
          'ajuste',
          ?,
          ?,
          ?
        )
        "#,
    )
    .bind(&descripcion)
    .bind(total_costo)
    .bind(&input.observacion)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO gasto_negocio (
          fecha_hora, categoria, descripcion, monto, id_usuario
        )
        VALUES (
          DATETIME('now','localtime'),
          'otros',
          ?,
          ?,
          ?
        )
        "#,
    )
    .bind(&descripcion)
    .bind(total_costo)
    .bind(input.id_usuario)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}

pub async fn stock_compra_tx(
    tx: &mut Transaction<'_, Sqlite>,
    id_producto: i64,
    cantidad_maples: i64,
    costo_unitario: i64,
    costo_total: i64,
    referencia: String,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO stock_mov
          (id_producto, cantidad_delta, motivo, referencia, costo_unitario, total_costo)
        VALUES
          (?, ?, 'compra', ?, ?, ?)
        "#,
    )
    .bind(id_producto)
    .bind(cantidad_maples)
    .bind(referencia)
    .bind(costo_unitario)
    .bind(costo_total)
    .execute(&mut **tx)
    .await?;

    sqlx::query(
        r#"
        UPDATE producto
        SET costo_actual = ?
        WHERE id_producto = ?
        "#,
    )
    .bind(costo_unitario)
    .bind(id_producto)
    .execute(&mut **tx)
    .await?;

    sqlx::query(
        r#"
        UPDATE precio_historial
        SET vigente_hasta = CURRENT_TIMESTAMP
        WHERE id_producto = ? AND tipo = 'costo' AND vigente_hasta IS NULL
        "#,
    )
    .bind(id_producto)
    .execute(&mut **tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO precio_historial (id_producto, tipo, precio)
        VALUES (?, 'costo', ?)
        "#,
    )
    .bind(id_producto)
    .bind(costo_unitario)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

pub struct StockReposicionRowRepo {
    pub id_producto: i64,
    pub codigo_producto: String,
    pub nombre: String,
    pub vendidos: i64,
    pub reposicion_modo: String,
    pub reposicion_factor: i64,
}

pub async fn reporte_stock_reposicion_rango(
    pool: &SqlitePool,
    desde: &str,
    hasta: &str,
    solo_activos: bool,
) -> Result<Vec<StockReposicionRowRepo>, AppError> {
    if desde.trim().is_empty() || hasta.trim().is_empty() {
        return Err(AppError::Negocio("Las fechas desde/hasta son requeridas".into()));
    }

    let rows = sqlx::query(
        r#"
        SELECT
            p.id_producto,
            p.codigo_producto,
            p.nombre,
            COALESCE(SUM(CASE WHEN v.id_venta IS NOT NULL THEN vi.cantidad ELSE 0 END), 0) AS vendidos,
            COALESCE(p.reposicion_modo, 'unitario') AS reposicion_modo,
            COALESCE(p.reposicion_factor, 12)       AS reposicion_factor
        FROM producto p
        LEFT JOIN venta_item vi
            ON vi.id_producto = p.id_producto
        LEFT JOIN venta v
            ON v.id_venta = vi.id_venta
           AND v.estado = 'finalizada'
           AND v.fecha_hora >= DATETIME(?1 || ' 00:00:00')
           AND v.fecha_hora <  DATETIME(DATE(?2, '+1 day') || ' 00:00:00')
        WHERE (?3 = 0 OR p.activo = 1)
        GROUP BY
            p.id_producto, p.codigo_producto, p.nombre, p.reposicion_modo, p.reposicion_factor
        ORDER BY vendidos DESC, p.nombre ASC
        "#
    )
    .bind(desde)
    .bind(hasta)
    .bind(if solo_activos { 1 } else { 0 })
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| StockReposicionRowRepo {
            id_producto: r.get(0),
            codigo_producto: r.get(1),
            nombre: r.get(2),
            vendidos: r.get(3),
            reposicion_modo: r.get(4),
            reposicion_factor: r.get(5),
        })
        .collect())
}

pub async fn producto_actualizar_reposicion(
    pool: &SqlitePool,
    id_producto: i64,
    reposicion_modo: &str,
    reposicion_factor: i64,
) -> Result<(), AppError> {
    if id_producto <= 0 {
        return Err(AppError::Negocio("id_producto inválido".into()));
    }

    let (modo, factor) = logica::validar_reposicion(reposicion_modo, reposicion_factor)
        .map_err(AppError::Negocio)?;

    sqlx::query(
        r#"
        UPDATE producto
        SET reposicion_modo = ?1,
            reposicion_factor = ?2
        WHERE id_producto = ?3
        "#
    )
    .bind(modo)
    .bind(factor)
    .bind(id_producto)
    .execute(pool)
    .await?;

    Ok(())
}