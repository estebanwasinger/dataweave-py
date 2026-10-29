use dwpy_core::{compile_with_output_mime_type, ExecutionOptions};
use serde_json::json;

#[test]
fn override_replaces_directive_and_writer_properties() {
    let script = "%dw 2.0\noutput application/json indent=true\n---\n{message: payload.name}";
    let compiled = compile_with_output_mime_type(script, Some("application/json")).unwrap();
    assert_eq!(compiled.execute(json!({"name": "Ada"}), None, &ExecutionOptions::default()).unwrap(), json!("{\"message\":\"Ada\"}"));
    let yaml = compile_with_output_mime_type(script, Some("application/yaml")).unwrap();
    assert!(yaml.execute(json!({"name": "Ada"}), None, &ExecutionOptions::default()).unwrap().as_str().unwrap().contains("Ada"));
    assert!(compile_with_output_mime_type(script, Some("application/unknown")).is_err());
}
