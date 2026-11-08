import type { DatabaseType } from "@/types/database";
import { queryResultTransferSqlLiteral } from "./queryResultTransferValues";

/** Keep the Oracle multi-row form in sync with data/transfer.rs::InsertSqlTemplate. */
export function queryResultTransferInsertSql(table: string, columns: string[], values: string[], databaseType: DatabaseType, overrideIdentity = false): string[] {
  if (!values.length) return [];
  const destination = `${table} (${columns.join(", ")})`;
  if ((databaseType === "oracle" || databaseType === "oceanbase-oracle") && values.length > 1) {
    return [`INSERT ALL\n${values.map((row) => `INTO ${destination} VALUES ${row}`).join("\n")}\nSELECT 1 FROM dual`];
  }
  const override = overrideIdentity ? " OVERRIDING SYSTEM VALUE" : "";
  // Single-row statements also work on JDBC engines without comma-separated VALUES.
  if (!["mysql", "postgres", "sqlite", "sqlserver", "h2", "kingbase", "highgo", "vastbase", "goldendb"].includes(databaseType)) {
    return values.map((row) => `INSERT INTO ${destination}${override} VALUES ${row}`);
  }
  return [`INSERT INTO ${destination}${override} VALUES ${values.join(",\n")}`];
}

/** IDENTITY_INSERT is session state: restore it on both the success and error paths. */
export function withSqlServerIdentityInsert(table: string, statements: string[]): string {
  return `BEGIN TRY\nSET IDENTITY_INSERT ${table} ON;\n${statements.join(";\n")};\nSET IDENTITY_INSERT ${table} OFF;\nEND TRY\nBEGIN CATCH\nDECLARE @linop_db_transfer_error nvarchar(2048);\nSET @linop_db_transfer_error = ERROR_MESSAGE();\nSET IDENTITY_INSERT ${table} OFF;\nRAISERROR(N'%s', 16, 1, @linop_db_transfer_error);\nEND CATCH`;
}

/**
 * Advance an owned PostgreSQL sequence without moving it backwards or changing
 * its increment grid. ALTER SEQUENCE locks concurrent nextval calls; RESTART is
 * transactional on PostgreSQL 10+, unlike setval. The catalog branch preserves
 * compatibility with older SERIAL-only servers.
 * https://www.postgresql.org/docs/current/sql-altersequence.html
 * https://www.postgresql.org/docs/9.6/sql-altersequence.html
 */
export function postgresTransferSequenceSyncSql(table: string, column: string): string {
  const literal = (value: string) => queryResultTransferSqlLiteral(value, "postgres")!;
  const quotedColumn = '"' + column.replaceAll('"', '""') + '"';
  const body = `DECLARE
  linop_db_sequence regclass;
  linop_db_increment bigint;
  linop_db_last numeric;
  linop_db_called boolean;
  linop_db_next numeric;
  linop_db_edge numeric;
  linop_db_restart numeric;
BEGIN
  linop_db_sequence := pg_catalog.pg_get_serial_sequence(${literal(table)}, ${literal(column)})::regclass;
  IF linop_db_sequence IS NULL THEN
    RAISE EXCEPTION 'Unable to find the owned identity/serial sequence';
  END IF;
  IF EXISTS (SELECT 1 FROM pg_catalog.pg_class c JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace WHERE n.nspname = 'pg_catalog' AND c.relname = 'pg_sequence') THEN
    EXECUTE 'SELECT seqincrement FROM pg_catalog.pg_sequence WHERE seqrelid = $1' INTO linop_db_increment USING linop_db_sequence;
  ELSE
    EXECUTE 'SELECT increment_by FROM ' || linop_db_sequence::text INTO linop_db_increment;
  END IF;
  IF linop_db_increment IS NULL OR linop_db_increment = 0 THEN
    RAISE EXCEPTION 'Unable to read the identity/serial sequence increment';
  END IF;
  EXECUTE 'ALTER SEQUENCE ' || linop_db_sequence::text || ' INCREMENT BY ' || linop_db_increment::text;
  EXECUTE 'SELECT last_value, is_called FROM ' || linop_db_sequence::text INTO linop_db_last, linop_db_called;
  EXECUTE 'SELECT ' || CASE WHEN linop_db_increment > 0 THEN 'MAX' ELSE 'MIN' END || ${literal(`(${quotedColumn}::numeric) FROM ${table}`)} INTO linop_db_edge;
  linop_db_next := linop_db_last + CASE WHEN linop_db_called THEN linop_db_increment ELSE 0 END;
  IF (linop_db_increment > 0 AND linop_db_edge >= linop_db_next) OR (linop_db_increment < 0 AND linop_db_edge <= linop_db_next) THEN
    linop_db_restart := linop_db_next + FLOOR((linop_db_edge - linop_db_next) / linop_db_increment) * linop_db_increment;
    EXECUTE 'ALTER SEQUENCE ' || linop_db_sequence::text || ' RESTART WITH ' || linop_db_restart::text;
    PERFORM pg_catalog.nextval(linop_db_sequence);
  END IF;
END;`;
  let delimiter = "$linop_db_transfer$";
  while (body.includes(delimiter)) delimiter = delimiter.slice(0, -1) + "_$";
  return `DO ${delimiter}\n${body}\n${delimiter}`;
}
