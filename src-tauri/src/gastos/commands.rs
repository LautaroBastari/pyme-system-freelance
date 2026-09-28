use tauri::State;

use crate::app_state::AppState;
use crate::users::auth::AuthState;

use super::logica;
use super::model::{SueldoRegistrarInput, GastoRegistrarInput, GastoListarPeriodoInput, GastoNegocioRow};
use super::repo;
use super::model::{TotalesPeriodoInput, TotalOut};
use super::model::{SueldoListarPeriodoInput, SueldoPagoRow};
use super::model::SueldoPagoRowView;

fn read_uid(auth: &AuthState, state: &AppState) -> Option<i64> {
    // Intentar desde AuthState
    if let Ok(g) = auth.current_user_id.read() {
        if g.is_some() { return *g; }
    }
    // Fallback al viejo session_user
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
) -> Result<i64, String> {
    let uid = read_uid(&auth, &state)
        .ok_or_else(|| "Tenés que iniciar sesión para registrar un sueldo".to_string())?;

    let desc = logica::normalizar_texto_requerido(&input.descripcion, "descripcion")?;
    logica::validar_monto_positivo(input.monto)?;
    let id_dest = logica::requerir_usuario_destino(input.id_usuario_destino)?;

    repo::sueldo_insert(
        &state.pool,
        uid,
        input.fecha_hora,
        &desc,
        input.monto,
        Some(id_dest),
    )
    .await
    .map_err(|e| e.to_string())
}

// GASTOS

#[tauri::command(rename = "gasto_registrar")]
pub async fn gasto_registrar(
    state: State<'_, AppState>,
    auth: State<'_, AuthState>,
    input: GastoRegistrarInput,
) -> Result<i64, String> {

    let uid = read_uid(&auth, &state)
        .ok_or_else(|| "Tenés que iniciar sesión para registrar un gasto".to_string())?;

    let cat = logica::normalizar_texto_requerido(&input.categoria, "categoria")?;
    logica::validar_monto_positivo(input.monto)?;
    let desc = logica::normalizar_texto_opcional(input.descripcion);

    repo::gasto_insert(
        &state.pool,
        uid,
        input.fecha_hora,
        &cat,
        desc,
        input.monto,
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command(rename = "gasto_listar_por_periodo")]
pub async fn gasto_listar_por_periodo(
    state: State<'_, AppState>,
    auth: State<'_, AuthState>,
    filtro: GastoListarPeriodoInput,
) -> Result<Vec<GastoNegocioRow>, String> {

    let _uid = read_uid(&auth, &state)
        .ok_or_else(|| "Tenés que iniciar sesión.".to_string())?;


    repo::gasto_listar_por_periodo(&state.pool, filtro)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command(rename = "gasto_total_por_periodo")]
pub async fn gasto_total_por_periodo(
    state: State<'_, AppState>,
    auth: State<'_, AuthState>,
    input: TotalesPeriodoInput,
) -> Result<TotalOut, String> {

    let _uid = read_uid(&auth, &state)
        .ok_or_else(|| "Tenés que iniciar sesión.".to_string())?;

    let cat = input.categoria.as_ref().map(|s| s.trim().to_string()).filter(|s| !s.is_empty());

    let total = repo::gasto_total_por_periodo(
        &state.pool,
        &input.fecha_desde,
        &input.fecha_hasta,
        cat.as_deref(),
    )
    .await
    .map_err(|e| e.to_string())?;

    Ok(TotalOut { total })
}

#[tauri::command(rename = "sueldo_total_por_periodo")]
pub async fn sueldo_total_por_periodo(
    state: State<'_, AppState>,
    auth: State<'_, AuthState>,
    input: TotalesPeriodoInput,
) -> Result<TotalOut, String> {

    let _uid = read_uid(&auth, &state)
        .ok_or_else(|| "Tenés que iniciar sesión.".to_string())?;

    let total = repo::sueldo_total_por_periodo(
        &state.pool,
        &input.fecha_desde,
        &input.fecha_hasta,
        input.id_usuario_destino,
    )
    .await
    .map_err(|e| e.to_string())?;

    Ok(TotalOut { total })
}


#[tauri::command(rename = "sueldo_listar_por_periodo")]
pub async fn sueldo_listar_por_periodo(
    state: State<'_, AppState>,
    auth: State<'_, AuthState>,
    input: SueldoListarPeriodoInput,
) -> Result<Vec<SueldoPagoRowView>, String> {
    let _uid = read_uid(&auth, &state)
        .ok_or_else(|| "Tenés que iniciar sesión.".to_string())?;

    repo::sueldo_listar_por_periodo_view(
        &state.pool,
        &input.fecha_desde,
        &input.fecha_hasta,
        input.id_usuario_destino,
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command(rename = "gastos_ping")]
pub async fn gastos_ping() -> &'static str { "pong" }