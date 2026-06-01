use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn run_dataweave_smoke(script_source: &str, payload_json: &str) -> Result<String, JsValue> {
    let payload: serde_json::Value =
        serde_json::from_str(payload_json).map_err(|err| JsValue::from_str(&err.to_string()))?;
    let result = dwpy_core::execute_smoke(script_source, payload)
        .map_err(|err| JsValue::from_str(&err.to_string()))?;
    serde_json::to_string(&result).map_err(|err| JsValue::from_str(&err.to_string()))
}
