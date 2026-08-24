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
  properties?: Record<string, string>;
  render_output?: boolean;
  script: string;
  vars?: JsonValue;
};

declare const analyze: (request: AnalyzeRequest) => AnalyzeResult;
declare const execute: (request: ExecuteRequest) => JsonValue;

export {analyze, execute};
export type {AnalyzeRequest, AnalyzeResult, ExecuteRequest, JsonValue};
