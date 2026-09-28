use serde::Serialize;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EstadoCaja { Abierta, Cerrada }

impl EstadoCaja {
    pub fn as_str(self) -> &'static str {
        match self {
            EstadoCaja::Abierta => "abierta",
            EstadoCaja::Cerrada => "cerrada",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Caja {
    pub id_caja: i64,
    pub abierta_por: i64,
    pub abierta_en: String,       
    pub estado: EstadoCaja,
    pub cerrada_por: Option<i64>,
    pub cerrada_en: Option<String>,
}

#[derive(Serialize)]
pub struct MedioPagoResumen {
    pub medio: String,
    pub total_medio: i64,
}

#[derive(Serialize)]
pub struct CajaResumenDiario {
    pub id_cajas: Vec<i64>,
    pub cantidad_cajas: i32,
    pub cantidad_ventas: i32,
    pub total_general: i64,
    pub por_medio: Vec<MedioPagoResumen>,
}