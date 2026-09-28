use tauri::State;
use serde::{Deserialize, Serialize};
use crate::AppState;
use crate::error::AppError;
use super::{repo, logica, model::PromoComboAplicarInput};

#[derive(serde::Serialize)]
pub struct ProductoDisponible {
    pub id_producto: i64,
    pub nombre: String,
    pub precio_unitario: i64,
    pub stock_disponible: i64,
}

#[derive(serde::Serialize)]
pub struct VentaItemDto {
    pub id_item: i64,
    pub id_producto: i64,
    pub nombre: String,
    pub cantidad: i64,
    pub precio_unitario: i64,
    pub subtotal: i64,
}

#[derive(serde::Deserialize)]
pub struct AgregarItemInput {
    pub id_venta: i64,
    pub id_producto: i64,
    pub cantidad: i64,
}

#[derive(serde::Deserialize)]
pub struct SetCantidadInput {
    pub id_item: i64,
    pub cantidad: i64,
}

#[derive(serde::Deserialize)]
pub struct QuitarItemInput {
    pub id_item: i64,
}

#[derive(serde::Deserialize)]
pub struct VentaListarInput {
    pub id_venta: i64,
}

#[derive(serde::Deserialize)]
pub struct VentaCancelarInput {
    pub id_venta: i64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PagoInput {
    pub medio: String,
    pub monto: i64,
    pub referencia: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct VentaFinalizarInput {
    pub id_venta: i64,
    pub pagos: Vec<PagoInput>,
}

#[derive(serde::Serialize)]
pub struct HistorialItem {
    pub id_venta: i64,
    pub hora: String,
    pub codigo_producto: String,
    pub producto: String,
    pub cantidad: i64,
    pub precio_unitario: i64,
    pub subtotal: i64,
    pub total_venta: i64,
    pub pagos_detalle: Option<String>,
}

#[tauri::command]
pub async fn productos_disponibles(
    state: State<'_, AppState>,
) -> Result<Vec<ProductoDisponible>, AppError> {
    let rows = repo::obtener_productos_disponibles(&state.pool).await?;
    Ok(rows)
}

#[tauri::command]
pub async fn venta_iniciar(state: State<'_, AppState>) -> Result<i64, AppError> {
    let uid = *state
        .session_user
        .lock()
        .map_err(|_| AppError::Sistema("Error bloqueando sesión".into()))?
        .as_ref()
        .ok_or_else(|| AppError::Negocio("No hay sesión iniciada".into()))?;

    let id_venta = repo::iniciar_venta_repo(&state.pool, uid).await?;
    Ok(id_venta)
}

#[tauri::command]
pub async fn venta_agregar_item(
    state: State<'_, AppState>,
    input: AgregarItemInput,
) -> Result<(), AppError> {
    logica::validar_cantidad_item(input.cantidad)?;
    repo::agregar_item_repo(&state.pool, input).await?;
    Ok(())
}

#[tauri::command]
pub async fn venta_listar(
    state: State<'_, AppState>,
    input: VentaListarInput,
) -> Result<(Vec<VentaItemDto>, i64), AppError> {
    let res = repo::listar_items_venta(&state.pool, input.id_venta).await?;
    Ok(res)
}

#[tauri::command]
pub async fn venta_set_cantidad(
    state: State<'_, AppState>,
    input: SetCantidadInput,
) -> Result<(), AppError> {
    logica::validar_cantidad_item(input.cantidad)?;
    repo::set_cantidad_item(&state.pool, input).await?;
    Ok(())
}

#[tauri::command]
pub async fn venta_quitar_item(
    state: State<'_, AppState>,
    input: QuitarItemInput,
) -> Result<(), AppError> {
    repo::quitar_item(&state.pool, input).await?;
    Ok(())
}

#[tauri::command]
pub async fn venta_cancelar(
    state: State<'_, AppState>,
    input: VentaCancelarInput,
) -> Result<(), AppError> {
    repo::cancelar_venta(&state.pool, input.id_venta).await?;
    Ok(())
}

#[tauri::command]
pub async fn venta_finalizar(
    state: State<'_, AppState>,
    input: VentaFinalizarInput,
) -> Result<(), AppError> {
    if input.pagos.is_empty() {
        return Err(AppError::Negocio("Debe registrar al menos un método de pago".into()));
    }
    let suma_pagos: i64 = input.pagos.iter().map(|p| p.monto).sum();
    repo::finalizar_venta_repo(&state.pool, input, suma_pagos).await?;
    Ok(())
}

#[tauri::command]
pub async fn historial_ventas_hoy(
    state: State<'_, AppState>
) -> Result<Vec<HistorialItem>, AppError> {
    let uid = *state
        .session_user
        .lock()
        .map_err(|_| AppError::Sistema("Error en lock".into()))?
        .as_ref()
        .ok_or_else(|| AppError::Negocio("No hay sesión".into()))?;

    let items = repo::obtener_historial_hoy(&state.pool, uid).await?;
    Ok(items)
}

#[tauri::command(rename = "venta_aplicar_promo_combo")]
pub async fn venta_aplicar_promo_combo(
    state: State<'_, AppState>,
    input: PromoComboAplicarInput,
) -> Result<(), AppError> {
    repo::aplicar_promo_combo_repo(&state.pool, input).await?;
    Ok(())
}