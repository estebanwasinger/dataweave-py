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
declare const execute: (request: ExecuteRequest | string) => JsonValue;

export {analyze, execute};
export type {AnalyzeRequest, AnalyzeResult, ExecuteRequest, JsonValue};
