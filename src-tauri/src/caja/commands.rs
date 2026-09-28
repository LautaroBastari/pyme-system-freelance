use tauri::State;
use serde::{Deserialize, Serialize};
use crate::app_state::AppState;
use crate::users::auth::AuthState;
use crate::caja::repo;
// AGREGAMOS EL IMPORT DE NUESTRO ERROR
use crate::error::AppError;

use super::model::{CajaResumenDiario, MedioPagoResumen};

// UTILIDADES

#[tauri::command]
pub async fn ping_inline() -> &'static str { "pong" }

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

// CONSULTAS BÁSICAS

#[tauri::command]
pub async fn caja_esta_abierta(state: State<'_, AppState>) -> Result<bool, AppError> {
    // El '?' captura automáticamente cualquier sqlx::Error y lo convierte en AppError::Database
    let esta_abierta = repo::existe_caja_abierta(&state.pool).await?;
    Ok(esta_abierta)
}

#[tauri::command]
pub async fn caja_estado(state: State<'_, AppState>) -> Result<bool, AppError> {
    let id = repo::ultima_caja_abierta_id(&state.pool).await?;
    Ok(id.is_some())
}

// ABRIR CAJA

#[derive(Serialize)]
pub struct CajaAbrirOut { pub id_caja: i64 }

#[tauri::command]
pub async fn caja_abrir(
    state: State<'_, AppState>,
    auth: State<'_, AuthState>
) -> Result<CajaAbrirOut, AppError> {

    // Lanzamos error de negocio si no hay sesión
    let uid = read_uid(&auth, &state)
        .ok_or_else(|| AppError::Negocio("Tenés que iniciar sesión para abrir caja".to_string()))?;

    // Si ya hay caja abierta, la cerramos automáticamente
    if repo::existe_caja_abierta(&state.pool).await? {
        if let Some(id) = repo::ultima_caja_abierta_id(&state.pool).await? {
            repo::cerrar_caja(&state.pool, id, uid).await?;
        }
    }

    let id_nueva = repo::abrir_caja(&state.pool, uid).await?;

    Ok(CajaAbrirOut { id_caja: id_nueva })
}

// CERRAR CAJA INDIVIDUAL

#[derive(Serialize)]
pub struct CajaCerrarOut { pub id_caja: i64 }

#[tauri::command]
pub async fn caja_cerrar(
    state: State<'_, AppState>,
    auth: State<'_, AuthState>
) -> Result<CajaCerrarOut, AppError> {

    let uid = read_uid(&auth, &state)
        .ok_or_else(|| AppError::Negocio("Tenés que iniciar sesión para cerrar la caja".to_string()))?;

    let id = repo::ultima_caja_abierta_id(&state.pool)
        .await?
        .ok_or_else(|| AppError::Negocio("No hay caja abierta para cerrar".to_string()))?;

    repo::cerrar_caja(&state.pool, id, uid).await?;

    Ok(CajaCerrarOut { id_caja: id })
}

// RESUMEN DIARIO DEL USUARIO

#[tauri::command]
pub async fn caja_resumen_diario(
    state: State<'_, AppState>,
    auth: State<'_, AuthState>
) -> Result<CajaResumenDiario, AppError> {

    let uid = read_uid(&auth, &state)
        .ok_or_else(|| AppError::Negocio("Tenés que iniciar sesión.".to_string()))?;

    let resumen = repo::obtener_resumen_diario(&state.pool, uid).await?;

    Ok(resumen)
}

// CERRAR TODAS LAS CAJAS DEL DÍA DEL USUARIO

#[derive(Deserialize)]
pub struct CierreDiarioInput {
    pub id_cajas: Vec<i64>,
}

#[tauri::command]
pub async fn caja_cerrar_diario(
    state: State<'_, AppState>,
    auth: State<'_, AuthState>,
    input: CierreDiarioInput
) -> Result<(), AppError> {

    let uid = read_uid(&auth, &state)
        .ok_or_else(|| AppError::Negocio("Tenés que iniciar sesión.".to_string()))?;

    for id in input.id_cajas {
        repo::cerrar_caja(&state.pool, id, uid).await?;
    }

    Ok(())
}

// LOGOUT
#[tauri::command]
pub async fn auth_logout(auth: State<'_, AuthState>) -> Result<(), AppError> {
    *auth.current_user_id.write().map_err(|_| AppError::Sistema("Error liberando sesión".to_string()))? = None;
    Ok(())
}