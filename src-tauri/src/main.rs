#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use tauri::Manager;
use std::sync::{Arc, Mutex};
use tracing_subscriber::fmt::writer::MakeWriterExt;
// módulos externos
mod app_state;
mod db;
mod users { pub mod model; pub mod crypto; pub mod repo; pub mod commands; pub mod auth; }
mod ventas;
mod caja;
mod stock;
mod reportes;
mod compras;
mod gastos;
mod promos;
mod ventas_admin;
mod PNL;
mod home;
mod error;
// === Imports de estructuras expuestas ===
use app_state::AppState;
use users::auth::AuthState;

fn main() {

    fn setup_logging() {
    // Crea un archivo nuevo por día: "pyme-system.2026-09-28.log"
    let file_appender = tracing_appender::rolling::daily("logs", "pyme-system.log");
    
    tracing_subscriber::fmt()
        .with_writer(file_appender.and(std::io::stdout)) // Escribe en archivo y consola
        .with_env_filter("info,sqlx=warn") // Oculta logs ruidosos de sqlx, muestra infos tuyos
        .init();
}
    setup_logging();
    tauri::Builder::default()
        .setup(|app| {
            use std::sync::{Arc, Mutex};
            let handle = app.handle();

            let pool_res = tauri::async_runtime::block_on(db::init_db(&handle));

            match pool_res {
                Ok(pool) => {
                    app.manage(AppState {
                        pool,
                        session_user: Arc::new(Mutex::new(None)),
                    });
                    Ok(())
                }
                Err(e) => {
                    // Mostrar/registrar el error real
                    eprintln!("[INIT_DB] {e}");

                    Err(e.to_string().into())
                }
            }
        })
        // Registrar AuthState global (login)
        .manage(AuthState::default())
        // Registrar comandos
        .invoke_handler(tauri::generate_handler![
            // === HOME ADMIN ===
            home::commands::admin_home_resumen,
            // === USUARIOS ===
            users::commands::usuario_crear,
            users::commands::usuario_obtener,
            users::commands::usuario_listar,
            users::commands::login,
            users::commands::session_info,
            users::commands::usuario_actual,
            users::commands::usuario_listar_opciones,
            // === VENTAS ===
            ventas::commands::productos_disponibles,
            ventas::commands::venta_iniciar,
            ventas::commands::venta_agregar_item,
            ventas::commands::venta_listar,
            ventas::commands::venta_set_cantidad,
            ventas::commands::venta_quitar_item,
            ventas::commands::venta_cancelar,
            ventas::commands::venta_finalizar,
            ventas::commands::historial_ventas_hoy,
            ventas::commands::venta_aplicar_promo_combo,
            // === CAJA ===
            caja::commands::caja_esta_abierta,
            caja::commands::caja_abrir,
            caja::commands::caja_cerrar,
            caja::commands::auth_logout,
            caja::commands::ping_inline,
            caja::commands::caja_estado,
            caja::commands::caja_resumen_diario,
            caja::commands::caja_cerrar_diario,
            // ==== STOCK ===
            stock::commands::stock_listar,
            stock::commands::producto_crear,
            stock::commands::producto_actualizar,
            stock::commands::producto_set_activo,
            stock::commands::stock_ajustar,
            stock::commands::precio_actualizar,
            stock::commands::stock_mov_listar,
            stock::commands::precio_hist_listar,
            stock::commands::stock_fijar_absoluto,
            stock::commands::reporte_stock_general,
            stock::commands::stock_registrar_merma,
            stock::commands::stock_compra,
            stock::commands::reporte_stock_reposicion,
            stock::commands::producto_actualizar_reposicion,
            // === REPORTES ===
            reportes::commands::admin_historial_dia,
            reportes::rentabilidad::reporte_rentabilidad,
            reportes::rentabilidad::reporte_rentabilidad_negocio,
            // === COMPRAS ===
            compras::commands::registrar_compra,
            // === GASTOS ===
            gastos::commands::sueldo_registrar,
            gastos::commands::gasto_registrar,
            gastos::commands::gasto_listar_por_periodo,
            gastos::commands::gasto_total_por_periodo,
            gastos::commands::sueldo_total_por_periodo,
            gastos::commands::sueldo_listar_por_periodo,
            // === PROMOS ===
            promos::commands::promo_combo_crear,
            promos::commands::promo_combo_listar,
            promos::commands::promo_combo_detalle,
            promos::commands::promo_combo_eliminar,
            // === VENTAS ADMIN ===
            ventas_admin::commands::ventas_admin_listar,
            ventas_admin::commands::venta_admin_detalle,
            ventas_admin::editar::venta_admin_editar_guardar,
            ventas_admin::editar::producto_listar_basico,
            ventas_admin::commands::usuarios_listar_operadores,
            // === PNL ===
            PNL::commands::pnl_reporte,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
