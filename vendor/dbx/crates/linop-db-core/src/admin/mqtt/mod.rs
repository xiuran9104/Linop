//! MQTT 模块：MQTT broker 连接管理、消息订阅/发布。
//!
//! ## 架构
//! ```text
//! DBX 前端 (Vue)
//!     │ Tauri invoke
//!     ▼
//! src-tauri/src/commands/mqtt_cmd.rs  (Tauri command 入口)
//!     │
//!     ▼
//! crates/linop-db-core/src/admin/mqtt/service.rs  (共享核心逻辑)
//!     │
//!     ▼
//! crates/linop-db-core/src/admin/mqtt/client.rs   (rumqttc 客户端封装)
//!     │
//!     ▼
//! MQTT Broker (EMQX / Mosquitto / HiveMQ / ...)
//! ```

pub mod client;
pub mod service;
pub use linop_db_types::mqtt as types;
