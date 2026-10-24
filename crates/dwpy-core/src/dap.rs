//! Transport-independent Debug Adapter Protocol. Hosts provide framing and
//! call `poll` between requests; no host I/O or threads are needed in WASM.
use crate::{
    debug::{
        position, DebugCommand, DebugEvent, DebugSession, DebugState, DebugValue, SourceRange,
    },
    DwValue, ExecutionContext, ExecutionOptions,
};
use serde_json::{json, Value as Json};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

enum Reference {
    Scope(Vec<(String, DebugValue)>),
    Value(DebugValue),
}
#[derive(Default)]
struct Logs {
    entries: Vec<crate::LogEntry>,
    count: usize,
    bytes: usize,
    truncated: bool,
    reported: bool,
}
/// One adapter owns one live execution. Multiple adapters can coexist.
pub struct DebugAdapter {
    session: Option<DebugSession>,
    seq: u64,
    next_ref: u64,
    refs: BTreeMap<u64, Reference>,
    frames: BTreeMap<u64, u64>,
    source: Json,
    lines_one: bool,
    columns_one: bool,
    stop_entry: bool,
    configured: bool,
    ended: bool,
    exception_enabled: bool,
    logs: Arc<Mutex<Logs>>,
    breakpoints: Vec<(usize, Option<usize>)>,
}
impl Default for DebugAdapter {
    fn default() -> Self {
        Self::new()
    }
}
impl DebugAdapter {
    pub fn new() -> Self {
        Self {
            session: None,
            seq: 1,
            next_ref: 1,
            refs: BTreeMap::new(),
            frames: BTreeMap::new(),
            source: json!({"name":"script.dwl","sourceReference":1}),
            lines_one: true,
            columns_one: true,
            stop_entry: true,
            configured: false,
            ended: false,
            exception_enabled: false,
            logs: Arc::new(Mutex::new(Logs::default())),
            breakpoints: vec![],
        }
    }
    pub fn is_running(&self) -> bool {
        !self.ended
            && self.configured
            && self
                .session
                .as_ref()
                .is_some_and(|s| matches!(s.state, DebugState::Running | DebugState::Completed))
    }
    fn message(&mut self, mut v: Json) -> Json {
        v["seq"] = json!(self.seq);
        self.seq += 1;
        v
    }
    fn event(&mut self, name: &str, body: Json) -> Json {
        self.message(json!({"type":"event","event":name,"body":body}))
    }
    fn reference(&mut self, value: Reference) -> u64 {
        let id = self.next_ref;
        self.next_ref += 1;
        self.refs.insert(id, value);
        id
    }
    fn session(&self) -> Result<&DebugSession, String> {
        self.session
            .as_ref()
            .ok_or_else(|| "No debug session has been launched".into())
    }
    fn session_mut(&mut self) -> Result<&mut DebugSession, String> {
        self.session
            .as_mut()
            .ok_or_else(|| "No debug session has been launched".into())
    }
    fn invalidate(&mut self) {
        self.refs.clear();
        self.frames.clear();
    }
    /// Return the response followed by any events caused by a request.
    pub fn dispatch(&mut self, request: Json) -> Vec<Json> {
        let command = request
            .get("command")
            .and_then(Json::as_str)
            .unwrap_or("")
            .to_string();
        let args = request.get("arguments").cloned().unwrap_or(json!({}));
        let mut events = vec![];
        let result = if request.get("type").and_then(Json::as_str) != Some("request") {
            Err("Expected a DAP request".into())
        } else {
            self.handle(&command, &args, &mut events)
        };
        let response = match result {
            Ok(body) => {
                json!({"type":"response","request_seq":request.get("seq").cloned().unwrap_or(json!(0)),"command":command,"success":true,"body":body})
            }
            Err(message) => {
                json!({"type":"response","request_seq":request.get("seq").cloned().unwrap_or(json!(0)),"command":command,"success":false,"message":message})
            }
        };
        let mut messages = vec![self.message(response)];
        for (event, body) in events {
            messages.push(self.event(&event, body));
        }
        messages
    }
    fn handle(
        &mut self,
        command: &str,
        args: &Json,
        events: &mut Vec<(String, Json)>,
    ) -> Result<Json, String> {
        match command {
            "initialize" => {
                self.lines_one = args
                    .get("linesStartAt1")
                    .and_then(Json::as_bool)
                    .unwrap_or(true);
                self.columns_one = args
                    .get("columnsStartAt1")
                    .and_then(Json::as_bool)
                    .unwrap_or(true);
                Ok(
                    json!({"supportsConfigurationDoneRequest":true,"supportsTerminateRequest":true,"supportsEvaluateForHovers":true,"supportsBreakpointLocationsRequest":true,"supportsExceptionInfoRequest":true,"exceptionBreakpointFilters":[{"filter":"uncaught","label":"Uncaught runtime errors","default":false}]}),
                )
            }
            "launch" => {
                if self.session.is_some() {
                    return Err("Disconnect the current session before launching another".into());
                }
                let source = args.get("source").and_then(Json::as_str).ok_or(
                    "launch requires inline 'source'; the native host also accepts 'program'",
                )?;
                let typed = args.get("typed_transport").and_then(Json::as_bool) == Some(true);
                let decode = |v: Json| {
                    if typed {
                        DwValue::from_transport_json(v).map_err(|e| e.to_string())
                    } else {
                        Ok(DwValue::from(v))
                    }
                };
                let payload = decode(args.get("payload").cloned().unwrap_or(Json::Null))?;
                let payload = crate::parse_native_payload_format_with_options(
                    payload,
                    args.get("payload_format").and_then(Json::as_str),
                    args.get("payload_format_options")
                        .cloned()
                        .map(DwValue::from)
                        .as_ref(),
                )
                .map_err(|e| e.to_string())?;
                let vars = args.get("vars").cloned().map(decode).transpose()?;
                let attributes = args.get("attributes").cloned().map(decode).transpose()?;
                let properties = args
                    .get("properties")
                    .cloned()
                    .map(serde_json::from_value::<BTreeMap<String, String>>)
                    .transpose()
                    .map_err(|e| e.to_string())?;
                let mut context = ExecutionContext::from_values(vars, attributes, properties)
                    .map_err(|e| e.to_string())?;
                self.logs = Arc::new(Mutex::new(Logs::default()));
                let logs = self.logs.clone();
                context.loggers.push(move |entry: &crate::LogEntry| {
                    let mut logs = logs.lock().unwrap();
                    let bytes = entry.message.len() + entry.prefix.as_ref().map_or(0, String::len);
                    if logs.count >= 1000 || logs.bytes + bytes > 1024 * 1024 {
                        logs.truncated = true;
                        return;
                    }
                    logs.count += 1;
                    logs.bytes += bytes;
                    logs.entries.push(entry.clone());
                });
                let options = ExecutionOptions {
                    render_output: args
                        .get("render_output")
                        .and_then(Json::as_bool)
                        .unwrap_or(true),
                    ..ExecutionOptions::default()
                };
                let mut session = DebugSession::new(source, payload, context, options)
                    .map_err(|e| e.to_string())?;
                session.stop_on_exception = self.exception_enabled;
                self.stop_entry = args
                    .get("stopOnEntry")
                    .and_then(Json::as_bool)
                    .unwrap_or(true);
                self.ended = false;
                self.configured = false;
                let path = args
                    .get("sourcePath")
                    .or_else(|| args.get("program"))
                    .and_then(Json::as_str);
                self.source = if let Some(path) = path {
                    json!({"name":path.rsplit(['/', '\\']).next().unwrap_or("script.dwl"),"path":path,"sourceReference":1})
                } else {
                    json!({"name":"script.dwl","sourceReference":1})
                };
                self.session = Some(session);
                self.invalidate();
                events.push(("initialized".into(), json!({})));
                Ok(json!({}))
            }
            "configurationDone" => {
                if self.configured {
                    return Err("Session is already configured".into());
                }
                let cmd = if self.stop_entry {
                    DebugCommand::Into
                } else {
                    DebugCommand::Continue
                };
                self.session_mut()?
                    .command(cmd)
                    .map_err(|e| e.to_string())?;
                self.configured = true;
                Ok(json!({}))
            }
            "setBreakpoints" => {
                self.session()?;
                if !self.matches_source(args.get("source")) {
                    return Ok(
                        json!({"breakpoints":args.get("breakpoints").and_then(Json::as_array).unwrap_or(&vec![]).iter().map(|_|json!({"verified":false,"message":"Source does not belong to this session"})).collect::<Vec<_>>()}),
                    );
                }
                self.breakpoints = args
                    .get("breakpoints")
                    .and_then(Json::as_array)
                    .unwrap_or(&vec![])
                    .iter()
                    .map(|b| {
                        Ok((
                            self.from_line(integer(b, "line")? as usize),
                            b.get("column")
                                .and_then(Json::as_u64)
                                .map(|v| self.from_column(v as usize)),
                        ))
                    })
                    .collect::<Result<_, String>>()?;
                let locations = self.session()?.locations();
                let mut ranges = vec![];
                let mut result = vec![];
                for (i, (line, column)) in self.breakpoints.iter().enumerate() {
                    let found = locations
                        .iter()
                        .filter(|r| {
                            let (l, c) = position(self.session.as_ref().unwrap().source(), r.start);
                            l == *line && column.is_none_or(|col| c >= col)
                        })
                        .min_by_key(|r| (r.start, r.end));
                    if let Some(range) = found {
                        ranges.push(*range);
                        let mut b = self.location(*range);
                        b["id"] = json!(i + 1);
                        b["verified"] = json!(true);
                        b["source"] = self.source.clone();
                        result.push(b);
                    } else {
                        result.push(json!({"id":i+1,"verified":false,"line":self.to_line(*line),"message":"No executable expression on this line"}));
                    }
                }
                self.session_mut()?.set_breakpoints(ranges);
                Ok(json!({"breakpoints":result}))
            }
            "breakpointLocations" => {
                if !self.matches_source(args.get("source")) {
                    return Ok(json!({"breakpoints":[]}));
                }
                let start = self.from_line(integer(args, "line")? as usize);
                let end = args
                    .get("endLine")
                    .and_then(Json::as_u64)
                    .map(|v| self.from_line(v as usize))
                    .unwrap_or(start);
                Ok(
                    json!({"breakpoints":self.session()?.locations().into_iter().filter(|r|{let(l,_)=position(self.session.as_ref().unwrap().source(),r.start);l>=start&&l<=end}).map(|r|self.location(r)).collect::<Vec<_>>()}),
                )
            }
            "setExceptionBreakpoints" => {
                let enabled = args
                    .get("filters")
                    .and_then(Json::as_array)
                    .is_some_and(|v| v.iter().any(|f| f == "uncaught"));
                self.exception_enabled = enabled;
                if let Some(s) = &mut self.session {
                    s.stop_on_exception = enabled;
                }
                Ok(json!({"breakpoints":[]}))
            }
            "threads" => {
                self.session()?;
                Ok(json!({"threads":[{"id":1,"name":"DataWeave"}]}))
            }
            "stackTrace" => {
                self.require_stopped()?;
                let frames = self.session()?.frames().to_vec();
                let total = frames.len();
                let start = args.get("startFrame").and_then(Json::as_u64).unwrap_or(0) as usize;
                let count = args
                    .get("levels")
                    .and_then(Json::as_u64)
                    .filter(|n| *n > 0)
                    .unwrap_or(total as u64) as usize;
                let mut out = vec![];
                for f in frames.into_iter().rev().skip(start).take(count) {
                    let id = self.next_ref;
                    self.next_ref += 1;
                    self.frames.insert(id, f.id);
                    let mut item = self.location(f.range);
                    item["id"] = json!(id);
                    item["name"] = json!(f.name);
                    item["source"] = self.source.clone();
                    out.push(item);
                }
                Ok(json!({"stackFrames":out,"totalFrames":total}))
            }
            "scopes" => {
                self.require_stopped()?;
                let id = integer(args, "frameId")?;
                let frame = *self.frames.get(&id).ok_or("Stale frame reference")?;
                let scopes = self.session()?.scopes(frame).map_err(|e| e.to_string())?;
                let mut out = vec![];
                for (name, values) in scopes {
                    let count = values.len();
                    let id = self.reference(Reference::Scope(values));
                    out.push(json!({"name":name,"variablesReference":id,"namedVariables":count,"expensive":false}));
                }
                Ok(json!({"scopes":out}))
            }
            "variables" => {
                self.require_stopped()?;
                let id = integer(args, "variablesReference")?;
                let start = args.get("start").and_then(Json::as_u64).unwrap_or(0) as usize;
                let reference = self.refs.get(&id).ok_or("Stale variable reference")?;
                let (total, indexed) = match reference {
                    Reference::Scope(v) => (v.len(), false),
                    Reference::Value(v) => (v.child_count(), v.type_name() == "Array"),
                };
                if args
                    .get("filter")
                    .and_then(Json::as_str)
                    .is_some_and(|f| (f == "indexed" && !indexed) || (f == "named" && indexed))
                {
                    return Ok(json!({"variables":[]}));
                }
                let count = args
                    .get("count")
                    .and_then(Json::as_u64)
                    .filter(|n| *n > 0)
                    .map(|n| n as usize)
                    .unwrap_or_else(|| total.saturating_sub(start));
                if count > 10000 {
                    return Err(
                        "Variable response exceeds 10000 entries; request a page with start/count"
                            .into(),
                    );
                }
                let values = match reference {
                    Reference::Scope(v) => v.iter().skip(start).take(count).cloned().collect(),
                    Reference::Value(v) => v.children_page(start, count),
                };
                let mut out = vec![];
                for (name, value) in values {
                    let mut item = self.variable(value);
                    item["name"] = json!(name);
                    out.push(item);
                }
                Ok(json!({"variables":out}))
            }
            "evaluate" => {
                self.require_stopped()?;
                let id = integer(args, "frameId")?;
                let frame = *self.frames.get(&id).ok_or("Stale frame reference")?;
                let expression = args
                    .get("expression")
                    .and_then(Json::as_str)
                    .ok_or("Missing expression")?;
                let value = self
                    .session()?
                    .evaluate(expression, frame)
                    .map_err(|e| e.to_string())?;
                let mut out = self.variable(value);
                out["result"] = out["value"].take();
                out.as_object_mut().unwrap().remove("value");
                Ok(out)
            }
            "continue" | "next" | "stepIn" | "stepOut" => {
                self.require_stopped()?;
                let cmd = match command {
                    "next" => DebugCommand::Over,
                    "stepIn" => DebugCommand::Into,
                    "stepOut" => DebugCommand::Out,
                    _ => DebugCommand::Continue,
                };
                self.session_mut()?
                    .command(cmd)
                    .map_err(|e| e.to_string())?;
                self.invalidate();
                Ok(if command == "continue" {
                    json!({"allThreadsContinued":true})
                } else {
                    json!({})
                })
            }
            "pause" => {
                self.session_mut()?
                    .command(DebugCommand::Pause)
                    .map_err(|e| e.to_string())?;
                Ok(json!({}))
            }
            "source" => {
                Ok(json!({"content":self.session()?.source(),"mimeType":"text/x-dataweave"}))
            }
            "exceptionInfo" => {
                self.require_stopped()?;
                let e = self
                    .session()?
                    .exception
                    .as_ref()
                    .ok_or("No exception at this stop")?;
                Ok(json!({"exceptionId":"DataWeaveError","description":e,"breakMode":"unhandled"}))
            }
            "terminate" | "disconnect" => {
                if let Some(s) = &mut self.session {
                    s.command(DebugCommand::Terminate)
                        .map_err(|e| e.to_string())?;
                }
                self.invalidate();
                if !self.ended {
                    events.push(("terminated".into(), json!({})));
                    self.ended = true;
                }
                if command == "disconnect" {
                    self.session = None;
                }
                Ok(json!({}))
            }
            _ => Err(format!("Unsupported DAP request: {command}")),
        }
    }
    fn require_stopped(&self) -> Result<(), String> {
        if self.session()?.state == DebugState::Stopped {
            Ok(())
        } else {
            Err("notStopped".into())
        }
    }
    fn from_line(&self, v: usize) -> usize {
        if self.lines_one {
            v
        } else {
            v + 1
        }
    }
    fn from_column(&self, v: usize) -> usize {
        if self.columns_one {
            v
        } else {
            v + 1
        }
    }
    fn to_line(&self, v: usize) -> usize {
        if self.lines_one {
            v
        } else {
            v.saturating_sub(1)
        }
    }
    fn to_column(&self, v: usize) -> usize {
        if self.columns_one {
            v
        } else {
            v.saturating_sub(1)
        }
    }
    fn location(&self, r: SourceRange) -> Json {
        let source = self.session.as_ref().unwrap().source();
        let (l, c) = position(source, r.start);
        let (el, ec) = position(source, r.end);
        json!({"line":self.to_line(l),"column":self.to_column(c),"endLine":self.to_line(el),"endColumn":self.to_column(ec)})
    }
    fn matches_source(&self, source: Option<&Json>) -> bool {
        source.is_some_and(|s| {
            s.get("sourceReference") == Some(&json!(1))
                || s.get("path")
                    .is_some_and(|p| Some(p) == self.source.get("path"))
                || (self.source.get("path").is_none() && s.get("name") == self.source.get("name"))
        })
    }
    fn variable(&mut self, value: DebugValue) -> Json {
        let mut item = json!({"value":value.preview(),"type":value.type_name(),"variablesReference":0,"presentationHint":{"attributes":["readOnly"]}});
        let count = value.child_count();
        if value.type_name() == "Array" {
            item["indexedVariables"] = json!(count);
        } else if value.type_name() == "Object" {
            item["namedVariables"] = json!(count);
        }
        if count > 0 {
            item["variablesReference"] = json!(self.reference(Reference::Value(value)));
        }
        item
    }
    /// Execute bounded work, then drain DAP output/stopped/completion events.
    pub fn poll(&mut self, fuel: usize) -> Vec<Json> {
        if self.ended || !self.configured {
            return vec![];
        }
        let event = if let Some(s) = &mut self.session {
            if matches!(s.state, DebugState::Running | DebugState::Completed) {
                Some(s.advance(fuel))
            } else {
                None
            }
        } else {
            None
        };
        let mut messages = vec![];
        let (entries, truncated) = {
            let mut logs = self.logs.lock().unwrap();
            let entries = std::mem::take(&mut logs.entries);
            let truncated = logs.truncated && !logs.reported;
            logs.reported |= truncated;
            (entries, truncated)
        };
        for entry in entries {
            messages.push(self.event("output",json!({"category":if matches!(entry.level,crate::LogLevel::Error){"stderr"}else{"console"},"output":format!("{}\n",entry.message),"data":{"kind":"log","entry":entry}})));
        }
        if truncated {
            messages.push(self.event("output",json!({"category":"console","output":"Log capture limit reached\n","data":{"kind":"logsTruncated"}})));
        }
        match event {
            Some(DebugEvent::Stopped(reason)) => {
                self.invalidate();
                messages.push(self.event(
                    "stopped",
                    json!({"reason":reason,"threadId":1,"allThreadsStopped":true}),
                ));
            }
            Some(DebugEvent::Completed) => {
                let s = self.session.as_ref().unwrap();
                let error = s.exception.clone();
                let result = s.result.as_ref().map(DwValue::to_transport_json);
                let output = s
                    .result
                    .as_ref()
                    .map(|v| match v {
                        DwValue::String(s) => s.clone(),
                        other => crate::json::render_json_value(
                            other,
                            crate::json::JsonOutputOptions {
                                indent: None,
                                ensure_ascii: false,
                                sort_keys: false,
                                duplicate_key_as_array: false,
                            },
                        )
                        .unwrap_or_else(|_| other.to_string()),
                    })
                    .unwrap_or_default();
                if let Some(e) = &error {
                    messages.push(self.event(
                        "output",
                        json!({"category":"stderr","output":format!("{e}\n")}),
                    ));
                }
                if let Some(result) = result {
                    messages.push(self.event("output",json!({"category":"stdout","output":format!("{output}\n"),"data":{"kind":"result","result":result,"typed_transport":true}})));
                }
                messages
                    .push(self.event("exited", json!({"exitCode":if error.is_some(){1}else{0}})));
                messages.push(self.event("terminated", json!({})));
                self.invalidate();
                self.ended = true;
            }
            _ => (),
        }
        messages
    }
}
fn integer(args: &Json, key: &str) -> Result<u64, String> {
    args.get(key)
        .and_then(Json::as_u64)
        .ok_or_else(|| format!("Missing or invalid {key}"))
}
