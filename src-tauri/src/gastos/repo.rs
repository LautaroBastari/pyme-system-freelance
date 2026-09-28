use sqlx::SqlitePool;
use crate::error::AppError;

use super::model::{
    GastoListarPeriodoInput, GastoNegocioRow, SueldoPagoRow, SueldoPagoRowView,
};

// ----------------------------------------------------------
// Helpers internos
// ----------------------------------------------------------

fn norm_str(s: &str) -> String {
    s.trim().to_string()
}

fn norm_opt_string(s: Option<String>) -> Option<String> {
    s.map(|x| x.trim().to_string()).filter(|x| !x.is_empty())
}

fn norm_opt_str(s: Option<&str>) -> Option<&str> {
    s.map(|x| x.trim()).filter(|x| !x.is_empty())
}

// INSERTS
pub async fn sueldo_insert(
    pool: &SqlitePool,
    id_usuario: i64,
    fecha_hora: Option<String>,
    descripcion: &str,
    monto: i64,
    id_usuario_destino: Option<i64>,
) -> Result<i64, AppError> {
    if let Some(uid_dest) = id_usuario_destino {
        let ok: Option<i64> = sqlx::query_scalar(
            "SELECT id_usuario FROM usuario WHERE id_usuario = ?1 AND activo = 1",
        )
        .bind(uid_dest)
        .fetch_optional(pool)
        .await?;

        if ok.is_none() {
            return Err(AppError::Negocio("El usuario destino no existe o está inactivo".into()));
        }
    }

    let desc = norm_str(descripcion);
    let fh = norm_opt_string(fecha_hora);

    let id = if let Some(fh) = fh {
        sqlx::query_scalar::<_, i64>(
            r#"
            INSERT INTO sueldo_pago (fecha_hora, descripcion, monto, id_usuario_destino, id_usuario)
            VALUES (?1, ?2, ?3, ?4, ?5)
            RETURNING id_sueldo_pago
            "#,
        )
        .bind(fh)
        .bind(desc)
        .bind(monto)
        .bind(id_usuario_destino)
        .bind(id_usuario)
        .fetch_one(pool)
        .await?
    } else {
        sqlx::query_scalar::<_, i64>(
            r#"
            INSERT INTO sueldo_pago (descripcion, monto, id_usuario_destino, id_usuario)
            VALUES (?1, ?2, ?3, ?4)
            RETURNING id_sueldo_pago
            "#,
        )
        .bind(desc)
        .bind(monto)
        .bind(id_usuario_destino)
        .bind(id_usuario)
        .fetch_one(pool)
        .await?
    };

    Ok(id)
}

pub async fn gasto_insert(
    pool: &SqlitePool,
    id_usuario: i64,
    fecha_hora: Option<String>,
    categoria: &str,
    descripcion: Option<String>,
    monto: i64,
) -> Result<i64, AppError> {
    let cat = norm_str(categoria);
    let desc = norm_opt_string(descripcion);
    let fh = norm_opt_string(fecha_hora);

    let id = if let Some(fh) = fh {
        sqlx::query_scalar::<_, i64>(
            r#"
            INSERT INTO gasto_negocio (fecha_hora, categoria, descripcion, monto, id_usuario)
            VALUES (?1, ?2, ?3, ?4, ?5)
            RETURNING id_gasto_negocio
            "#,
        )
        .bind(fh)
        .bind(cat)
        .bind(desc)
        .bind(monto)
        .bind(id_usuario)
        .fetch_one(pool)
        .await?
    } else {
        sqlx::query_scalar::<_, i64>(
            r#"
            INSERT INTO gasto_negocio (categoria, descripcion, monto, id_usuario)
            VALUES (?1, ?2, ?3, ?4)
            RETURNING id_gasto_negocio
            "#,
        )
        .bind(cat)
        .bind(desc)
        .bind(monto)
        .bind(id_usuario)
        .fetch_one(pool)
        .await?
    };

    Ok(id)
}

// LISTADOS

pub async fn gasto_listar_por_periodo(
    pool: &SqlitePool,
    filtro: GastoListarPeriodoInput,
) -> Result<Vec<GastoNegocioRow>, AppError> {
    let cat = norm_opt_string(filtro.categoria);

    let rows = if let Some(cat) = cat {
        sqlx::query_as::<_, GastoNegocioRow>(
            r#"
            SELECT id_gasto_negocio, fecha_hora, categoria, descripcion, monto, id_usuario
            FROM gasto_negocio
            WHERE date(fecha_hora) BETWEEN ?1 AND ?2
              AND categoria = ?3
            ORDER BY fecha_hora ASC, id_gasto_negocio ASC
            "#,
        )
        .bind(&filtro.fecha_desde)
        .bind(&filtro.fecha_hasta)
        .bind(cat)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, GastoNegocioRow>(
            r#"
            SELECT id_gasto_negocio, fecha_hora, categoria, descripcion, monto, id_usuario
            FROM gasto_negocio
            WHERE date(fecha_hora) BETWEEN ?1 AND ?2
            ORDER BY fecha_hora ASC, id_gasto_negocio ASC
            "#,
        )
        .bind(&filtro.fecha_desde)
        .bind(&filtro.fecha_hasta)
        .fetch_all(pool)
        .await?
    };

    Ok(rows)
}

pub async fn sueldo_listar_por_periodo(
    pool: &SqlitePool,
    fecha_desde: &str,
    fecha_hasta: &str,
    id_usuario_destino: Option<i64>,
) -> Result<Vec<SueldoPagoRow>, AppError> {
    let rows = if let Some(uid) = id_usuario_destino {
        sqlx::query_as::<_, SueldoPagoRow>(
            r#"
            SELECT id_sueldo_pago, fecha_hora, descripcion, monto, id_usuario_destino, id_usuario
            FROM sueldo_pago
            WHERE date(fecha_hora) BETWEEN ?1 AND ?2
              AND id_usuario_destino = ?3
            ORDER BY fecha_hora ASC, id_sueldo_pago ASC
            "#,
        )
        .bind(fecha_desde)
        .bind(fecha_hasta)
        .bind(uid)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, SueldoPagoRow>(
            r#"
            SELECT id_sueldo_pago, fecha_hora, descripcion, monto, id_usuario_destino, id_usuario
            FROM sueldo_pago
            WHERE date(fecha_hora) BETWEEN ?1 AND ?2
            ORDER BY fecha_hora ASC, id_sueldo_pago ASC
            "#,
        )
        .bind(fecha_desde)
        .bind(fecha_hasta)
        .fetch_all(pool)
        .await?
    };

    Ok(rows)
}

pub async fn sueldo_listar_por_periodo_view(
    pool: &SqlitePool,
    fecha_desde: &str,
    fecha_hasta: &str,
    id_usuario_destino: Option<i64>,
) -> Result<Vec<SueldoPagoRowView>, AppError> {
    let rows = if let Some(uid) = id_usuario_destino {
        sqlx::query_as::<_, SueldoPagoRowView>(
            r#"
            SELECT sp.id_sueldo_pago,
                   sp.fecha_hora,
                   sp.descripcion,
                   sp.monto,
                   sp.id_usuario_destino,
                   u.nombre AS usuario_destino_nombre
            FROM sueldo_pago sp
            JOIN usuario u ON u.id_usuario = sp.id_usuario_destino
            WHERE date(sp.fecha_hora) BETWEEN ?1 AND ?2
              AND sp.id_usuario_destino = ?3
            ORDER BY sp.fecha_hora ASC, sp.id_sueldo_pago ASC
            "#,
        )
        .bind(fecha_desde)
        .bind(fecha_hasta)
        .bind(uid)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, SueldoPagoRowView>(
            r#"
            SELECT sp.id_sueldo_pago,
                   sp.fecha_hora,
                   sp.descripcion,
                   sp.monto,
                   sp.id_usuario_destino,
                   u.nombre AS usuario_destino_nombre
            FROM sueldo_pago sp
            JOIN usuario u ON u.id_usuario = sp.id_usuario_destino
            WHERE date(sp.fecha_hora) BETWEEN ?1 AND ?2
            ORDER BY sp.fecha_hora ASC, sp.id_sueldo_pago ASC
            "#,
        )
        .bind(fecha_desde)
        .bind(fecha_hasta)
        .fetch_all(pool)
        .await?
    };

    Ok(rows)
}

// TOTALES
pub async fn gasto_total_por_periodo(
    pool: &SqlitePool,
    fecha_desde: &str,
    fecha_hasta: &str,
    categoria: Option<&str>,
) -> Result<i64, AppError> {
    let cat = norm_opt_str(categoria);

    let total = if let Some(cat) = cat {
        sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COALESCE(SUM(monto), 0)
            FROM gasto_negocio
            WHERE date(fecha_hora) BETWEEN ?1 AND ?2
              AND categoria = ?3
            "#,
        )
        .bind(fecha_desde)
        .bind(fecha_hasta)
        .bind(cat)
        .fetch_one(pool)
        .await?
    } else {
        sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COALESCE(SUM(monto), 0)
            FROM gasto_negocio
            WHERE date(fecha_hora) BETWEEN ?1 AND ?2
            "#,
        )
        .bind(fecha_desde)
        .bind(fecha_hasta)
        .fetch_one(pool)
        .await?
    };

    Ok(total)
}

pub async fn sueldo_total_por_periodo(
    pool: &SqlitePool,
    fecha_desde: &str,
    fecha_hasta: &str,
    id_usuario_destino: Option<i64>,
) -> Result<i64, AppError> {
    let total = if let Some(uid) = id_usuario_destino {
        sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COALESCE(SUM(monto), 0)
            FROM sueldo_pago
            WHERE date(fecha_hora) BETWEEN ?1 AND ?2
              AND id_usuario_destino = ?3
            "#,
        )
        .bind(fecha_desde)
        .bind(fecha_hasta)
        .bind(uid)
        .fetch_one(pool)
        .await?
    } else {
        sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COALESCE(SUM(monto), 0)
            FROM sueldo_pago
            WHERE date(fecha_hora) BETWEEN ?1 AND ?2
            "#,
        )
        .bind(fecha_desde)
        .bind(fecha_hasta)
        .fetch_one(pool)
        .await?
    };

    Ok(total)
}