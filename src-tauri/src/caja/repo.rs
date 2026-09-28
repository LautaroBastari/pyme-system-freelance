use sqlx::{SqlitePool, Row};
use crate::caja::model::{Caja, EstadoCaja};
use crate::caja::model::{CajaResumenDiario, MedioPagoResumen};

pub async fn existe_caja_abierta(pool: &SqlitePool) -> Result<bool, sqlx::Error> {
    let ok: Option<i64> = sqlx::query_scalar(
        "SELECT 1 FROM caja WHERE estado='abierta' LIMIT 1"
    ).fetch_optional(pool).await?;
    Ok(ok.is_some())
}

pub async fn abrir_caja(pool: &SqlitePool, user_id: i64) -> Result<i64, sqlx::Error> {
    let res = sqlx::query(
        "INSERT INTO caja (abierta_por, estado, abierta_en)
            VALUES (?1,'abierta',DATETIME('now','localtime'));"
    )
    .bind(user_id)
    .execute(pool).await?;
    Ok(res.last_insert_rowid())
}

pub async fn ultima_caja_abierta_id(pool: &SqlitePool) -> Result<Option<i64>, sqlx::Error> {
    let id: Option<i64> = sqlx::query_scalar(
        "SELECT id_caja FROM caja
         WHERE estado='abierta'
         ORDER BY id_caja DESC LIMIT 1"
    ).fetch_optional(pool).await?;
    Ok(id)
}

pub async fn cerrar_caja(pool: &SqlitePool, id_caja: i64, _user_id: i64) -> Result<(), sqlx::Error> {
    let hay_venta_en_curso: i64 = sqlx::query_scalar(
        "SELECT EXISTS(
            SELECT 1
            FROM venta
            WHERE estado = 'en_curso'
            LIMIT 1
        );"
    )
    .fetch_one(pool)
    .await?;

    if hay_venta_en_curso == 1 {
        return Err(sqlx::Error::Protocol(
            "No se puede cerrar caja: hay una venta en curso.".into()
        ));
    }

    let res = sqlx::query(
        "UPDATE caja
            SET estado='cerrada',
                cerrada_en=DATETIME('now','localtime')
          WHERE id_caja=?1 AND estado='abierta';"
    )
    .bind(id_caja)
    .execute(pool)
    .await?;

    if res.rows_affected() == 0 {
        return Err(sqlx::Error::Protocol(
            "No se cerró la caja (no existe o ya estaba cerrada).".into()
        ));
    }

    Ok(())
}

pub async fn obtener_caja(pool: &SqlitePool, id_caja: i64) -> Result<Option<Caja>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT id_caja, abierta_por, abierta_en, estado, cerrada_en
           FROM caja
          WHERE id_caja=?1"
    )
    .bind(id_caja)
    .fetch_optional(pool).await?;

    Ok(row.map(|r| Caja{
        id_caja: r.get("id_caja"),
        abierta_por: r.get("abierta_por"),
        abierta_en: r.get::<String,_>("abierta_en"),
        estado: match r.get::<String,_>("estado").as_str() {
            "abierta" => EstadoCaja::Abierta,
            _ => EstadoCaja::Cerrada,
        },
        cerrada_por: None,
        cerrada_en: r.try_get("cerrada_en").ok(),
    }))
}

pub async fn obtener_resumen_diario(pool: &SqlitePool, uid: i64) -> Result<CajaResumenDiario, sqlx::Error> {
    // cajas del día
    let cajas = sqlx::query(
        r#"
        SELECT id_caja
        FROM caja
        WHERE abierta_por = ?
          AND abierta_en >= datetime('now','localtime','start of day')
          AND abierta_en <  datetime('now','localtime','start of day','+1 day')
        "#)
        .bind(uid)
        .fetch_all(pool)
        .await?;

    let id_cajas: Vec<i64> = cajas.iter().map(|r| r.get::<i64,_>("id_caja")).collect();

    // total ventas finalizadas del día
    let ventas_count: i32 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM venta
        WHERE id_usuario = ?
          AND estado='finalizada'
          AND fecha_hora >= datetime('now','localtime','start of day')
          AND fecha_hora <  datetime('now','localtime','start of day','+1 day')
        "#)
        .bind(uid)
        .fetch_one(pool)
        .await?;

    let total_general: i64 = sqlx::query_scalar(
        r#"
        SELECT COALESCE(SUM(vp.monto),0)
        FROM venta v
        JOIN venta_pago vp ON vp.id_venta = v.id_venta
        WHERE v.id_usuario = ?
          AND v.estado='finalizada'
          AND v.fecha_hora >= datetime('now','localtime','start of day')
          AND v.fecha_hora <  datetime('now','localtime','start of day','+1 day')
        "#)
        .bind(uid)
        .fetch_one(pool)
        .await?;

    let pagos = sqlx::query(
        r#"
        SELECT vp.medio AS medio,
               SUM(vp.monto) AS total_medio
        FROM venta v
        JOIN venta_pago vp ON vp.id_venta = v.id_venta
        WHERE v.id_usuario = ?
          AND v.estado='finalizada'
          AND v.fecha_hora >= datetime('now','localtime','start of day')
          AND v.fecha_hora <  datetime('now','localtime','start of day','+1 day')
        GROUP BY vp.medio
        "#)
        .bind(uid)
        .fetch_all(pool)
        .await?;

    let por_medio = pagos.into_iter().map(|r| MedioPagoResumen {
        medio: r.get("medio"),
        total_medio: r.get("total_medio"),
    }).collect();

    let cantidad_cajas = id_cajas.len() as i32;

    Ok(CajaResumenDiario {
        id_cajas,
        cantidad_cajas,
        cantidad_ventas: ventas_count,
        total_general,
        por_medio,
    })
}