export const rustGroups = {
  foundation: ["linop-db-types", "linop-db-platform", "linop-db-sql-core", "linop-db-sql-dialect", "linop-db-sql-data", "linop-db-sql-schema", "linop-db-sql", "linop-db-formats", "linop-db-ai-provider", "linop-db-plugin-runtime"],
  drivers: ["linop-db-driver-support", "linop-db-driver-agent", "linop-db-driver-elasticsearch", "linop-db-driver-mongodb", "linop-db-driver-mysql", "linop-db-driver-postgres", "linop-db-driver-redis", "linop-db-driver-sqlserver", "linop-db-drivers", "linop-db-sqlite-worker"],
  application: ["dbx", "linop-db-core", "linop-db-web", "linop-db-cli", "linop-db-mcp", "linop-db-tauri-consul", "linop-db-tauri-schema"],
};

export const goAgents = [
  { driver: "oracle-go", binary: "oracle", race: false },
  { driver: "xugu", binary: "xugu", race: false },
  { driver: "rabbitmq", binary: "rabbitmq", race: false },
  { driver: "rocketmq", binary: "rocketmq", race: true },
  { driver: "zookeeper", binary: "zookeeper", race: true },
  { driver: "cassandra-go", binary: "cassandra", race: false },
  { driver: "hive-go", binary: "hive", race: false },
  { driver: "vastbase-go", binary: "vastbase", race: false },
  { driver: "kingbase-go", binary: "kingbase", race: false },
  { driver: "neo4j-go", binary: "neo4j", race: false },
  { driver: "nebula-go", binary: "nebula", race: false },
  { driver: "iotdb", binary: "iotdb", race: false },
];

export const rustAgents = ["duckdb", "tdengine"];

export const integrationCases = [
  ...["3.4.14", "3.5.5", "3.7.0", "3.9.5"].map((version) => ({ driver: "zookeeper", scenario: "zookeeper", version })),
  { driver: "zookeeper", scenario: "zookeeper-sasl", version: "3.7.0" },
  ...["4.9.8", "5.3.1"].map((version) => ({ driver: "rocketmq", scenario: "rocketmq", version })),
  ...["2.4.0.14", "2.6.0.34", "3.0.7.1", "3.3.6.13", "3.4.2.2"].map((version) => ({
    driver: "tdengine",
    scenario: "tdengine",
    version,
    image: `tdengine/${version === "3.4.2.2" ? "tsdb" : "tdengine"}:${version}`,
  })),
  ...["3.11.19", "5.0.6"].map((version) => ({ driver: "cassandra-go", scenario: "cassandra", version })),
  ...["3.13", "4.3"].map((version) => ({ driver: "rabbitmq", scenario: "rabbitmq", version })),
];
