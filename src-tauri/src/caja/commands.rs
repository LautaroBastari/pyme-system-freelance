use tauri::State;
use serde::{Deserialize, Serialize};
use crate::app_state::AppState;
use crate::users::auth::AuthState;
use crate::caja::repo;
use sqlx::Row;

// UTILIDADES

#[tauri::command]
pub async fn ping_inline() -> &'static str { "pong" }

fn read_uid(auth: &AuthState, state: &AppState) -> Option<i64> {
    //  Intentar desde AuthState
    if let Ok(g) = auth.current_user_id.read() {
        if g.is_some() { return *g; }
    }
    if let Ok(g) = state.session_user.lock() {
        *g
    } else {
        None
    }
}

// CONSULTAS BÁSICAS

#[tauri::command]
pub async fn caja_esta_abierta(state: State<'_, AppState>) -> Result<bool, String> {
    repo::existe_caja_abierta(&state.pool)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn caja_estado(state: State<'_, AppState>) -> Result<bool, String> {
    let id = sqlx::query_scalar::<_, i64>(
        "SELECT id_caja FROM caja WHERE estado='abierta' LIMIT 1"
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(id.is_some())
}

// ABRIR CAJA

#[derive(Serialize)]
pub struct CajaAbrirOut { pub id_caja: i64 }

#[tauri::command]
pub async fn caja_abrir(
    state: State<'_, AppState>,
    auth: State<'_, AuthState>
) -> Result<CajaAbrirOut, String> {

    let uid = read_uid(&auth, &state)
        .ok_or_else(|| "Tenés que iniciar sesión para abrir caja".to_string())?;

    // Si ya hay caja abierta, la cerramos automáticamente
    if repo::existe_caja_abierta(&state.pool).await.map_err(|e| e.to_string())? {
        if let Some(id) = repo::ultima_caja_abierta_id(&state.pool)
            .await.map_err(|e| e.to_string())? 
        {
            repo::cerrar_caja(&state.pool, id, uid)
                .await
                .map_err(|e| e.to_string())?;
        }
    }

    let id_nueva = repo::abrir_caja(&state.pool, uid)
        .await
        .map_err(|e| e.to_string())?;

    Ok(CajaAbrirOut { id_caja: id_nueva })
}

// CERRAR CAJA INDIVIDUAL

#[derive(Serialize)]
pub struct CajaCerrarOut { pub id_caja: i64 }

#[tauri::command]
pub async fn caja_cerrar(
    state: State<'_, AppState>,
    auth: State<'_, AuthState>
) -> Result<CajaCerrarOut, String> {

    let uid = read_uid(&auth, &state)
        .ok_or_else(|| "Tenés que iniciar sesión para cerrar la caja".to_string())?;

    let id = repo::ultima_caja_abierta_id(&state.pool)
        .await.map_err(|e| e.to_string())?
        .ok_or_else(|| "No hay caja abierta".to_string())?;

    repo::cerrar_caja(&state.pool, id, uid)
        .await
        .map_err(|e| e.to_string())?;

    Ok(CajaCerrarOut { id_caja: id })
}


//  NUEVO — RESUMEN DIARIO DEL USUARIO


use super::model::{CajaResumenDiario, MedioPagoResumen};

// En src-tauri/src/caja/commands.rs
#[tauri::command]
pub async fn caja_resumen_diario(
    state: State<'_, AppState>,
    auth: State<'_, AuthState>
) -> Result<CajaResumenDiario, String> {

    let uid = read_uid(&auth, &state)
        .ok_or_else(|| "Tenés que iniciar sesión.".to_string())?;

    // Toda la suciedad de SQL ahora vive encapsulada en repo.rs
    let resumen = repo::obtener_resumen_diario(&state.pool, uid)
        .await
        .map_err(|e| e.to_string())?;

    Ok(resumen)
}
// NUEVO — CERRAR TODAS LAS CAJAS DEL DÍA DEL USUARIO

#[derive(Deserialize)]
pub struct CierreDiarioInput {
    pub id_cajas: Vec<i64>,
}

#[tauri::command]
pub async fn caja_cerrar_diario(
    state: State<'_, AppState>,
    auth: State<'_, AuthState>,
    input: CierreDiarioInput
) -> Result<(), String> {

    let uid = read_uid(&auth, &state)
        .ok_or_else(|| "Tenés que iniciar sesión.".to_string())?;

    for id in input.id_cajas {
        repo::cerrar_caja(&state.pool, id, uid)
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

// LOGOUT
#[tauri::command]
pub async fn auth_logout(auth: State<'_, AuthState>) -> Result<(), String> {
    *auth.current_user_id.write().map_err(|_| "lock")? = None;
    Ok(())
}
