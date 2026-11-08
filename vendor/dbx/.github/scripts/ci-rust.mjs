import { spawnSync } from "node:child_process";
import path from "node:path";
import { pathToFileURL } from "node:url";
import { rustGroups } from "./ci-config.mjs";

export function rustCommand(action, group, mode) {
  if (!["test", "doctest", "clippy", "tree"].includes(action) || !["full", "fast"].includes(mode) || (group !== "workspace" && !Object.hasOwn(rustGroups, group)) || (action === "clippy" && group !== "workspace")) {
    throw new Error(`Unknown Rust CI configuration: ${action}/${group}/${mode}`);
  }
  const capabilityFeatures = ["duckdb-sidecar", "dynamodb", "mq-admin", "sqlite-sqlcipher", "sqlserver-native-attention"];
  if (mode === "full") capabilityFeatures.push("system-fonts");
  const appFeatures = ["dbx", "linop-db-core", "linop-db-web"].flatMap((name) => capabilityFeatures.map((feature) => `${name}/${feature}`));
  const foundationFeatures = [
    "linop-db-types/mq-admin",
    "linop-db-types/openapi",
    "linop-db-sql/duckdb-sidecar",
    "linop-db-sql/openapi",
    "linop-db-sql-data/openapi",
    "linop-db-sql-schema/duckdb-sidecar",
    "linop-db-platform/downloads",
    "linop-db-platform/host-prompts",
    "linop-db-platform/test-support",
    "linop-db-plugin-runtime/default",
    "linop-db-plugin-runtime/test-support",
  ];
  const features =
    group === "foundation"
      ? foundationFeatures
      : group === "drivers"
        ? [
            ...["duckdb-sidecar", "dynamodb", "mq-admin", "sqlite-bundled", "sqlite-sqlcipher", "sqlserver-native-attention", "test-support"].map((feature) => `linop-db-drivers/${feature}`),
            "linop-db-driver-agent/test-support",
            "linop-db-driver-mysql/test-support",
            "linop-db-driver-redis/test-support",
            "linop-db-driver-support/test-support",
            "linop-db-sqlite-worker/runtime",
            "linop-db-types/openapi",
            "linop-db-sql-data/openapi",
          ]
        : appFeatures;
  const packages = group === "workspace" ? ["--workspace"] : rustGroups[group].flatMap((name) => ["--package", name]);
  const command = action === "test" ? ["nextest", "run", "--no-fail-fast"] : action === "doctest" ? ["test", "--doc"] : [action];
  return [...command, ...packages, "--locked", ...(["test", "clippy"].includes(action) ? ["--timings"] : []), ...(action === "clippy" ? ["--all-targets"] : []), "--no-default-features", "--features", features.join(","), ...(action === "clippy" ? ["--", "-D", "warnings"] : [])];
}

if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) {
  const args = rustCommand(...process.argv.slice(2, 5));
  console.log(`cargo ${args.join(" ")}`);
  const result = spawnSync("cargo", args, { stdio: "inherit" });
  if (result.error) throw result.error;
  process.exit(result.status ?? 1);
}
