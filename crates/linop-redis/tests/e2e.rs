//! Redis 客户端端到端:linop-redis ↔ testsupport 迷你 Redis。

use linop_redis::{KeyValue, RedisClient};
use linop_testsupport::{RedisData, TestRedisServer};

fn preloaded() -> RedisData {
    RedisData::default()
        .with_string("app:name", "linop")
        .with_string("app:count", "42")
        .with_list("queue:jobs", &["job-1", "job-2", "job-3"])
        .with_hash("user:1", &[("name", "alice"), ("role", "admin")])
        .with_set("tags", &["red", "green", "blue"])
        .with_zset("rank", &[(1.5, "a"), (0.5, "b"), (2.0, "c")])
}

#[tokio::test]
async fn scan_infos_and_values() -> anyhow::Result<()> {
    let server = TestRedisServer::start(preloaded())?;
    let mut client = RedisClient::connect(&server.addr.to_string(), None)?;

    assert_eq!(client.dbsize()?, 6);

    // SCAN:按字典序返回。
    let keys = client.scan_keys(100)?;
    assert_eq!(
        keys,
        vec![
            "app:count",
            "app:name",
            "queue:jobs",
            "rank",
            "tags",
            "user:1"
        ]
    );

    // 概要:类型/长度/TTL。
    let infos = client.key_infos(&keys)?;
    let find = |name: &str| infos.iter().find(|i| i.name == name).unwrap().clone();
    let info = find("app:name");
    assert_eq!(
        (info.kind.as_str(), info.len, info.ttl_secs),
        ("string", 5, -1)
    );
    assert_eq!(find("queue:jobs").kind, "list");
    assert_eq!(find("queue:jobs").len, 3);
    assert_eq!(find("user:1").kind, "hash");
    assert_eq!(find("tags").kind, "set");
    assert_eq!(find("rank").kind, "zset");

    // 值:五种类型。
    assert_eq!(
        client.key_value("app:name", "string")?,
        KeyValue::Str("linop".into())
    );
    assert_eq!(
        client.key_value("queue:jobs", "list")?,
        KeyValue::List(vec!["job-1".into(), "job-2".into(), "job-3".into()])
    );
    assert_eq!(
        client.key_value("user:1", "hash")?,
        KeyValue::Hash(vec![
            ("name".into(), "alice".into()),
            ("role".into(), "admin".into())
        ])
    );
    let KeyValue::Set(members) = client.key_value("tags", "set")? else {
        panic!("tags 应为 set");
    };
    assert_eq!(members, vec!["blue", "green", "red"]);
    assert_eq!(
        client.key_value("rank", "zset")?,
        KeyValue::ZSet(vec![
            ("b".into(), "0.5".into()),
            ("a".into(), "1.5".into()),
            ("c".into(), "2".into())
        ])
    );

    // DEL + 不存在的键。
    assert!(client.del("tags")?);
    assert!(!client.del("tags")?);
    assert!(client.key_info("tags")?.is_none());
    assert!(client.key_value("tags", "string").is_ok()); // GET 缺键 → BulkNil → 空串
    Ok(())
}

#[tokio::test]
async fn scan_limit_truncates() -> anyhow::Result<()> {
    let server = TestRedisServer::start(preloaded())?;
    let mut client = RedisClient::connect(&server.addr.to_string(), None)?;
    let keys = client.scan_keys(3)?;
    assert_eq!(keys.len(), 3);
    assert_eq!(keys[0], "app:count"); // 截断保持字典序前缀
    Ok(())
}

#[tokio::test]
async fn auth_enforced_by_server() -> anyhow::Result<()> {
    let data = RedisData::default().with_string("k", "v");
    let data = RedisData {
        password: Some("secret".into()),
        ..data
    };
    let server = TestRedisServer::start(data)?;

    // 错误口令:AUTH 被拒。
    assert!(RedisClient::connect(&server.addr.to_string(), Some("wrong")).is_err());
    // 正确口令:可浏览。
    let mut client = RedisClient::connect(&server.addr.to_string(), Some("secret"))?;
    assert_eq!(client.key_value("k", "string")?, KeyValue::Str("v".into()));
    Ok(())
}
