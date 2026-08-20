type JsonPrimitive = boolean | null | number | string;
type JsonValue = JsonPrimitive | JsonValue[] | {[key: string]: JsonValue};

type AnalyzeRequest = {
  expression: string;
  payload?: JsonValue;
  vars?: JsonValue;
};

type AnalyzeResult = {
  diagnostics: Array<{message: string; severity: "error" | "warning"}>;
  inferredType: JsonValue;
  references: string[];
  unresolvedReferences: string[];
  wildcardReferences: string[];
};

type ExecuteRequest = {
  payload?: JsonValue;
  render_output?: boolean;
  script: string;
  vars?: JsonValue;
};

declare const analyze: (request: AnalyzeRequest) => AnalyzeResult;
declare const execute: (request: ExecuteRequest) => JsonValue;

export {analyze, execute};
export type {AnalyzeRequest, AnalyzeResult, ExecuteRequest, JsonValue};
