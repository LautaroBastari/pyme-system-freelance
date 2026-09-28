# Sistema Desktop de Ventas, Stock y Caja

![CI](https://github.com/LautaroBastari/pyme-system-freelance/actions/workflows/ci.yaml/badge.svg)

Aplicación de escritorio **offline** para comercios minoristas, con ventas, stock, caja, compras, gastos y reportes de rentabilidad. Desarrollada como proyecto freelance y **en uso real en un comercio**.

![Ventas](docs/screenshots/02_venta.png)

---

## Problema que resuelve

En muchos comercios pequeños y medianos es habitual:

- Registrar ventas de forma manual o en planillas
- Tener desfasajes entre el stock real y el registrado
- No contar con cierres de caja claros ni horarios consistentes
- No poder reconstruir qué ocurrió ante errores o faltantes
- No saber cuánto gana realmente el negocio una vez descontados costos, gastos y sueldos

Este sistema centraliza la operación diaria y asegura que:

- Cada venta impacte directamente en el stock
- La caja quede registrada con apertura y cierre por jornada
- Cada movimiento de stock guarde el costo que tenía en ese momento
- Todas las operaciones queden guardadas para su posterior análisis

---

## Funcionalidades

### Ventas
- Registro de ventas con detalle por producto
- Cálculo automático de totales y descuento de stock por ítem
- Pagos combinados con distintos medios (efectivo, débito, crédito, transferencia)
- Promociones y combos (packs con precio propio)
- Estados de la venta: en curso, finalizada, anulada

### Stock
- Alta y edición de productos, con baja lógica (activar / desactivar)
- Movimientos de stock con historial: compras, mermas y ajustes
- Ajuste a un valor absoluto: si el stock baja, se registra como pérdida al costo actual
- Historial de precios de venta y de costo por producto
- Reposición configurable por producto (unitario o por cajón)

### Compras
- Registro de compras con opción de actualizar o mantener el costo del producto

### Caja
- Apertura y cierre de caja con timestamps
- No permite cerrar una caja mientras haya una venta en curso
- Resumen diario por usuario, con totales por medio de pago

### Gastos y sueldos
- Registro de gastos del negocio por categoría
- Registro de pagos de sueldos a usuarios
- Totales y listados por período

### Usuarios y roles
- Inicio de sesión y registro
- Roles diferenciados (admin / operador)
- Acceso a vistas según rol

### Reportes
- Rendimiento de ventas y resultado del negocio (PNL y ganancias)
- Reporte de inventario: valor total, rotación, días de stock restante, variación de ventas contra el período anterior, clasificación ABC y riesgo de quiebre
- Reporte de reposición por rango de fechas

---

## Tecnologías y arquitectura

- **Frontend:** React + TypeScript + Tailwind CSS
- **Desktop:** Tauri
- **Backend:** Rust (comandos Tauri)
- **Base de datos:** SQLite
- **Acceso a datos:** SQLx + migraciones
- **CI:** GitHub Actions (compilación, tests y clippy en cada push)

---

## Decisiones técnicas

### ¿Por qué Tauri + Rust y no Electron?

El sistema corre en las PCs reales del negocio, que suelen ser equipos de varios años. Tauri usa el motor web nativo del sistema operativo en lugar de empaquetar un Chromium completo, lo que da un instalador y un consumo de memoria mucho menores. La lógica de stock y caja corre en Rust, que previene en tiempo de compilación errores de memoria comunes, algo valioso cuando se manejan movimientos de dinero y mercadería sin conexión.

**Trade-off asumido:** Rust tiene una curva de desarrollo más alta que otros stacks. Se aceptó a cambio de un producto más liviano, rápido y confiable para el contexto real de uso.

### ¿Por qué SQLite?

Permite una aplicación liviana, sin servidor y completamente offline. Los datos viven en un único archivo local, adecuado para el uso diario en escritorio.

### Organización del backend

Cada módulo (`caja`, `stock`, `compras`, `gastos`) separa responsabilidades:

| Archivo | Responsabilidad |
|---|---|
| `commands.rs` | Comandos Tauri: reciben el pedido y orquestan |
| `repo.rs` | Acceso a datos (SQL) |
| `logica.rs` | Reglas de negocio puras, testeables sin base de datos |
| `model.rs` | Tipos de entrada y salida |

Separar las reglas de negocio de las consultas SQL permite testearlas de forma aislada: cálculo de costos, ajustes de stock, validaciones de compra y de cierre de caja, clasificación ABC, niveles de riesgo, entre otras.

---

## Calidad y tests

El proyecto cuenta con **66 tests unitarios** sobre la lógica de negocio, que se ejecutan automáticamente en cada push mediante GitHub Actions.

```bash
cd src-tauri
cargo test
```

---

## Modelo de datos

| Tabla | Descripción |
|---|---|
| `usuario` | Credenciales, rol y estado |
| `caja` | Aperturas y cierres por jornada |
| `producto` | Catálogo, precio y costo actual, configuración de reposición |
| `producto_stock` | Stock actual por producto |
| `stock_mov` | Movimientos de stock, con costo histórico de cada uno |
| `precio_historial` | Vigencia de precios de venta y de costo |
| `venta` | Cabecera de la operación, con estado |
| `venta_item` | Productos de la venta, con precio, costo al momento y datos de promo |
| `venta_pago` | Pagos de la venta, uno por medio de pago |
| `promo_combo` / `promo_combo_item` | Combos promocionales y sus productos |
| `gasto_negocio` | Gastos operativos |
| `sueldo_pago` | Pagos de sueldos |
| `gasto_rentabilidad` | Registro contable que alimenta el resultado del negocio |

**Integridad de datos**
- Claves foráneas activadas
- Restricciones `CHECK` en montos, cantidades y estados
- Importes almacenados como enteros
- Cada ítem de venta guarda el costo unitario del momento, para que la rentabilidad histórica no cambie si el costo se actualiza después

---

## Flujo de una venta

1. El operador inicia sesión
2. Selecciona productos y cantidades
3. Se validan los datos ingresados
4. Se registra la venta y sus ítems
5. Se registran los pagos por medio
6. Se actualiza el stock de cada producto
7. La operación queda persistida

Las operaciones críticas se ejecutan dentro de transacciones, evitando inconsistencias entre ventas, pagos y stock.

---

## Estructura del repositorio

```
src/                       Frontend (React / TypeScript)
src-tauri/                 Backend en Rust y comandos Tauri
src-tauri/migrations/      Migraciones SQL
docs/screenshots/          Capturas del sistema
.github/workflows/         CI (GitHub Actions)
```

---

## Capturas

![Login](docs/screenshots/01_login.png)
![Ventas](docs/screenshots/02_venta.png)
![Stock](docs/screenshots/03_stock.png)
![Resultados (PNL)](docs/screenshots/04_pnl.png)
![Ganancias](docs/screenshots/05_ganancias.png)
![Caja](docs/screenshots/06_cajas.png)

---

## Ejecución en desarrollo

### Requisitos

- Node.js (LTS)
- Rust toolchain
- En Windows: Visual Studio Build Tools con el componente "Desarrollo para el escritorio con C++"
- SQLx CLI (opcional, para manejo manual de migraciones)

### Pasos

```bash
git clone https://github.com/LautaroBastari/pyme-system-freelance
cd pyme-system-freelance
npm install
npm run tauri dev
```

La base de datos se inicializa y migra automáticamente. En Windows queda en el directorio de datos de la aplicación (`AppData/Roaming/...`).

---

## Estado del proyecto

- [x] Autenticación y roles
- [x] Ventas con pagos combinados y promociones
- [x] Stock con movimientos, costos históricos y reposición
- [x] Compras, gastos y sueldos
- [x] Caja con apertura, cierre y resumen diario
- [x] Reportes de inventario, rentabilidad y ganancias
- [x] Tests unitarios de la lógica de negocio (66) y CI
- [ ] Manejo de errores unificado y registro de logs en archivo
- [ ] Backups automáticos de la base de datos
- [ ] Instalador descargable desde Releases

---

## Autor

Lautaro Bastari
GitHub: https://github.com/LautaroBastari

Documentación detallada: https://drive.google.com/drive/folders/1ROyFuz2DuQomwt8iiL6bmKo52x8VZIuo?hl=es-419
