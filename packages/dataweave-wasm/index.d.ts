type JsonPrimitive = boolean | null | number | string;
type JsonValue = JsonPrimitive | JsonValue[] | {[key: string]: JsonValue};

type AnalyzeRequest = {
  expression: string;
  payload?: JsonValue;
  vars?: JsonValue;
  attributes?: JsonValue;
  properties?: Record<string, string>;
};

type AnalyzeResult = {
  diagnostics: Array<{message: string; severity: "error" | "warning"}>;
  inferredType: JsonValue;
  references: string[];
  unresolvedReferences: string[];
  wildcardReferences: string[];
};

type ExecuteRequest = {
  attributes?: JsonValue;
  payload?: JsonValue;
  payload_format?: string;
  payload_format_options?: {[key: string]: JsonValue};
  properties?: Record<string, string>;
  render_output?: boolean;
  script: string;
  vars?: JsonValue;
};

/** Serialized JSON requests preserve duplicate keys in execution data. */
declare const analyze: (request: AnalyzeRequest | string) => AnalyzeResult;
/** Pass raw request JSON, or JSON payload text with payload_format, to retain duplicate keys. */
declare const execute: (request: ExecuteRequest | string, options?: {loggers?: DataWeaveLogger[]}) => JsonValue;

type LogEntry = {
  level: "debug" | "info" | "warn" | "error";
  prefix: string | null;
  message: string;
};
type DataWeaveLogger = (entry: LogEntry) => void;
type ExecutionWithLogs = {
  logs: LogEntry[];
  logs_truncated: boolean;
} & ({result: JsonValue; error?: never} | {error: string; result?: never});
/** Captures up to 1000 entries / ~1 MiB; messages are limited to 16 KiB. */
declare const executeWithLogs: (request: ExecuteRequest | string) => ExecutionWithLogs;

export {analyze, execute, executeWithLogs};
export type {AnalyzeRequest, AnalyzeResult, ExecuteRequest, JsonValue, LogEntry, DataWeaveLogger, ExecutionWithLogs};
