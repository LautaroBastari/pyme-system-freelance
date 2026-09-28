use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    // Captura automáticamente los errores de SQLx
    #[error("Error interno de base de datos")]
    Database(#[from] sqlx::Error),

    // Errores que queremos mostrarle explícitamente al usuario en React
    #[error("{0}")]
    Negocio(String),

    // Para cualquier otro fallo imprevisto
    #[error("Error del sistema: {0}")]
    Sistema(String),
}

// Tauri exige que el error implemente Serialize para enviarlo por IPC al frontend de React
impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        // Solo mandamos el mensaje formateado, ocultando detalles técnicos sensibles
        serializer.serialize_str(self.to_string().as_ref())
    }
}