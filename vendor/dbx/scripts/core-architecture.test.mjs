import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync, readdirSync } from "node:fs";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../", import.meta.url));
const read = (name) => readFileSync(path.join(root, name), "utf8");
const metadata = JSON.parse(
  execFileSync("cargo", ["metadata", "--locked", "--offline", "--no-deps", "--format-version", "1"], {
    cwd: root,
    encoding: "utf8",
    maxBuffer: 8 * 1024 * 1024,
  }),
);
const members = new Map(metadata.packages.filter((pkg) => metadata.workspace_members.includes(pkg.id)).map((pkg) => [pkg.name, pkg]));
const allowedDependencies = {
  "linop-db-types": [],
  "linop-db-platform": [],
  "linop-db-formats": [],
  "linop-db-sql-core": ["linop-db-types"],
  "linop-db-sql-dialect": ["linop-db-types"],
  "linop-db-sql-data": ["linop-db-sql-core", "linop-db-sql-dialect", "linop-db-types"],
  "linop-db-sql-schema": ["linop-db-sql-core", "linop-db-sql-dialect", "linop-db-types"],
  "linop-db-sql": ["linop-db-sql-core", "linop-db-sql-data", "linop-db-sql-dialect", "linop-db-sql-schema", "linop-db-types"],
  "linop-db-ai-provider": ["linop-db-platform"],
  "linop-db-plugin-runtime": ["linop-db-types", "linop-db-platform"],
  "linop-db-driver-support": ["linop-db-types", "linop-db-platform"],
  "linop-db-driver-agent": ["linop-db-types", "linop-db-sql-core", "linop-db-platform"],
  "linop-db-driver-elasticsearch": ["linop-db-types", "linop-db-driver-support"],
  "linop-db-driver-mongodb": ["linop-db-types", "linop-db-driver-support"],
  "linop-db-driver-mysql": ["linop-db-types", "linop-db-sql-core", "linop-db-sql-dialect", "linop-db-driver-support"],
  "linop-db-driver-postgres": ["linop-db-types", "linop-db-sql-core", "linop-db-driver-support"],
  "linop-db-driver-redis": ["linop-db-types", "linop-db-driver-support"],
  "linop-db-driver-sqlserver": ["linop-db-types", "linop-db-sql-core", "linop-db-sql-data", "linop-db-driver-support"],
  "linop-db-drivers": ["linop-db-types", "linop-db-sql-core", "linop-db-sql-data", "linop-db-sql-dialect", "linop-db-platform", "linop-db-sqlite-worker", "linop-db-driver-support", "linop-db-driver-agent", "linop-db-driver-elasticsearch", "linop-db-driver-mongodb", "linop-db-driver-mysql", "linop-db-driver-postgres", "linop-db-driver-redis", "linop-db-driver-sqlserver"],
};
const core = members.get("linop-db-core");

for (const [name, allowed] of Object.entries(allowedDependencies)) {
  test(`${name} has a one-way dependency boundary`, () => {
    const pkg = members.get(name);
    assert.ok(pkg, `${name} must be a workspace member`);
    assert.deepEqual(pkg.publish, [], `${name} is an internal workspace crate`);
    for (const dependency of pkg.dependencies) {
      if (members.has(dependency.name)) {
        assert.ok(allowed.includes(dependency.name), `${name} must not depend on ${dependency.name}, including test/build dependencies`);
      }
      if (dependency.kind !== "dev") {
        assert.ok(!dependency.features.includes("test-support"), `${name} must not enable production test hooks`);
      }
    }
  });
}

test("core owns application orchestration in real business directories", () => {
  const ownedFiles = [
    "connection/mod.rs",
    "connection/connection_secrets.rs",
    "connection/driver_runtime.rs",
    "query/mod.rs",
    "query/query_cancel.rs",
    "query/document_ops.rs",
    "schema/mod.rs",
    "schema/table_structure_sql/sqlite_rebuild.rs",
    "data/transfer.rs",
    "data/transfer_runner.rs",
    "data/table_import.rs",
    "data/docs/export.rs",
    "data/cloudflare_d1/import.rs",
    "ai/agent_loop.rs",
    "ai/agent_tools.rs",
    "admin/consul/mod.rs",
    "admin/mq/mod.rs",
    "admin/mqtt/mod.rs",
    "persistence/storage.rs",
    "persistence/config/mod.rs",
    "safety/production_safety.rs",
    "host/update.rs",
  ];
  for (const name of ownedFiles) {
    assert.ok(existsSync(path.join(root, "crates/linop-db-core/src", name)), `missing owned business source: ${name}`);
  }
  const rootSources = readdirSync(path.join(root, "crates/linop-db-core/src")).filter((name) => name.endsWith(".rs"));
  assert.deepEqual(rootSources, ["cc_switch.rs", "lib.rs"]);
  assert.match(read("crates/linop-db-core/src/ai/mod.rs"), /pub use linop_db_ai_provider::ai::\*/);
  assert.match(read("crates/linop-db-core/src/db/mod.rs"), /pub use crate::data::cloudflare_d1/);
  assert.ok(!existsSync(path.join(root, "crates/linop-db-core/build.rs")));
});

test("transfer entrypoints delegate orchestration to the core runner", () => {
  const entrypoints = [
    "src-tauri/src/commands/transfer.rs",
    "crates/linop-db-web/src/routes/transfer.rs",
  ];
  const orchestrationCalls = [
    "rename_tables_to_backup",
    "transfer_postgres_schema_dependencies",
    "clear_foreign_key_linked_overwrite_targets",
    "transfer_table_with_result",
    "transfer_schema_objects",
    "drop_backup_tables",
  ];

  for (const entrypoint of entrypoints) {
    const source = read(entrypoint);
    assert.match(source, /transfer_runner::run_transfer\(/, `${entrypoint} must use the shared transfer runner`);
    for (const call of orchestrationCalls) {
      const directCall = new RegExp(`(?:linop_db_core::)?transfer::${call}\\(`);
      assert.doesNotMatch(source, directCall, `${entrypoint} must not own ${call} orchestration`);
    }
  }
});

test("database comparison admission comes from the shared driver manifest", () => {
  const source = read("apps/desktop/src/lib/database/databaseCompareCapabilities.ts");
  assert.match(source, /supportsDatabaseCompareMode\(type, mode\)/);
  assert.doesNotMatch(source, /new Set<DatabaseType>/);
  assert.doesNotMatch(source, /SQL_COMPARE_TYPES/);
});

test("core forwards capability flags without enabling driver defaults or test hooks", () => {
  for (const feature of ["duckdb-sidecar", "dynamodb", "mq-admin", "sqlite-bundled", "sqlite-sqlcipher", "sqlite-multiple-ciphers"]) {
    assert.ok(core.features[feature].includes(`linop-db-drivers/${feature}`), `missing driver forwarding for ${feature}`);
  }
  for (const name of ["linop-db-types", "linop-db-sql"]) {
    assert.ok(core.features.openapi.includes(`${name}/openapi`));
  }
  for (const name of ["linop-db-types", "linop-db-sql", "linop-db-drivers"]) {
    const dependency = core.dependencies.find((dep) => dep.name === name && dep.kind === null);
    assert.equal(dependency.uses_default_features, false, `${name} must follow explicit core features`);
  }
  for (const dependency of core.dependencies.filter((dep) => dep.kind !== "dev")) {
    assert.ok(!dependency.features.includes("test-support"));
  }
  assert.deepEqual(core.features.default, ["duckdb-sidecar", "dynamodb", "mq-admin", "sqlite-sqlcipher", "system-fonts"]);
});

test("code generators and runtime assets belong to their implementation crates", () => {
  assert.match(read("crates/linop-db-types/build.rs"), /plugins\/connection-types/);
  assert.match(read("crates/linop-db-sql-dialect/build.rs"), /plugins\/dialects/);
  assert.match(read("crates/linop-db-ai-provider/src/ai_pi_agent_cli.rs"), /assets\/pi-mcp-bridge\.mjs/);
  assert.ok(existsSync(path.join(root, "crates/linop-db-ai-provider/assets/pi-mcp-bridge.mjs")));
  assert.deepEqual(JSON.parse(read("crates/linop-db-driver-agent/assets/agent-protocol-v2.json")), JSON.parse(read("agents/common/src/main/resources/agent-protocol-v2.json")));
  assert.match(read("crates/linop-db-platform/src/lib.rs"), /#\[cfg\(all\(target_os = "windows", target_env = "gnu"\)\)\]\s*mod nanosleep_stub/);
});

test("Docker dependency caching and Nix include the full workspace", () => {
  const dockerfile = read("deploy/Dockerfile");
  const dependencyStage = dockerfile.split("COPY crates/ crates/")[0];
  for (const pkg of members.values()) {
    const relativeManifest = path.relative(root, pkg.manifest_path).replaceAll(path.sep, "/");
    assert.ok(dependencyStage.includes(`COPY ${relativeManifest} `), `Docker dependency stage misses ${relativeManifest}`);
  }
  for (const input of ["plugins/dialects/", "plugins/connection-types/"]) {
    assert.ok(dockerfile.includes(`COPY ${input} ${input}`));
  }
  assert.ok(dockerfile.includes("linop-db-sqlite-worker/src/main.rs"));
  assert.match(read("flake.nix"), /src = pkgs\.lib\.cleanSource \.\/\.;/);
});

test("CI and release tracking follow the new source owners", () => {
  const ci = read(".github/workflows/ci.yml");
  assert.ok(ci.includes("node --test scripts/core-architecture.test.mjs"));
  assert.ok(ci.includes("'crates/**'"));
  for (const input of ["plugins/dialects/**", "plugins/connection-types/**", "crates/linop-db-driver-agent/assets/agent-protocol-v2.json"]) {
    assert.ok(ci.includes(`'${input}'`), `CI misses ${input}`);
  }
  assert.ok(read("scripts/release.mjs").includes('"crates/linop-db-driver-mongodb/src/mongo_shell.rs"'));
  assert.ok(read("scripts/release.mjs").includes('"skills/dbx/"'));
  assert.ok(ci.includes("'skills/dbx/**'"));
});

test("the standalone DuckDB lockfile includes core's internal dependency closure", () => {
  const lockedPackages = new Map(
    read("agents/drivers/duckdb/Cargo.lock")
      .replaceAll("\r\n", "\n")
      .split("\n[[package]]\n")
      .slice(1)
      .map((block) => [block.match(/^name = "([^"]+)"$/m)[1], block]),
  );
  const visited = new Set();
  function inspect(name) {
    if (visited.has(name)) return;
    visited.add(name);
    const pkg = members.get(name);
    const locked = lockedPackages.get(name);
    assert.ok(locked, `DuckDB lockfile misses ${name}`);
    assert.ok(locked.includes(`version = "${pkg.version}"\n`), `DuckDB lockfile has a stale version of ${name}`);
    assert.doesNotMatch(locked, /^source = /m);
    const dependencies = new Set([...locked.matchAll(/^ "([^"]+)",$/gm)].map((match) => match[1].split(" ")[0]));
    for (const dependency of pkg.dependencies.filter((dep) => dep.kind !== "dev" && !dep.optional && members.has(dep.name))) {
      assert.ok(dependencies.has(dependency.name), `DuckDB lockfile misses ${name} -> ${dependency.name}`);
      inspect(dependency.name);
    }
  }
  inspect("linop-db-core");
});

test("source-sensitive client tests point to existing owners", () => {
  function inspect(directory) {
    for (const entry of readdirSync(path.join(root, directory), { withFileTypes: true })) {
      const name = `${directory}/${entry.name}`;
      if (entry.isDirectory()) inspect(name);
      else if (/\.(?:spec|test)\.[cm]?[jt]s$/.test(name)) {
        for (const match of read(name).matchAll(/crates\/linop-db-[a-z-]+\/(?:src|assets)\/[a-zA-Z0-9_/.-]+\.(?:rs|mjs)/g)) {
          assert.ok(existsSync(path.join(root, match[0])), `${name} points at missing source ${match[0]}`);
        }
      }
    }
  }
  inspect("apps/desktop/src");
  inspect("packages/app-tests");
});
