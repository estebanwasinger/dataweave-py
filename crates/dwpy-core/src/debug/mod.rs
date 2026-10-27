//! Cooperative DataWeave debugger shared by native and WebAssembly hosts.
//! Execution is held in explicit continuations; inspection never replays code.
mod collections;
mod extended;
mod lazy;
mod parser;
use crate::{DwError, DwMap as Map, DwValue as Value, ExecutionContext, ExecutionOptions};
use parser::{Declaration, Expr, Kind, Param, UpdateCase};
use serde::Serialize;
use std::{collections::BTreeMap, rc::Rc};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct SourceRange {
    pub start: usize,
    pub end: usize,
}
/// One-based lines and UTF-16 columns, suitable for editor protocols.
pub fn position(source: &str, offset: usize) -> (usize, usize) {
    let prefix = &source[..offset.min(source.len())];
    (
        prefix.bytes().filter(|b| *b == b'\n').count() + 1,
        prefix
            .rsplit('\n')
            .next()
            .unwrap_or("")
            .encode_utf16()
            .count()
            + 1,
    )
}
#[derive(Clone)]
pub enum DebugValue {
    Data(Rc<Value>),
    Function(Rc<DebugFunction>),
}
#[derive(Clone)]
pub struct DebugFunction {
    params: Vec<Param>,
    body: Expr,
    captures: Bindings,
    name: String,
    lexical: bool,
    callback: bool,
    // The first definition lives on this value; subsequent definitions retain
    // source order, matching normal runtime overload selection.
    overloads: Vec<Rc<DebugFunction>>,
}
type Bindings = BTreeMap<String, DebugValue>;
type Env = Rc<Bindings>;
impl DebugValue {
    pub fn data(&self) -> Result<Value, DwError> {
        match self {
            Self::Data(v) => Ok(v.as_ref().clone()),
            _ => Err(error("function cannot be used as execution data")),
        }
    }
    pub fn preview(&self) -> String {
        match self {
            Self::Function(f) => format!(
                "{}({}){}",
                f.name,
                f.params
                    .iter()
                    .map(|p| p.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
                if f.overloads.is_empty() {
                    String::new()
                } else {
                    format!(" +{} overloads", f.overloads.len())
                }
            ),
            Self::Data(v) => {
                let text = match v.as_ref() {
                    Value::String(s) => {
                        let preview = s.chars().take(256).collect::<String>();
                        format!(
                            "{preview:?}{}",
                            if s.chars().nth(256).is_some() {
                                "…"
                            } else {
                                ""
                            }
                        )
                    }
                    Value::Array(a) => format!("Array({})", a.len()),
                    Value::Object(_) => format!("Object({})", self.child_count()),
                    other => other.to_string(),
                };
                let mut short = text.chars().take(256).collect::<String>();
                if text.chars().count() > 256 {
                    short.push('…');
                }
                short
            }
        }
    }
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Function(_) => "Function",
            Self::Data(v) => match v.as_ref() {
                Value::Null => "Null",
                Value::Bool(_) => "Boolean",
                Value::Number(_) => "Number",
                Value::String(_) => "String",
                Value::Array(_) => "Array",
                Value::Object(_) => "Object",
                Value::Date(_) => "Date",
                Value::DateTime(_) => "DateTime",
                Value::LocalDateTime(_) => "LocalDateTime",
                Value::Time(_) => "Time",
                Value::LocalTime(_) => "LocalTime",
                Value::TimeZone(_) => "TimeZone",
                Value::Period(_) => "Period",
            },
        }
    }
    pub fn child_count(&self) -> usize {
        match self {
            Self::Data(v) => match v.as_ref() {
                Value::Array(a) => a.len(),
                Value::Object(o) => o
                    .get(crate::selectors::DW_OBJECT_PAIRS_MARKER)
                    .and_then(Value::as_array)
                    .map_or(o.len(), Vec::len),
                _ => 0,
            },
            _ => 0,
        }
    }
    pub fn children_page(&self, start: usize, count: usize) -> Vec<(String, DebugValue)> {
        let Self::Data(v) = self else {
            return vec![];
        };
        match v.as_ref() {
            Value::Array(a) => a
                .iter()
                .enumerate()
                .skip(start)
                .take(count)
                .map(|(i, v)| (i.to_string(), data(v.clone())))
                .collect(),
            Value::Object(o) => {
                if let Some(Value::Array(pairs)) = o.get(crate::selectors::DW_OBJECT_PAIRS_MARKER) {
                    pairs
                        .iter()
                        .skip(start)
                        .take(count)
                        .filter_map(|p| {
                            Some((
                                p.get("key")?.as_str()?.to_owned(),
                                data(p.get("value")?.clone()),
                            ))
                        })
                        .collect()
                } else {
                    o.iter()
                        .skip(start)
                        .take(count)
                        .map(|(k, v)| (k.clone(), data(v.clone())))
                        .collect()
                }
            }
            _ => vec![],
        }
    }
}

#[derive(Clone)]
pub struct DebugFrame {
    pub id: u64,
    pub name: String,
    pub range: SourceRange,
    env: Env,
    entry_env: Env,
    declared: Vec<String>,
    parameters: Vec<String>,
    captured: Vec<String>,
    pub last: Option<DebugValue>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DebugState {
    Ready,
    Running,
    Stopped,
    Completed,
    Terminated,
}
#[derive(Clone, Copy)]
pub enum DebugCommand {
    Continue,
    Into,
    Over,
    Out,
    Pause,
    Terminate,
}
#[derive(Clone, Debug, PartialEq)]
pub enum DebugEvent {
    Yielded,
    Stopped(&'static str),
    Completed,
    Terminated,
}
enum Mode {
    Continue,
    Into,
    Over(usize, usize),
    Out(usize),
}

pub struct DebugSession {
    program: Expr,
    payload: Rc<Value>,
    source: String,
    masked: String,
    output: Option<String>,
    options: ExecutionOptions,
    tasks: Vec<Task>,
    values: Vec<DebugValue>,
    frames: Vec<DebugFrame>,
    next_frame: u64,
    globals: Env,
    mode: Mode,
    skip_stop: bool,
    stopped_depth: usize,
    breakpoints: Vec<SourceRange>,
    pause: bool,
    lazy_range: Option<SourceRange>,
    pub state: DebugState,
    pub result: Option<Value>,
    pub exception: Option<String>,
    pub stop_on_exception: bool,
    loggers: crate::Loggers,
}
#[derive(Clone)]
struct Stage {
    op: String,
    function: Rc<DebugFunction>,
    index: usize,
}
enum Cursor {
    Text(String, usize),
    Array(std::vec::IntoIter<Value>),
    Range {
        next: i64,
        end: i64,
        step: i64,
        done: bool,
    },
}
impl Cursor {
    fn next(&mut self) -> Option<Value> {
        match self {
            Self::Text(source, offset) => {
                let ch = source[*offset..].chars().next()?;
                *offset += ch.len_utf8();
                Some(Value::String(ch.to_string()))
            }
            Self::Array(a) => a.next(),
            Self::Range {
                next,
                end,
                step,
                done,
            } => {
                if *done {
                    return None;
                }
                let v = *next;
                if v == *end {
                    *done = true;
                } else if let Some(n) = v.checked_add(*step) {
                    *next = n;
                } else {
                    *done = true;
                }
                Some(Value::Number(v.into()))
            }
        }
    }
}
struct Iteration {
    cursor: Cursor,
    stages: Vec<Stage>,
    output: Vec<Value>,
    accumulator: Option<Value>,
    reduce: bool,
    string_output: bool,
    bytes: usize,
}
struct UpdateCursor {
    value: Value,
    cases: Rc<Vec<UpdateCase>>,
    index: usize,
    env: Env,
    depth: usize,
}
enum Task {
    Lazy(lazy::Task),
    Higher(collections::Task),
    Extra(extended::Task),
    Eval(Expr, Env, usize),
    Remember,
    UpdateStart(Rc<Vec<UpdateCase>>, Env, usize),
    UpdateNext(UpdateCursor),
    UpdateGuard(UpdateCursor, Vec<crate::selectors::PathSegment>, Env),
    UpdateApply(UpdateCursor, Vec<crate::selectors::PathSegment>),
    UpdateWith,
    ExitBlock(Env, Vec<String>),
    RestoreLocation(SourceRange),
    Array(usize),
    Object(Vec<String>),
    BinaryLeft(String, Expr, Env, usize),
    Binary(String),
    Unary(String),
    Type(String, String, Env),
    If(Expr, Expr, Env, usize),
    Select(String, Env),
    Index,
    Slice,
    MeasureRange,
    NullAfterSeed,
    Call(String, usize, Env, usize, SourceRange),
    BindCall(Rc<DebugFunction>, Vec<DebugValue>, usize, Bindings, usize),
    DefaultCall(Rc<DebugFunction>, Vec<DebugValue>, usize, Bindings, usize),
    Return,
    Block(Vec<Declaration>, usize, Expr, Env, usize),
    Bind(String, Vec<Declaration>, usize, Expr, Env, usize),
    StartCollection(Vec<(String, Expr)>, Env, usize),
    CompiledSeed(Expr, Expr, Vec<(String, Expr)>, Env, usize, usize),
    CompiledInput(
        Expr,
        Expr,
        Vec<(String, Expr)>,
        Env,
        usize,
        Option<Value>,
        usize,
    ),
    CompiledRange(Vec<(String, Expr)>, Env, usize, Option<Value>),
    Seed(Iteration),
    Iterate(Iteration),
    Stage(Iteration, Value, usize),
    StageResult(Iteration, Value, usize),
}
fn error(s: impl Into<String>) -> DwError {
    DwError::Parse(format!("Debug: {}", s.into()))
}
fn data(v: Value) -> DebugValue {
    DebugValue::Data(Rc::new(v))
}
fn native(env: &Bindings) -> Map {
    env.iter()
        .filter_map(|(k, v)| {
            if let DebugValue::Data(v) = v {
                if k == "payload" {
                    None
                } else {
                    Some((k.clone(), v.as_ref().clone()))
                }
            } else {
                None
            }
        })
        .collect()
}
impl DebugSession {
    pub fn new(
        source: &str,
        payload: Value,
        context: ExecutionContext,
        options: ExecutionOptions,
    ) -> Result<Self, DwError> {
        let program = parser::parse(source)?;
        let loggers = context.loggers.clone();
        let mut bindings = context
            .into_locals()
            .into_iter()
            .map(|(k, v)| (k, data(v)))
            .collect::<Bindings>();
        let payload = Rc::new(payload);
        bindings.insert("payload".into(), DebugValue::Data(payload.clone()));
        let env = Rc::new(bindings);
        let body_range = if let Kind::Block(_, body) = &program.root.kind {
            body.range
        } else {
            program.root.range
        };
        Ok(Self {
            program: program.root.clone(),
            payload,
            source: source.into(),
            masked: program.masked,
            output: program.output,
            options,
            tasks: vec![Task::Eval(program.root.clone(), env.clone(), 0)],
            values: vec![],
            frames: vec![DebugFrame {
                id: 1,
                name: "script".into(),
                range: body_range,
                env: env.clone(),
                entry_env: env.clone(),
                declared: vec![],
                parameters: vec![],
                captured: vec![],
                last: None,
            }],
            next_frame: 2,
            globals: env,
            mode: Mode::Into,
            skip_stop: false,
            stopped_depth: 0,
            breakpoints: vec![],
            pause: false,
            lazy_range: program.lazy.then_some(body_range),
            state: DebugState::Ready,
            result: None,
            exception: None,
            stop_on_exception: false,
            loggers,
        })
    }
    pub fn source(&self) -> &str {
        &self.source
    }
    pub fn frames(&self) -> &[DebugFrame] {
        &self.frames
    }
    pub fn locations(&self) -> Vec<SourceRange> {
        fn walk(e: &Expr, ranges: &mut Vec<SourceRange>) {
            if e.stop {
                ranges.push(e.range);
            }
            match &e.kind {
                Kind::Extra(extra) => extra.visit(&mut |child| walk(child, ranges)),
                Kind::Array(a) | Kind::Call(_, a) => {
                    for e in a {
                        walk(e, ranges)
                    }
                }
                Kind::Object(a) => {
                    for (_, e) in a {
                        walk(e, ranges)
                    }
                }
                Kind::Binary(_, a, b) | Kind::Index(a, b) | Kind::Collection(_, a, b) => {
                    walk(a, ranges);
                    walk(b, ranges)
                }
                Kind::Unary(_, e) | Kind::Type(_, e, _) | Kind::Select(e, _) => walk(e, ranges),
                Kind::If(a, b, c) | Kind::Slice(a, b, c) | Kind::UpdateWith(a, b, c) => {
                    walk(a, ranges);
                    walk(b, ranges);
                    walk(c, ranges)
                }
                Kind::Update(input, cases) => {
                    walk(input, ranges);
                    for case in cases.iter() {
                        if let Some(guard) = &case.guard {
                            walk(guard, ranges);
                        }
                        walk(&case.body, ranges);
                    }
                }
                Kind::Lambda(p, e) => {
                    for p in p {
                        if let Some(e) = &p.default {
                            walk(e, ranges)
                        }
                    }
                    walk(e, ranges)
                }
                Kind::Block(d, e) => {
                    for d in d {
                        match d {
                            Declaration::Var(_, e) => walk(e, ranges),
                            Declaration::Fun(_, p, e) => {
                                for p in p {
                                    if let Some(e) = &p.default {
                                        walk(e, ranges)
                                    }
                                }
                                walk(e, ranges)
                            }
                            _ => (),
                        }
                    }
                    walk(e, ranges)
                }
                _ => (),
            }
        }
        let mut ranges = vec![];
        // Retain the source-mapped program independently of live continuations.
        walk(&self.program, &mut ranges);
        ranges.sort_by_key(|r| (r.start, r.end));
        ranges.dedup();
        ranges
    }
    pub fn set_breakpoints(&mut self, ranges: Vec<SourceRange>) {
        self.breakpoints = ranges;
    }
    pub fn command(&mut self, command: DebugCommand) -> Result<(), DwError> {
        if matches!(command, DebugCommand::Terminate) {
            self.tasks.clear();
            self.values.clear();
            self.frames.clear();
            self.state = DebugState::Terminated;
            return Ok(());
        }
        if matches!(command, DebugCommand::Pause) {
            if self.state == DebugState::Running {
                self.pause = true;
            }
            return Ok(());
        }
        if !matches!(self.state, DebugState::Ready | DebugState::Stopped) {
            return Err(error("execution is not stopped"));
        }
        if self.exception.is_some() {
            self.tasks.clear();
            self.state = DebugState::Completed;
            return Ok(());
        }
        self.mode = match command {
            DebugCommand::Into => Mode::Into,
            DebugCommand::Over => Mode::Over(self.stopped_depth, self.frames.len()),
            DebugCommand::Out => Mode::Out(self.frames.len()),
            _ => Mode::Continue,
        };
        self.state = DebugState::Running;
        Ok(())
    }
    pub fn scopes(&self, id: u64) -> Result<Vec<(String, Vec<(String, DebugValue)>)>, DwError> {
        if self.state != DebugState::Stopped {
            return Err(error("execution is not stopped"));
        }
        let frame = self
            .frames
            .iter()
            .find(|f| f.id == id)
            .ok_or_else(|| error("stale frame reference"))?;
        let mut scopes = vec![];
        for (name, keys) in [
            ("Parameters", &frame.parameters),
            ("Captured", &frame.captured),
        ] {
            let values = keys
                .iter()
                .filter_map(|k| frame.entry_env.get(k).map(|v| (k.clone(), v.clone())))
                .collect::<Vec<_>>();
            if !values.is_empty() {
                scopes.push((name.into(), values));
            }
        }
        let locals = frame
            .env
            .iter()
            .filter(|(k, _)| {
                !k.starts_with("__")
                    && ((!frame.parameters.contains(k) && !frame.captured.contains(k))
                        || frame.declared.contains(k))
                    && (frame.declared.contains(k)
                        || !self.globals.contains_key(*k)
                        || !same_binding(frame.env.get(*k), self.globals.get(*k)))
            })
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        scopes.push(("Locals".into(), locals));
        scopes.push((
            "Globals".into(),
            self.globals
                .iter()
                .filter(|(k, _)| !k.starts_with("__"))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        ));
        if let Some(value) = &frame.last {
            scopes.push(("Last result".into(), vec![("value".into(), value.clone())]));
        }
        Ok(scopes)
    }
    pub fn evaluate(&self, expression: &str, frame_id: u64) -> Result<DebugValue, DwError> {
        let frame = self
            .frames
            .iter()
            .find(|f| f.id == frame_id)
            .ok_or_else(|| error("stale frame reference"))?;
        if self.state != DebugState::Stopped {
            return Err(error("execution is not stopped"));
        }
        let root = parser::parse_watch(expression)?;
        fn validate(e: &Expr, env: &Bindings) -> bool {
            match &e.kind {
                Kind::Extra(extra) => extra.pure(&mut |child| validate(child, env)),
                Kind::Atom(s) => {
                    !s.starts_with("__") && !matches!(env.get(s), Some(DebugValue::Function(_)))
                }
                Kind::Array(a) => a.iter().all(|e| validate(e, env)),
                Kind::Object(a) => a.iter().all(|(_, e)| validate(e, env)),
                Kind::Binary(_, a, b) | Kind::Index(a, b) => validate(a, env) && validate(b, env),
                Kind::Unary(_, e) | Kind::Type(_, e, _) | Kind::Select(e, _) => validate(e, env),
                Kind::If(a, b, c) | Kind::Slice(a, b, c) => {
                    validate(a, env) && validate(b, env) && validate(c, env)
                }
                Kind::Call(name, args) => {
                    [
                        "sizeOf", "isEmpty", "keysOf", "valuesOf", "upper", "lower", "abs",
                    ]
                    .contains(&name.as_str())
                        && !matches!(env.get(name), Some(DebugValue::Function(_)))
                        && args.iter().all(|e| validate(e, env))
                }
                Kind::Block(d, e) => d.is_empty() && validate(e, env),
                _ => false,
            }
        }
        if !validate(&root, &frame.env) {
            return Err(error(
                "watch permits only data inspection and allowlisted pure builtins",
            ));
        }
        let mut watch = Self::new(
            expression,
            Value::Null,
            ExecutionContext::default(),
            ExecutionOptions {
                render_output: false,
                ..self.options
            },
        )?;
        watch.tasks = vec![Task::Eval(root, frame.env.clone(), 0)];
        watch.globals = frame.env.clone();
        watch.payload = self.payload.clone();
        watch.mode = Mode::Continue;
        watch.state = DebugState::Running;
        for _ in 0..100 {
            if watch.advance(100) == DebugEvent::Completed {
                if let Some(e) = watch.exception {
                    return Err(error(e));
                }
                return watch
                    .result
                    .map(data)
                    .ok_or_else(|| error("watch did not produce a value"));
            }
        }
        Err(error("watch evaluation budget exceeded"))
    }
    pub fn advance(&mut self, fuel: usize) -> DebugEvent {
        if self.state == DebugState::Terminated {
            return DebugEvent::Terminated;
        }
        if self.state == DebugState::Completed {
            return DebugEvent::Completed;
        }
        if self.state == DebugState::Stopped {
            return DebugEvent::Yielded;
        }
        self.state = DebugState::Running;
        let _logging = self.loggers.enter();
        for _ in 0..fuel.min(10_000) {
            if self.pause {
                self.pause = false;
                self.state = DebugState::Stopped;
                return DebugEvent::Stopped("pause");
            }
            if let Some(Task::Eval(expr, env, depth)) = self.tasks.last() {
                if let Some(frame) = self.frames.last_mut() {
                    frame.range = expr.range;
                    frame.env = env.clone();
                }
                if expr.stop && !self.skip_stop {
                    let breakpoint = self.breakpoints.contains(&expr.range);
                    let step = match self.mode {
                        Mode::Into => true,
                        Mode::Over(d, f) => {
                            self.frames.len() < f || (self.frames.len() == f && *depth <= d)
                        }
                        Mode::Out(f) => self.frames.len() < f,
                        _ => false,
                    };
                    if breakpoint || step {
                        self.stopped_depth = *depth;
                        self.skip_stop = true;
                        self.state = DebugState::Stopped;
                        return DebugEvent::Stopped(if breakpoint {
                            "breakpoint"
                        } else if self.next_frame == 2
                            && self.values.is_empty()
                            && self.frames[0].last.is_none()
                        {
                            "entry"
                        } else {
                            "step"
                        });
                    }
                }
                self.skip_stop = false;
            }
            let Some(task) = self.tasks.pop() else {
                let value = self.values.pop().unwrap_or(data(Value::Null));
                let result = value.data().and_then(|v| {
                    crate::output::render_output_value(
                        self.output.as_deref(),
                        v,
                        self.options.render_output,
                    )
                });
                match result {
                    Ok(v) => self.result = Some(v),
                    Err(e) => return self.fail(e),
                }
                self.state = DebugState::Completed;
                return DebugEvent::Completed;
            };
            if self.tasks.len() > 100_000 || self.frames.len() > 1024 {
                return self.fail(error("continuation or call depth limit exceeded"));
            }
            if let Err(e) = self.tick(task) {
                return self.fail(e);
            }
            if self.state == DebugState::Stopped {
                return DebugEvent::Stopped("step");
            }
        }
        DebugEvent::Yielded
    }
    fn fail(&mut self, e: DwError) -> DebugEvent {
        self.exception = Some(e.to_string());
        if self.stop_on_exception {
            self.state = DebugState::Stopped;
            DebugEvent::Stopped("exception")
        } else {
            self.tasks.clear();
            self.state = DebugState::Completed;
            DebugEvent::Completed
        }
    }
    fn pop(&mut self) -> Result<DebugValue, DwError> {
        self.values
            .pop()
            .ok_or_else(|| error("invalid evaluation stack"))
    }
    fn pop_data(&mut self) -> Result<Value, DwError> {
        self.pop()?.data()
    }
    fn args(&mut self, n: usize) -> Result<Vec<DebugValue>, DwError> {
        if self.values.len() < n {
            return Err(error("invalid argument stack"));
        }
        Ok(self.values.split_off(self.values.len() - n))
    }
    fn eval(&mut self, e: Expr, env: Env, depth: usize) {
        self.tasks.push(Task::Eval(e, env, depth));
    }
    fn tick(&mut self, task: Task) -> Result<(), DwError> {
        match task {
            Task::Lazy(task) => self.tick_lazy(task)?,
            Task::Higher(task) => self.tick_higher(task)?,
            Task::Extra(task) => self.tick_extra(task)?,
            Task::Eval(expr, env, depth) => {
                self.tasks.push(Task::Remember);
                match &expr.kind {
                    Kind::Extra(extra) => {
                        self.start_extra(extra.clone(), env, depth, expr.range)?
                    }
                    Kind::Atom(s) => {
                        let v = if s == "payload" {
                            DebugValue::Data(self.payload.clone())
                        } else if let Some(v) = env.get(s) {
                            v.clone()
                        } else {
                            data(crate::evaluate_expression_scoped(
                                s,
                                self.payload.as_ref(),
                                &native(&env),
                            )?)
                        };
                        self.values.push(v);
                    }
                    Kind::Array(items) => {
                        self.tasks.push(Task::Array(items.len()));
                        for e in items.iter().rev() {
                            self.eval(e.clone(), env.clone(), depth + 1);
                        }
                    }
                    Kind::Object(fields) => {
                        self.tasks.push(Task::Object(
                            fields.iter().map(|(k, _)| k.clone()).collect(),
                        ));
                        for (_, e) in fields.iter().rev() {
                            self.eval(e.clone(), env.clone(), depth + 1);
                        }
                    }
                    Kind::Binary(op, a, b) => {
                        self.tasks.push(Task::BinaryLeft(
                            op.clone(),
                            b.clone(),
                            env.clone(),
                            depth,
                        ));
                        self.eval(a.clone(), env, depth + 1);
                    }
                    Kind::Unary(op, e) => {
                        self.tasks.push(Task::Unary(op.clone()));
                        self.eval(e.clone(), env, depth + 1);
                    }
                    Kind::Type(op, e, t) => {
                        self.tasks
                            .push(Task::Type(op.clone(), t.clone(), env.clone()));
                        self.eval(e.clone(), env, depth + 1);
                    }
                    Kind::If(c, a, b) => {
                        self.tasks
                            .push(Task::If(a.clone(), b.clone(), env.clone(), depth));
                        self.eval(c.clone(), env, depth + 1);
                    }
                    Kind::Select(e, s) => {
                        self.tasks.push(Task::Select(s.clone(), env.clone()));
                        self.eval(e.clone(), env, depth + 1);
                    }
                    Kind::Index(e, i) => {
                        self.tasks.push(Task::Index);
                        self.eval(i.clone(), env.clone(), depth + 1);
                        self.eval(e.clone(), env, depth + 1);
                    }
                    Kind::Slice(base, start, end) => {
                        self.tasks.push(Task::Slice);
                        self.eval(end.clone(), env.clone(), depth + 1);
                        self.eval(start.clone(), env.clone(), depth + 1);
                        self.eval(base.clone(), env, depth + 1);
                    }
                    Kind::Update(input, cases) => {
                        self.tasks
                            .push(Task::UpdateStart(cases.clone(), env.clone(), depth));
                        self.eval(input.clone(), env, depth + 1);
                    }
                    Kind::UpdateWith(input, selector, replacement) => {
                        self.tasks.push(Task::UpdateWith);
                        self.eval(replacement.clone(), env.clone(), depth + 1);
                        self.eval(selector.clone(), env.clone(), depth + 1);
                        self.eval(input.clone(), env, depth + 1);
                    }
                    Kind::Call(name, args) => {
                        if args.len() == 1
                            && !matches!(env.get(name), Some(DebugValue::Function(_)))
                            && resolved_name(name, &env) == "sizeOf"
                        {
                            let argument = &args[0];
                            if self.masked[argument.range.start..argument.range.end]
                                .contains(" as Number")
                            {
                                self.values.push(data(Value::Number(1.into())));
                                return Ok(());
                            }
                            let range = if let Kind::Collection(op, input, _) = &argument.kind {
                                if op == "map" {
                                    input
                                } else {
                                    argument
                                }
                            } else {
                                argument
                            };
                            if let Kind::Binary(op, a, b) = &range.kind {
                                if op == "to" {
                                    self.tasks.push(Task::MeasureRange);
                                    self.eval(b.clone(), env.clone(), depth + 1);
                                    self.eval(a.clone(), env, depth + 1);
                                    return Ok(());
                                }
                            }
                        }
                        self.tasks.push(Task::Call(
                            name.clone(),
                            args.len(),
                            env.clone(),
                            depth,
                            expr.range,
                        ));
                        self.tasks.push(Task::RestoreLocation(expr.range));
                        for e in args.iter().rev() {
                            self.eval(e.clone(), env.clone(), depth + 1);
                        }
                    }
                    Kind::Lambda(params, body) => {
                        self.values
                            .push(DebugValue::Function(Rc::new(DebugFunction {
                                params: params.clone(),
                                body: body.clone(),
                                captures: (*env).clone(),
                                name: "lambda".into(),
                                lexical: true,
                                callback: false,
                                overloads: vec![],
                            })))
                    }
                    Kind::Block(decls, body) => {
                        if depth != 0 || self.frames.len() > 1 {
                            if let Some(frame) = self.frames.last() {
                                self.tasks
                                    .push(Task::ExitBlock(env.clone(), frame.declared.clone()));
                            }
                        }
                        self.tasks
                            .push(Task::Block(decls.clone(), 0, body.clone(), env, depth))
                    }
                    Kind::Collection(op, input, lambda) => {
                        if self.lazy_range == Some(expr.range) {
                            let (base, stages) = pipeline(&expr);
                            let seed = stages.last().filter(|(op, _)| op == "reduce").and_then(
                                |(_, e)| {
                                    if let Kind::Lambda(params, _) = &e.kind {
                                        params.get(1).and_then(|p| p.default.clone())
                                    } else {
                                        None
                                    }
                                },
                            );
                            let attempts = if stages.last().is_some_and(|(op, _)| {
                                ![
                                    "reduce",
                                    "some",
                                    "every",
                                    "firstWith",
                                    "indexWhere",
                                    "countBy",
                                    "sumBy",
                                ]
                                .contains(&op.as_str())
                            }) && self.options.render_output
                                && self
                                    .output
                                    .as_deref()
                                    .and_then(crate::mime::output_mime)
                                    .is_some_and(crate::mime::is_json_mime)
                                && !self.output.as_deref().unwrap_or("").contains("with binary")
                            {
                                2
                            } else {
                                1
                            };
                            if let Some(seed) = seed {
                                self.tasks.push(Task::CompiledSeed(
                                    expr.clone(),
                                    base,
                                    stages,
                                    env.clone(),
                                    depth,
                                    attempts,
                                ));
                                self.eval(seed, env, depth + 1);
                            } else {
                                self.schedule_compiled(
                                    expr.clone(),
                                    base,
                                    stages,
                                    env,
                                    depth,
                                    None,
                                    attempts,
                                );
                            }
                        } else {
                            self.tasks.push(Task::StartCollection(
                                vec![(op.clone(), lambda.clone())],
                                env.clone(),
                                depth,
                            ));
                            self.tasks.push(Task::RestoreLocation(expr.range));
                            self.eval(input.clone(), env, depth + 1);
                        }
                    }
                }
            }
            Task::UpdateStart(cases, env, depth) => {
                let value = self.pop_data()?;
                if let Some(frame) = self.frames.last() {
                    self.tasks
                        .push(Task::ExitBlock(env.clone(), frame.declared.clone()));
                }
                self.tasks.push(Task::UpdateNext(UpdateCursor {
                    value,
                    cases,
                    index: 0,
                    env,
                    depth,
                }));
            }
            Task::UpdateNext(cursor) => {
                if let Some(case) = cursor.cases.get(cursor.index) {
                    let path = crate::resolve_update_path_source(&case.path, &native(&cursor.env));
                    let path = crate::selectors::parse_path_segments(&path)?;
                    let selected = crate::select_update_path_value(&cursor.value, &path);
                    let mut bindings = (*cursor.env).clone();
                    bindings.insert(case.binding.clone(), data(selected));
                    self.declare_local(&case.binding, cursor.depth + 1);
                    let scoped = Rc::new(bindings);
                    let depth = cursor.depth + 1;
                    if let Some(guard) = case.guard.clone() {
                        self.tasks
                            .push(Task::UpdateGuard(cursor, path, scoped.clone()));
                        self.eval(guard, scoped, depth);
                    } else {
                        let body = case.body.clone();
                        self.tasks.push(Task::UpdateApply(cursor, path));
                        self.eval(body, scoped, depth);
                    }
                } else {
                    self.values.push(data(cursor.value));
                }
            }
            Task::UpdateGuard(mut cursor, path, scoped) => {
                if self.pop_data()?.as_bool().unwrap_or(false) {
                    let body = cursor.cases[cursor.index].body.clone();
                    let depth = cursor.depth + 1;
                    self.tasks.push(Task::UpdateApply(cursor, path));
                    self.eval(body, scoped, depth);
                } else {
                    cursor.index += 1;
                    self.tasks.push(Task::UpdateNext(cursor));
                }
            }
            Task::UpdateApply(mut cursor, path) => {
                let replacement = self.pop_data()?;
                cursor.value = crate::apply_update_path(&cursor.value, &path, &replacement);
                self.check_size(&cursor.value)?;
                cursor.index += 1;
                self.tasks.push(Task::UpdateNext(cursor));
            }
            Task::UpdateWith => {
                let replacement = self.pop_data()?;
                let selector = crate::parse_update_selector(&self.pop_data()?)?;
                let input = self.pop_data()?;
                let value = crate::apply_update_selector(&input, &selector, &replacement);
                self.check_size(&value)?;
                self.values.push(data(value));
            }
            Task::RestoreLocation(range) => {
                if let Some(frame) = self.frames.last_mut() {
                    frame.range = range;
                }
            }
            Task::ExitBlock(env, declared) => {
                if let Some(frame) = self.frames.last_mut() {
                    frame.env = env;
                    frame.declared = declared;
                }
            }
            Task::Remember => {
                if let (Some(frame), Some(v)) = (self.frames.last_mut(), self.values.last()) {
                    frame.last = Some(v.clone());
                }
            }
            Task::Array(n) => {
                let values = self
                    .args(n)?
                    .into_iter()
                    .map(|v| v.data())
                    .collect::<Result<Vec<_>, _>>()?;
                self.check_size(&Value::Array(values.clone()))?;
                self.values.push(data(Value::Array(values)));
            }
            Task::Object(keys) => {
                let values = self
                    .args(keys.len())?
                    .into_iter()
                    .map(|v| v.data())
                    .collect::<Result<Vec<_>, _>>()?;
                let pairs = keys.into_iter().zip(values).collect::<Vec<_>>();
                let mut object = Map::new();
                let mut duplicate = false;
                for (key, value) in &pairs {
                    duplicate |= object.insert(key.clone(), value.clone()).is_some();
                }
                let value = if duplicate {
                    crate::selectors::duplicate_object_value(pairs)
                } else {
                    Value::Object(object)
                };
                self.check_size(&value)?;
                self.values.push(data(value));
            }
            Task::BinaryLeft(op, right, env, depth) => {
                let left = self
                    .values
                    .last()
                    .ok_or_else(|| error("missing left operand"))?
                    .data()?;
                let short = match op.as_str() {
                    "and" => Some(!crate::is_truthy(&left)),
                    "or" => Some(crate::is_truthy(&left)),
                    "default" => Some(!left.is_null()),
                    _ => None,
                };
                if short == Some(true) {
                    if op != "default" {
                        self.pop()?;
                        self.values.push(data(Value::Bool(op == "or")));
                    }
                } else {
                    self.tasks.push(Task::Binary(op));
                    self.eval(right, env, depth + 1);
                }
            }
            Task::Binary(op) => {
                let b = self.pop_data()?;
                let a = self.pop_data()?;
                if op == "to" {
                    let (start, end) = crate::operators::range_bounds(&a, &b)?;
                    let count = (end as i128 - start as i128).unsigned_abs() + 1;
                    if count * 32 > self.options.max_materialized_bytes as u128 {
                        return Err(error("materialized range limit exceeded"));
                    }
                }
                let value = if op == "default" {
                    b
                } else if op == "and" || op == "or" {
                    Value::Bool(crate::is_truthy(&b))
                } else {
                    apply_binary(&op, a, b)?
                };
                self.check_size(&value)?;
                self.values.push(data(value));
            }
            Task::Unary(op) => {
                let v = self.pop_data()?;
                let mut locals = Map::new();
                locals.insert("_debug_value".into(), v);
                let v = crate::evaluate_expression_scoped(
                    &format!("{op} _debug_value"),
                    &Value::Null,
                    &locals,
                )?;
                self.values.push(data(v));
            }
            Task::Type(op, t, env) => {
                let v = self.pop_data()?;
                let mut locals = native(&env);
                locals.insert("_debug_value".into(), v);
                let v = crate::evaluate_expression_scoped(
                    &format!("_debug_value {op} {t}"),
                    self.payload.as_ref(),
                    &locals,
                )?;
                self.values.push(data(v));
            }
            Task::If(a, b, env, depth) => {
                let condition = self.pop_data()?;
                self.eval(
                    if crate::is_truthy(&condition) { a } else { b },
                    env,
                    depth + 1,
                );
            }
            Task::Select(s, env) => {
                let v = self.pop_data()?;
                let mut locals = native(&env);
                locals.insert("_debug_value".into(), v);
                let v = crate::evaluate_expression_scoped(
                    &format!("_debug_value{s}"),
                    self.payload.as_ref(),
                    &locals,
                )?;
                self.values.push(data(v));
            }
            Task::MeasureRange => {
                let right = self.pop_data()?;
                let left = self.pop_data()?;
                let mut locals = Map::new();
                locals.insert("_debug_left".into(), left);
                locals.insert("_debug_right".into(), right);
                let value = crate::calls::evaluate_function_call(
                    "sizeOf",
                    &["_debug_left to _debug_right"],
                    self.payload.as_ref(),
                    &locals,
                )?;
                self.values.push(data(value));
            }
            Task::Slice => {
                let end = self.pop_data()?;
                let start = self.pop_data()?;
                let base = self.pop_data()?;
                let len = match &base {
                    Value::Array(a) => a.len(),
                    Value::String(s) => s.chars().count(),
                    _ => 0,
                } as i128;
                let start_n = crate::operators::number_value(&start)? as i64 as i128;
                let end_n = crate::operators::number_value(&end)? as i64 as i128;
                let start_n = if start_n < 0 { start_n + len } else { start_n };
                let end_n = if end_n < 0 { end_n + len } else { end_n };
                if ((end_n - start_n).unsigned_abs() + 1) * 32
                    > self.options.max_materialized_bytes as u128
                {
                    return Err(error("materialized slice limit exceeded"));
                }
                let value = crate::operators::evaluate_index_range(&base, &start, &end)?;
                self.values.push(data(value));
            }
            Task::Index => {
                let i = self.pop_data()?;
                let v = self.pop_data()?;
                let value = crate::operators::evaluate_index_access(&v, &i)?;
                self.values.push(data(value));
            }
            Task::NullAfterSeed => {
                self.pop()?;
                self.values.push(data(Value::Null));
            }
            Task::Call(name, n, env, depth, range) => {
                let args = self.args(n)?;
                self.call_value(&name, args, env, depth, range)?;
            }
            Task::BindCall(function, args, i, mut bindings, depth) => {
                if args.len() > function.params.len() {
                    return Err(error("too many function arguments"));
                }
                if i == function.params.len() {
                    let captured = function
                        .captures
                        .keys()
                        .filter(|k| {
                            !k.starts_with("__")
                                && !same_binding(function.captures.get(*k), self.globals.get(*k))
                        })
                        .cloned()
                        .collect();
                    let env = Rc::new(bindings);
                    let id = self.next_frame;
                    self.next_frame += 1;
                    self.frames.push(DebugFrame {
                        id,
                        name: function.name.clone(),
                        range: function.body.range,
                        env: env.clone(),
                        entry_env: env.clone(),
                        declared: vec![],
                        parameters: function.params.iter().map(|p| p.name.clone()).collect(),
                        captured,
                        last: None,
                    });
                    self.tasks.push(Task::Return);
                    self.eval(function.body.clone(), env, 0);
                } else if let Some(arg) = args.get(i) {
                    let mut bound_arg = arg.clone();
                    if !function.callback {
                        if let Some(t) = &function.params[i].type_source {
                            bound_arg = coerce_debug_parameter(&function.params[i], arg)
                                .ok_or_else(|| {
                                    error(format!(
                                        "argument {} does not match {t}",
                                        function.params[i].name
                                    ))
                                })?;
                        }
                    }
                    bindings.insert(function.params[i].name.clone(), bound_arg);
                    self.tasks
                        .push(Task::BindCall(function, args, i + 1, bindings, depth));
                } else if let Some(default) = &function.params[i].default {
                    self.tasks.push(Task::DefaultCall(
                        function.clone(),
                        args,
                        i,
                        bindings.clone(),
                        depth,
                    ));
                    self.eval(default.clone(), Rc::new(bindings), depth + 1);
                } else {
                    return Err(error(format!(
                        "missing argument {}",
                        function.params[i].name
                    )));
                }
            }
            Task::DefaultCall(function, args, i, mut bindings, depth) => {
                bindings.insert(function.params[i].name.clone(), self.pop()?);
                self.tasks
                    .push(Task::BindCall(function, args, i + 1, bindings, depth));
            }
            Task::Return => {
                self.frames.pop();
                if let (Some(frame), Some(value)) = (self.frames.last_mut(), self.values.last()) {
                    frame.last = Some(value.clone());
                }
                if matches!(self.mode,Mode::Out(f) if self.frames.len()<f) {
                    self.state = DebugState::Stopped;
                }
            }
            Task::Block(decls, i, body, env, depth) => {
                if i == decls.len() {
                    self.eval(body, env, depth + 1);
                } else {
                    match decls[i].clone() {
                        Declaration::Var(name, value) => {
                            self.tasks
                                .push(Task::Bind(name, decls, i, body, env.clone(), depth));
                            self.eval(value, env, depth + 1);
                        }
                        Declaration::Fun(name, params, body_expr) => {
                            self.declare_local(&name, depth);
                            let mut bindings = (*env).clone();
                            let candidate = Rc::new(DebugFunction {
                                params,
                                body: body_expr,
                                captures: Bindings::new(),
                                name: name.clone(),
                                lexical: false,
                                callback: false,
                                overloads: vec![],
                            });
                            let function =
                                if let Some(DebugValue::Function(existing)) = bindings.get(&name) {
                                    if !existing.lexical {
                                        let mut group = existing.as_ref().clone();
                                        group.overloads.push(candidate);
                                        Rc::new(group)
                                    } else {
                                        candidate
                                    }
                                } else {
                                    candidate
                                };
                            bindings.insert(name, DebugValue::Function(function));
                            let env = Rc::new(bindings);
                            if depth == 0 && self.frames.len() == 1 {
                                self.globals = env.clone();
                            }
                            self.tasks.push(Task::Block(decls, i + 1, body, env, depth));
                        }
                        Declaration::Directive(source) => {
                            let mut locals = native(&env);
                            crate::functions::evaluate_header_declarations(
                                &source,
                                self.payload.as_ref(),
                                &mut locals,
                            )?;
                            let mut bindings = (*env).clone();
                            for (k, v) in locals {
                                if let Some(DebugValue::Data(existing)) = bindings.get(&k) {
                                    if existing.as_ref() == &v {
                                        continue;
                                    }
                                }
                                bindings.insert(k, data(v));
                            }
                            let env = Rc::new(bindings);
                            if depth == 0 && self.frames.len() == 1 {
                                self.globals = env.clone();
                            }
                            self.tasks.push(Task::Block(decls, i + 1, body, env, depth));
                        }
                    }
                }
            }
            Task::Bind(name, decls, i, body, env, depth) => {
                self.declare_local(&name, depth);
                let mut bindings = (*env).clone();
                bindings.insert(name, self.pop()?);
                let env = Rc::new(bindings);
                if depth == 0 && self.frames.len() == 1 {
                    self.globals = env.clone();
                }
                self.tasks.push(Task::Block(decls, i + 1, body, env, depth));
            }
            Task::StartCollection(stages, env, depth) => {
                let v = self.pop_data()?;
                if stages.len() == 1 && !["map", "filter", "reduce"].contains(&stages[0].0.as_str())
                {
                    let (op, lambda) = stages.into_iter().next().unwrap();
                    let range = self.frames.last().map(|f| f.range).unwrap_or(lambda.range);
                    return self.start_higher(op, v, lambda, env, range);
                }
                match v {
                    Value::Null => self.invalid_collection(stages, env, depth),
                    Value::Array(items) => self.start_iteration(
                        Cursor::Array(items.into_iter()),
                        stages,
                        env,
                        depth,
                        false,
                        None,
                    )?,
                    Value::String(text)
                        if stages.len() == 1
                            && ["reduce", "filter"].contains(&stages[0].0.as_str()) =>
                    {
                        self.start_iteration(
                            Cursor::Text(text, 0),
                            stages.clone(),
                            env,
                            depth,
                            stages[0].0 == "filter",
                            None,
                        )?
                    }
                    _ => self.invalid_collection(stages, env, depth),
                }
            }

            Task::CompiledSeed(original, base, stages, env, depth, attempts) => {
                let seed = self.pop_data()?;
                self.schedule_compiled(original, base, stages, env, depth, Some(seed), attempts);
            }
            Task::CompiledInput(original, base, stages, env, depth, seed, attempts) => {
                let value = self.pop_data()?;
                let reduce = stages.last().is_some_and(|(op, _)| op == "reduce");
                match value {
                    Value::Array(items) => self.start_iteration(
                        Cursor::Array(items.into_iter()),
                        stages,
                        env,
                        depth,
                        false,
                        Some(seed),
                    )?,
                    Value::String(text)
                        if reduce
                            || !stages.last().is_some_and(|(op, _)| {
                                [
                                    "some",
                                    "every",
                                    "firstWith",
                                    "indexWhere",
                                    "countBy",
                                    "sumBy",
                                ]
                                .contains(&op.as_str())
                            }) =>
                    {
                        self.start_iteration(
                            Cursor::Text(text, 0),
                            stages,
                            env,
                            depth,
                            false,
                            Some(seed),
                        )?
                    }
                    _ => {
                        if attempts > 1 {
                            self.schedule_compiled(
                                original,
                                base,
                                stages,
                                env,
                                depth,
                                seed,
                                attempts - 1,
                            );
                        } else {
                            self.lazy_range = None;
                            self.eval(original, env, depth);
                        }
                    }
                }
            }
            Task::CompiledRange(stages, env, depth, seed) => {
                let end = crate::operators::number_value(&self.pop_data()?)? as i64;
                let start = crate::operators::number_value(&self.pop_data()?)? as i64;
                self.start_iteration(
                    Cursor::Range {
                        next: start,
                        end,
                        step: if start <= end { 1 } else { -1 },
                        done: false,
                    },
                    stages,
                    env,
                    depth,
                    false,
                    Some(seed),
                )?;
            }
            Task::Seed(mut it) => {
                it.accumulator = Some(self.pop_data()?);
                self.tasks.push(Task::Iterate(it));
            }
            Task::Iterate(mut it) => {
                if let Some(item) = it.cursor.next() {
                    self.tasks.push(Task::Stage(it, item, 0));
                } else {
                    let v = if it.reduce {
                        it.accumulator.unwrap_or(Value::Null)
                    } else if it.string_output {
                        Value::String(
                            it.output
                                .iter()
                                .map(crate::as_dataweave_string)
                                .collect::<String>(),
                        )
                    } else {
                        Value::Array(it.output)
                    };
                    self.values.push(data(v));
                }
            }
            Task::Stage(mut it, item, index) => {
                if index == it.stages.len() {
                    it.bytes = it.bytes.saturating_add(estimated_size(&item));
                    if it.bytes > self.options.max_materialized_bytes {
                        return Err(error("materialized value limit exceeded"));
                    }
                    it.output.push(item);
                    self.tasks.push(Task::Iterate(it));
                } else {
                    let stage = &mut it.stages[index];
                    let secondary = if stage.op == "reduce" {
                        if let Some(acc) = it.accumulator.take() {
                            acc
                        } else {
                            it.accumulator = Some(item);
                            self.tasks.push(Task::Iterate(it));
                            return Ok(());
                        }
                    } else {
                        Value::Number(stage.index.into())
                    };
                    stage.index += 1;
                    let f = stage.function.clone();
                    let mut bindings = f.captures.clone();
                    bindings.insert("$".into(), data(item.clone()));
                    bindings.insert("$$".into(), data(secondary.clone()));
                    let args = if f.params.is_empty() {
                        vec![]
                    } else if f.params.len() == 1 {
                        vec![data(item.clone())]
                    } else {
                        vec![data(item.clone()), data(secondary)]
                    };
                    self.tasks.push(Task::StageResult(it, item, index));
                    self.tasks.push(Task::BindCall(f, args, 0, bindings, 0));
                }
            }
            Task::StageResult(mut it, item, index) => {
                let value = self.pop_data()?;
                match it.stages[index].op.as_str() {
                    "map" => self.tasks.push(Task::Stage(it, value, index + 1)),
                    "filter" => {
                        if crate::is_truthy(&value) {
                            self.tasks.push(Task::Stage(it, item, index + 1));
                        } else {
                            self.tasks.push(Task::Iterate(it));
                        }
                    }
                    "reduce" => {
                        it.accumulator = Some(value);
                        self.tasks.push(Task::Iterate(it));
                    }
                    _ => unreachable!(),
                }
            }
        }
        Ok(())
    }
    fn declare_local(&mut self, name: &str, depth: usize) {
        if depth != 0 || self.frames.len() > 1 {
            if let Some(frame) = self.frames.last_mut() {
                if !frame.declared.iter().any(|n| n == name) {
                    frame.declared.push(name.into());
                }
            }
        }
    }
    fn schedule_compiled(
        &mut self,
        original: Expr,
        base: Expr,
        stages: Vec<(String, Expr)>,
        env: Env,
        depth: usize,
        seed: Option<Value>,
        attempts: usize,
    ) {
        if let Kind::Binary(op, a, b) = &base.kind {
            if op == "to" {
                self.tasks
                    .push(Task::CompiledRange(stages, env.clone(), depth, seed));
                self.tasks.push(Task::RestoreLocation(original.range));
                self.eval(b.clone(), env.clone(), depth + 1);
                self.eval(a.clone(), env, depth + 1);
                return;
            }
        }
        self.tasks.push(Task::CompiledInput(
            original.clone(),
            base.clone(),
            stages,
            env.clone(),
            depth,
            seed,
            attempts,
        ));
        self.tasks.push(Task::RestoreLocation(original.range));
        self.eval(base, env, depth + 1);
    }
    fn invalid_collection(&mut self, stages: Vec<(String, Expr)>, env: Env, depth: usize) {
        let seed = stages
            .last()
            .filter(|(op, _)| op == "reduce")
            .and_then(|(_, e)| {
                if let Kind::Lambda(params, _) = &e.kind {
                    params.get(1).and_then(|p| p.default.clone())
                } else {
                    None
                }
            });
        if let Some(seed) = seed {
            self.tasks.push(Task::NullAfterSeed);
            self.eval(seed, env, depth + 1);
        } else {
            self.values.push(data(Value::Null));
        }
    }
    fn start_iteration(
        &mut self,
        cursor: Cursor,
        expressions: Vec<(String, Expr)>,
        env: Env,
        depth: usize,
        string_output: bool,
        seed_override: Option<Option<Value>>,
    ) -> Result<(), DwError> {
        if expressions
            .iter()
            .any(|(op, _)| !["map", "filter", "reduce"].contains(&op.as_str()))
        {
            return self.start_lazy(cursor, expressions, env, seed_override.flatten());
        }
        let mut stages = vec![];
        let mut seed = None;
        for (op, e) in expressions {
            let Kind::Lambda(params, body) = &e.kind else {
                return Err(error("collection callback must be a lambda"));
            };
            if op == "reduce" {
                seed = params.get(1).and_then(|p| p.default.clone());
            }
            stages.push(Stage {
                op,
                function: Rc::new(DebugFunction {
                    // Native collection callbacks bind only their supplied item/
                    // secondary slots. Only reduce's seed uses a parameter default.
                    params: params
                        .iter()
                        .take(2)
                        .map(|p| Param {
                            name: p.name.clone(),
                            type_source: None,
                            default: None,
                        })
                        .collect(),
                    body: body.clone(),
                    captures: (*env).clone(),
                    name: "lambda".into(),
                    lexical: true,
                    callback: true,
                    overloads: vec![],
                }),
                index: 0,
            });
        }
        let reduce = stages.last().is_some_and(|s| s.op == "reduce");
        let it = Iteration {
            cursor,
            stages,
            output: vec![],
            accumulator: seed_override.clone().flatten(),
            reduce,
            string_output,
            bytes: 0,
        };
        if seed_override.is_some() {
            self.tasks.push(Task::Iterate(it));
        } else if let Some(seed) = seed {
            self.tasks.push(Task::Seed(it));
            self.eval(seed, env, depth + 1);
        } else {
            self.tasks.push(Task::Iterate(it));
        }
        Ok(())
    }
    fn call_value(
        &mut self,
        name: &str,
        args: Vec<DebugValue>,
        env: Env,
        depth: usize,
        range: SourceRange,
    ) -> Result<(), DwError> {
        if let Some(DebugValue::Function(value)) = env.get(name) {
            // A reference to a named function still dispatches through
            // its current overload set, including recursive calls.
            let group = if !value.lexical {
                match env.get(&value.name) {
                    Some(DebugValue::Function(current)) if !current.lexical => current,
                    _ => value,
                }
            } else {
                value
            };
            let mut bindings = if group.lexical {
                group.captures.clone()
            } else {
                (*env).clone()
            };
            let function = if group.lexical {
                if !accepts_arguments(group, &args) && !accepts_coercible_arguments(group, &args) {
                    let call = &self.masked[range.start..range.end];
                    let sources = crate::syntax::parse_computed_call(call)
                        .or_else(|| crate::literals::parse_call_args(call))
                        .map(|(_, a)| a.join(", "))
                        .unwrap_or_default();
                    return Err(DwError::UnsupportedFeature(format!("lambda({sources})")));
                }
                group.clone()
            } else {
                bindings.insert(group.name.clone(), DebugValue::Function(group.clone()));
                let candidates = || std::iter::once(group).chain(group.overloads.iter());
                candidates()
                    .find(|function| accepts_arguments(function, &args))
                    .or_else(|| {
                        candidates().find(|function| accepts_coercible_arguments(function, &args))
                    })
                    .cloned()
                    .ok_or_else(|| {
                        let sources =
                            crate::literals::parse_call_args(&self.masked[range.start..range.end])
                                .map(|(_, args)| args.join(", "))
                                .unwrap_or_default();
                        DwError::UnsupportedFeature(format!(
                            "no matching overload for {}({sources})",
                            group.name
                        ))
                    })?
            };
            self.tasks
                .push(Task::BindCall(function, args, 0, bindings, depth));
        } else {
            let mut locals = native(&env);
            let mut names = vec![];
            for (i, arg) in args.into_iter().enumerate() {
                let key = format!("_debug_arg_{i}");
                locals.insert(key.clone(), arg.data()?);
                names.push(key);
            }
            let resolved = crate::functions::resolve_invoked_function_name(&name, &locals)
                .unwrap_or_else(|| name.to_owned());
            if parser::unsupported_builtin(&resolved)
                || crate::COLLECTION_OPERATORS.contains(&resolved.as_str())
                || [
                    "try",
                    "eval",
                    "run",
                    "readUrl",
                    "join",
                    "leftJoin",
                    "outerJoin",
                    "by",
                ]
                .contains(&resolved.as_str())
            {
                return Err(error("unsupported callback builtin"));
            }
            let sources = names.iter().map(String::as_str).collect::<Vec<_>>();
            let v = crate::calls::evaluate_function_call(
                &resolved,
                &sources,
                self.payload.as_ref(),
                &locals,
            )?;
            self.check_size(&v)?;
            self.values.push(data(v));
        }

        Ok(())
    }
    fn check_size(&self, v: &Value) -> Result<(), DwError> {
        if estimated_size(v) > self.options.max_materialized_bytes {
            Err(error("materialized value limit exceeded"))
        } else {
            Ok(())
        }
    }
}

fn estimated_size(v: &Value) -> usize {
    match v {
        Value::String(s) => s.len(),
        Value::Array(a) => a
            .iter()
            .map(estimated_size)
            .sum::<usize>()
            .saturating_add(a.len() * 32),
        Value::Object(o) => o
            .iter()
            .map(|(k, v)| k.len() + estimated_size(v) + 32)
            .sum(),
        _ => 32,
    }
}
fn apply_binary(op: &str, a: Value, b: Value) -> Result<Value, DwError> {
    let mut locals = Map::new();
    locals.insert("_debug_left".into(), a);
    locals.insert("_debug_right".into(), b);
    crate::evaluate_expression_scoped(
        &format!("_debug_left {op} _debug_right"),
        &Value::Null,
        &locals,
    )
}

fn pipeline(root: &Expr) -> (Expr, Vec<(String, Expr)>) {
    let mut base = root.clone();
    let mut stages = vec![];
    while let Kind::Collection(op, input, lambda) = &base.kind {
        stages.push((op.clone(), lambda.clone()));
        base = input.clone();
    }
    stages.reverse();
    (base, stages)
}

fn same_binding(a: Option<&DebugValue>, b: Option<&DebugValue>) -> bool {
    match (a, b) {
        (Some(DebugValue::Data(a)), Some(DebugValue::Data(b))) => Rc::ptr_eq(a, b),
        (Some(DebugValue::Function(a)), Some(DebugValue::Function(b))) => Rc::ptr_eq(a, b),
        _ => false,
    }
}

fn resolved_name(name: &str, env: &Bindings) -> String {
    let mut locals = Map::new();
    for key in [name, "__aliases"] {
        if let Some(DebugValue::Data(value)) = env.get(key) {
            locals.insert(key.into(), value.as_ref().clone());
        }
    }
    crate::functions::resolve_invoked_function_name(name, &locals).unwrap_or_else(|| name.into())
}

fn parameter_accepts(param: &Param, argument: &DebugValue) -> bool {
    let Some(t) = &param.type_source else {
        return true;
    };
    let meta = Value::Object(Map::from_iter([("type".into(), Value::String(t.clone()))]));
    match argument {
        DebugValue::Data(value) => crate::functions::parameter_accepts_value(&meta, value),
        DebugValue::Function(_) => crate::functions::parameter_accepts_value(
            &meta,
            &Value::Object(Map::from_iter([("__lambda".into(), Value::Bool(true))])),
        ),
    }
}

fn coerce_debug_parameter(param: &Param, argument: &DebugValue) -> Option<DebugValue> {
    if parameter_accepts(param, argument) {
        return Some(argument.clone());
    }
    let DebugValue::Data(value) = argument else {
        return None;
    };
    let type_source = param.type_source.as_ref()?;
    let meta = Value::Object(Map::from_iter([(
        "type".into(),
        Value::String(type_source.clone()),
    )]));
    crate::functions::coerce_parameter_value(&meta, value)
        .map(|value| DebugValue::Data(Rc::new(value)))
}

fn accepts_coercible_arguments(function: &DebugFunction, args: &[DebugValue]) -> bool {
    let required = function
        .params
        .iter()
        .filter(|p| p.default.is_none())
        .count();
    args.len() >= required
        && args.len() <= function.params.len()
        && function
            .params
            .iter()
            .zip(args)
            .all(|(param, arg)| coerce_debug_parameter(param, arg).is_some())
}
fn accepts_arguments(function: &DebugFunction, args: &[DebugValue]) -> bool {
    let required = function
        .params
        .iter()
        .filter(|p| p.default.is_none())
        .count();
    args.len() >= required
        && args.len() <= function.params.len()
        && function
            .params
            .iter()
            .zip(args)
            .all(|(param, arg)| parameter_accepts(param, arg))
}
