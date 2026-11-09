//! 数据库 profile 模型与注册表(`databases.toml`)。

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::{CoreError, Result};

/// 一个数据库连接 profile。
/// - `sqlite`:本地文件(`path`);
/// - `redis`:TCP 服务(`host:port`,可选 `password`);
/// - `postgres` / `mysql`:TCP 服务(`host:port`,`user`/`database`/`password`);
///
/// 驱动字段为后续更多引擎保留(数据库内核 = DBX,见 linop-dbx)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DbProfile {
    pub name: String,
    pub driver: DbDriver,
    /// SQLite:数据库文件路径。
    pub path: PathBuf,
    /// Redis/Postgres/MySQL:主机与端口(缺省 6379 / 5432 / 3306)。
    pub host: Option<String>,
    pub port: Option<u16>,
    /// Redis/Postgres/MySQL:口令(可选)。
    pub password: Option<String>,
    /// Postgres/MySQL:用户名(缺省 postgres / root)。
    pub user: Option<String>,
    /// Postgres/MySQL:库名(缺省引擎默认库)。
    pub database: Option<String>,
    /// 经由哪条 SSH 连接(profile 名)建隧道访问;None = 直连。
    pub via: Option<String>,
}

/// 已编译进 linop-db-drivers 的驱动全集(db/mod.rs + 核心路由覆盖;
/// as_str 即 DBX DatabaseType 的 serde 名,注入后按其路由到驱动)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DbDriver {
    Sqlite,
    Redis,
    Postgres,
    MySql,
    Mongo,
    SqlServer,
    Elastic,
    DuckDb,
    // ---- MySQL 协议家族 ----
    Doris,
    StarRocks,
    ManticoreSearch,
    // ---- ClickHouse / 时序 ----
    ClickHouse,
    QuestDB,
    InfluxDB,
    InfluxDB3,
    VictoriaMetrics,
    // ---- 搜索 ----
    Solr,
    Meilisearch,
    EasySearch,
    CouchDB,
    // ---- 其他 ----
    HBase,
    RQLite,
    Turso,
    CloudflareD1,
    DynamoDB,
}

impl DbDriver {
    pub fn as_str(&self) -> &'static str {
        match self {
            DbDriver::Sqlite => "sqlite",
            DbDriver::Redis => "redis",
            DbDriver::Postgres => "postgres",
            DbDriver::MySql => "mysql",
            DbDriver::Mongo => "mongodb",
            DbDriver::SqlServer => "sqlserver",
            DbDriver::Elastic => "elasticsearch",
            DbDriver::DuckDb => "duckdb",
            DbDriver::Doris => "doris",
            DbDriver::StarRocks => "starrocks",
            DbDriver::ManticoreSearch => "manticoresearch",
            DbDriver::ClickHouse => "clickhouse",
            DbDriver::QuestDB => "questdb",
            DbDriver::InfluxDB => "influxdb",
            DbDriver::InfluxDB3 => "influxdb3",
            DbDriver::VictoriaMetrics => "victoriametrics",
            DbDriver::Solr => "solr",
            DbDriver::Meilisearch => "meilisearch",
            DbDriver::EasySearch => "easysearch",
            DbDriver::CouchDB => "couchdb",
            DbDriver::HBase => "hbase",
            DbDriver::RQLite => "rqlite",
            DbDriver::Turso => "turso",
            DbDriver::CloudflareD1 => "cloudflare-d1",
            DbDriver::DynamoDB => "dynamodb",
        }
    }

    /// 未显式填端口时的驱动缺省端口;文件/云/特殊通道(0)不走常规端口。
    pub fn default_port(self) -> u16 {
        match self {
            DbDriver::Sqlite | DbDriver::DuckDb => 0,
            DbDriver::Redis => 6379,
            DbDriver::Postgres => 5432,
            DbDriver::MySql => 3306,
            DbDriver::Mongo => 27017,
            DbDriver::SqlServer => 1433,
            DbDriver::Elastic | DbDriver::EasySearch => 9200,
            DbDriver::Doris | DbDriver::StarRocks => 9030,
            DbDriver::ManticoreSearch => 9306,
            DbDriver::ClickHouse => 8123,
            DbDriver::QuestDB => 8812,
            DbDriver::InfluxDB => 8086,
            DbDriver::InfluxDB3 => 8181,
            DbDriver::VictoriaMetrics => 8428,
            DbDriver::Solr => 8983,
            DbDriver::Meilisearch => 7700,
            DbDriver::CouchDB => 5984,
            DbDriver::RQLite => 4001,
            DbDriver::HBase
            | DbDriver::Turso
            | DbDriver::CloudflareD1
            | DbDriver::DynamoDB => 0,
        }
    }

    /// 文件型驱动(路径即库,无主机端口)。
    pub fn is_file_based(self) -> bool {
        matches!(self, DbDriver::Sqlite | DbDriver::DuckDb)
    }

    /// MySQL 协议家族(标识符引用用反引号)。
    pub fn is_mysql_family(self) -> bool {
        matches!(
            self,
            DbDriver::MySql
                | DbDriver::Doris
                | DbDriver::StarRocks
                | DbDriver::ManticoreSearch
        )
    }

    /// ClickHouse(标识符引用用反引号,字符串字面量单引号)。
    pub fn is_clickhouse(self) -> bool {
        matches!(self, DbDriver::ClickHouse)
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "sqlite" => Some(DbDriver::Sqlite),
            "duckdb" => Some(DbDriver::DuckDb),
            "redis" => Some(DbDriver::Redis),
            "postgres" | "postgresql" => Some(DbDriver::Postgres),
            "mysql" | "mariadb" => Some(DbDriver::MySql),
            "mongo" | "mongodb" => Some(DbDriver::Mongo),
            "sqlserver" | "mssql" => Some(DbDriver::SqlServer),
            "elastic" | "elasticsearch" => Some(DbDriver::Elastic),
            "doris" => Some(DbDriver::Doris),
            "starrocks" => Some(DbDriver::StarRocks),
            "manticore" | "manticoresearch" => Some(DbDriver::ManticoreSearch),
            "clickhouse" | "ch" => Some(DbDriver::ClickHouse),
            "questdb" => Some(DbDriver::QuestDB),
            "influxdb" | "influx" => Some(DbDriver::InfluxDB),
            "influxdb3" | "influx3" => Some(DbDriver::InfluxDB3),
            "victoriametrics" | "vm" => Some(DbDriver::VictoriaMetrics),
            "solr" => Some(DbDriver::Solr),
            "meilisearch" | "meili" => Some(DbDriver::Meilisearch),
            "easysearch" => Some(DbDriver::EasySearch),
            "couchdb" => Some(DbDriver::CouchDB),
            "hbase" => Some(DbDriver::HBase),
            "rqlite" => Some(DbDriver::RQLite),
            "turso" => Some(DbDriver::Turso),
            "cloudflare-d1" | "d1" => Some(DbDriver::CloudflareD1),
            "dynamodb" => Some(DbDriver::DynamoDB),
            _ => None,
        }
    }
}

impl DbProfile {
    pub fn sqlite(name: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        DbProfile {
            name: name.into(),
            driver: DbDriver::Sqlite,
            path: path.into(),
            host: None,
            port: None,
            password: None,
            user: None,
            database: None,
            via: None,
        }
    }

    pub fn redis(
        name: impl Into<String>,
        host: impl Into<String>,
        port: u16,
        password: Option<String>,
    ) -> Self {
        DbProfile {
            name: name.into(),
            driver: DbDriver::Redis,
            path: PathBuf::new(),
            host: Some(host.into()),
            port: Some(port),
            password,
            user: None,
            database: None,
            via: None,
        }
    }

    pub fn postgres(
        name: impl Into<String>,
        host: impl Into<String>,
        port: u16,
        user: impl Into<String>,
        password: Option<String>,
        database: Option<String>,
    ) -> Self {
        DbProfile {
            name: name.into(),
            driver: DbDriver::Postgres,
            path: PathBuf::new(),
            host: Some(host.into()),
            port: Some(port),
            password,
            user: Some(user.into()),
            database,
            via: None,
        }
    }

    pub fn mysql(
        name: impl Into<String>,
        host: impl Into<String>,
        port: u16,
        user: impl Into<String>,
        password: Option<String>,
        database: Option<String>,
    ) -> Self {
        DbProfile {
            name: name.into(),
            driver: DbDriver::MySql,
            path: PathBuf::new(),
            host: Some(host.into()),
            port: Some(port),
            password,
            user: Some(user.into()),
            database,
            via: None,
        }
    }

    /// Redis 端点 `host:port`(端口缺省 6379)。
    pub fn redis_endpoint(&self) -> String {
        format!(
            "{}:{}",
            self.host.clone().unwrap_or_default(),
            self.port.unwrap_or(6379)
        )
    }

    /// Postgres 端点 `host:port/db`(端口缺省 5432)。
    pub fn pg_endpoint(&self) -> String {
        format!(
            "{}:{}/{}",
            self.host.clone().unwrap_or_default(),
            self.port.unwrap_or(5432),
            self.database.clone().unwrap_or_default()
        )
    }

    /// MySQL 端点 `host:port`(端口缺省 3306)。
    pub fn mysql_endpoint(&self) -> String {
        format!(
            "{}:{}",
            self.host.clone().unwrap_or_default(),
            self.port.unwrap_or(3306)
        )
    }
}

/// 数据库注册表,按名称排序。
#[derive(Debug, Clone, Default)]
pub struct DbRegistry {
    profiles: Vec<DbProfile>,
}

/// databases.toml 中的条目结构。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct RawDb {
    driver: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    host: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    port: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    password: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    user: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    database: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    via: Option<String>,
}

/// 默认数据库配置路径:`~/.config/linop/databases.toml`。
pub fn default_databases_path() -> PathBuf {
    crate::config_dir().join("databases.toml")
}

impl FromIterator<DbProfile> for DbRegistry {
    fn from_iter<I: IntoIterator<Item = DbProfile>>(iter: I) -> Self {
        let mut reg = Self::default();
        for profile in iter {
            reg.upsert(profile);
        }
        reg
    }
}

impl DbRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn profiles(&self) -> &[DbProfile] {
        &self.profiles
    }

    pub fn get(&self, name: &str) -> Option<&DbProfile> {
        self.profiles.iter().find(|p| p.name == name)
    }

    pub fn upsert(&mut self, profile: DbProfile) {
        match self.profiles.iter_mut().find(|p| p.name == profile.name) {
            Some(slot) => *slot = profile,
            None => {
                self.profiles.push(profile);
                self.profiles.sort_by(|a, b| a.name.cmp(&b.name));
            }
        }
    }

    pub fn remove(&mut self, name: &str) -> Result<DbProfile> {
        let idx = self
            .profiles
            .iter()
            .position(|p| p.name == name)
            .ok_or_else(|| CoreError::NotFound(name.to_string()))?;
        Ok(self.profiles.remove(idx))
    }

    /// 加载;文件不存在返回空表,驱动名非法的条目报错。
    pub fn load(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(path)?;
        let raw: BTreeMap<String, RawDb> =
            toml::from_str(&text).map_err(|source| CoreError::Parse {
                path: path.to_path_buf(),
                source,
            })?;
        let mut profiles = Vec::new();
        for (name, entry) in raw {
            let driver = DbDriver::parse(&entry.driver).ok_or_else(|| {
                CoreError::NotFound(format!("数据库 {name} 的驱动不支持:{}", entry.driver))
            })?;
            profiles.push(DbProfile {
                name,
                driver,
                path: PathBuf::from(entry.path.unwrap_or_default()),
                host: entry.host,
                port: entry.port,
                password: entry.password,
                user: entry.user,
                database: entry.database,
                via: entry.via,
            });
        }
        profiles.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(Self { profiles })
    }

    pub fn save(&self, path: impl AsRef<std::path::Path>) -> Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let raw: BTreeMap<String, RawDb> = self
            .profiles
            .iter()
            .map(|p| {
                (
                    p.name.clone(),
                    RawDb {
                        driver: p.driver.as_str().to_string(),
                        path: if p.path.as_os_str().is_empty() {
                            None
                        } else {
                            Some(p.path.display().to_string())
                        },
                        host: p.host.clone(),
                        port: p.port,
                        password: p.password.clone(),
                        user: p.user.clone(),
                        database: p.database.clone(),
                        via: p.via.clone(),
                    },
                )
            })
            .collect();
        let text = toml::to_string_pretty(&raw)?;
        let tmp = path.with_extension("toml.tmp");
        std::fs::write(&tmp, text)?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }

    pub fn load_default() -> Result<Self> {
        Self::load(default_databases_path())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_databases_toml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("databases.toml");
        let mut reg = DbRegistry::new();
        reg.upsert(DbProfile::sqlite("local", "/tmp/app.db"));
        reg.upsert(DbProfile::sqlite("zeta", "/home/me/z.db"));
        reg.save(&path).unwrap();

        let loaded = DbRegistry::load(&path).unwrap();
        assert_eq!(loaded.profiles(), reg.profiles());
        // 按名称排序。
        assert_eq!(loaded.profiles()[0].name, "local");
        assert_eq!(loaded.profiles()[1].name, "zeta");
    }

    #[test]
    fn missing_file_and_bad_driver() {
        assert!(
            DbRegistry::load("/nonexistent/databases.toml")
                .unwrap()
                .profiles()
                .is_empty()
        );

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("d.toml");
        std::fs::write(&path, "[x]\ndriver = \"oracle\"\npath = \"/a\"").unwrap();
        assert!(DbRegistry::load(&path).is_err());
    }

    #[test]
    fn driver_parse() {
        assert_eq!(DbDriver::parse("SQLite"), Some(DbDriver::Sqlite));
        assert_eq!(DbDriver::parse("Redis"), Some(DbDriver::Redis));
        assert_eq!(DbDriver::parse("mongodb"), Some(DbDriver::Mongo));
        assert_eq!(DbDriver::parse("MSSQL"), Some(DbDriver::SqlServer));
        assert_eq!(DbDriver::parse("elastic"), Some(DbDriver::Elastic));
        assert_eq!(DbDriver::parse("duckdb"), Some(DbDriver::DuckDb));
        assert_eq!(DbDriver::parse("mariadb"), Some(DbDriver::MySql));
        assert_eq!(DbDriver::parse("oracle"), None);
    }

    #[test]
    fn redis_profile_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("databases.toml");
        let mut reg = DbRegistry::new();
        reg.upsert(DbProfile::redis(
            "cache",
            "10.0.0.5",
            6380,
            Some("secret".into()),
        ));
        reg.save(&path).unwrap();

        let loaded = DbRegistry::load(&path).unwrap();
        let cache = loaded.get("cache").unwrap();
        assert_eq!(cache.redis_endpoint(), "10.0.0.5:6380");

        // via 字段往返。
        let mut tunneled = cache.clone();
        tunneled.name = "tun".into();
        tunneled.via = Some("jump".into());
        reg.upsert(tunneled);
        reg.save(&path).unwrap();
        let loaded = DbRegistry::load(&path).unwrap();
        assert_eq!(loaded.get("tun").unwrap().via.as_deref(), Some("jump"));
        assert_eq!(loaded.get("cache").unwrap().via, None);
    }
}
