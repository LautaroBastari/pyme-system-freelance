use tauri::State;
use crate::app_state::AppState;
use crate::users::auth::AuthState;
use crate::error::AppError;

use super::logica;
use super::model::{SueldoRegistrarInput, GastoRegistrarInput, GastoListarPeriodoInput, GastoNegocioRow};
use super::repo;
use super::model::{TotalesPeriodoInput, TotalOut};
use super::model::{SueldoListarPeriodoInput, SueldoPagoRow};
use super::model::SueldoPagoRowView;

fn read_uid(auth: &AuthState, state: &AppState) -> Option<i64> {
    if let Ok(g) = auth.current_user_id.read() {
        if g.is_some() { return *g; }
    }
    if let Ok(g) = state.session_user.lock() {
        *g
    } else {
        None
    }
}

// SUELDOS

#[tauri::command(rename = "sueldo_registrar")]
pub async fn sueldo_registrar(
    state: State<'_, AppState>,
    auth: State<'_, AuthState>,
    input: SueldoRegistrarInput,
) -> Result<i64, AppError> {
    let uid = read_uid(&auth, &state)
        .ok_or_else(|| AppError::Negocio("Tenés que iniciar sesión para registrar un sueldo".to_string()))?;

    let desc = logica::normalizar_texto_requerido(&input.descripcion, "descripcion")
        .map_err(AppError::Negocio)?;
    logica::validar_monto_positivo(input.monto)
        .map_err(AppError::Negocio)?;
    let id_dest = logica::requerir_usuario_destino(input.id_usuario_destino)
        .map_err(AppError::Negocio)?;

    let id = repo::sueldo_insert(
        &state.pool,
        uid,
        input.fecha_hora,
        &desc,
        input.monto,
        Some(id_dest),
    ).await?;
    
    Ok(id)
}

// GASTOS

#[tauri::command(rename = "gasto_registrar")]
pub async fn gasto_registrar(
    state: State<'_, AppState>,
    auth: State<'_, AuthState>,
    input: GastoRegistrarInput,
) -> Result<i64, AppError> {
    let uid = read_uid(&auth, &state)
        .ok_or_else(|| AppError::Negocio("Tenés que iniciar sesión para registrar un gasto".to_string()))?;

    let cat = logica::normalizar_texto_requerido(&input.categoria, "categoria")
        .map_err(AppError::Negocio)?;
    logica::validar_monto_positivo(input.monto)
        .map_err(AppError::Negocio)?;
    let desc = logica::normalizar_texto_opcional(input.descripcion);

    let id = repo::gasto_insert(
        &state.pool,
        uid,
        input.fecha_hora,
        &cat,
        desc,
        input.monto,
    ).await?;
    
    Ok(id)
}

#[tauri::command(rename = "gasto_listar_por_periodo")]
pub async fn gasto_listar_por_periodo(
    state: State<'_, AppState>,
    auth: State<'_, AuthState>,
    filtro: GastoListarPeriodoInput,
) -> Result<Vec<GastoNegocioRow>, AppError> {
    let _uid = read_uid(&auth, &state)
        .ok_or_else(|| AppError::Negocio("Tenés que iniciar sesión.".to_string()))?;

    let rows = repo::gasto_listar_por_periodo(&state.pool, filtro).await?;
    Ok(rows)
}

#[tauri::command(rename = "gasto_total_por_periodo")]
pub async fn gasto_total_por_periodo(
    state: State<'_, AppState>,
    auth: State<'_, AuthState>,
    input: TotalesPeriodoInput,
) -> Result<TotalOut, AppError> {
    let _uid = read_uid(&auth, &state)
        .ok_or_else(|| AppError::Negocio("Tenés que iniciar sesión.".to_string()))?;

    let cat = input.categoria.as_ref().map(|s| s.trim().to_string()).filter(|s| !s.is_empty());

    let total = repo::gasto_total_por_periodo(
        &state.pool,
        &input.fecha_desde,
        &input.fecha_hasta,
        cat.as_deref(),
    ).await?;

    Ok(TotalOut { total })
}

#[tauri::command(rename = "sueldo_total_por_periodo")]
pub async fn sueldo_total_por_periodo(
    state: State<'_, AppState>,
    auth: State<'_, AuthState>,
    input: TotalesPeriodoInput,
) -> Result<TotalOut, AppError> {
    let _uid = read_uid(&auth, &state)
        .ok_or_else(|| AppError::Negocio("Tenés que iniciar sesión.".to_string()))?;

    let total = repo::sueldo_total_por_periodo(
        &state.pool,
        &input.fecha_desde,
        &input.fecha_hasta,
        input.id_usuario_destino,
    ).await?;

    Ok(TotalOut { total })
}

#[tauri::command(rename = "sueldo_listar_por_periodo")]
pub async fn sueldo_listar_por_periodo(
    state: State<'_, AppState>,
    auth: State<'_, AuthState>,
    input: SueldoListarPeriodoInput,
) -> Result<Vec<SueldoPagoRowView>, AppError> {
    let _uid = read_uid(&auth, &state)
        .ok_or_else(|| AppError::Negocio("Tenés que iniciar sesión.".to_string()))?;

    let rows = repo::sueldo_listar_por_periodo_view(
        &state.pool,
        &input.fecha_desde,
        &input.fecha_hasta,
        input.id_usuario_destino,
    ).await?;
    
    Ok(rows)
}

#[tauri::command(rename = "gastos_ping")]
pub async fn gastos_ping() -> &'static str { "pong" }