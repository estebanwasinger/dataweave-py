//! Continuations for source-mapped grammar beyond the initial debugger subset.
use super::*;
#[derive(Clone)]
pub(super) enum Expression {
    Wait(Expr, Expr),
    Join(String, Vec<Expr>, Vec<Expr>),
    Object(Rc<Vec<Entry>>),
    Format(Vec<Part>),
    Match(Expr, Rc<Vec<MatchCase>>),
    Selector(Expr, u8, Expr),
    RawSelect(Expr, String),
    Invoke(Expr, Vec<Expr>),
    Replace(Expr, Expr, Expr),
}
#[derive(Clone)]
pub(super) enum Key {
    Static(String),
    Computed(Expr),
}
#[derive(Clone)]
pub(super) struct Entry {
    pub key: Option<Key>,
    pub value: Expr,
    pub guard: Option<Expr>,
    pub source: String,
}
#[derive(Clone)]
pub(super) enum Part {
    Literal(String),
    Value(Expr),
}
#[derive(Clone)]
pub(super) enum Pattern {
    Any(Option<String>),
    Type(Option<String>, String),
    Equals(Option<String>, Expr),
    Regex(String, Expr),
}
impl Pattern {
    pub fn binding(&self) -> Option<&str> {
        match self {
            Self::Any(n) | Self::Type(n, _) | Self::Equals(n, _) => n.as_deref(),
            Self::Regex(n, _) => Some(n),
        }
    }
}
#[derive(Clone)]
pub(super) struct MatchCase {
    pub pattern: Pattern,
    pub guard: Option<Expr>,
    pub body: Expr,
}
impl Expression {
    pub fn visit(&self, f: &mut impl FnMut(&Expr)) {
        match self {
            Self::Wait(a, b) => {
                f(a);
                f(b);
            }
            Self::Join(_, inputs, callbacks) => {
                for e in inputs.iter().chain(callbacks) {
                    f(e);
                }
            }

            Self::Object(entries) => {
                for e in entries.iter() {
                    if let Some(g) = &e.guard {
                        f(g)
                    }
                    if let Some(Key::Computed(k)) = &e.key {
                        f(k)
                    }
                    f(&e.value)
                }
            }
            Self::Format(parts) => {
                for p in parts {
                    if let Part::Value(e) = p {
                        f(e)
                    }
                }
            }
            Self::Match(v, cases) => {
                f(v);
                for c in cases.iter() {
                    match &c.pattern {
                        Pattern::Equals(_, e) | Pattern::Regex(_, e) => f(e),
                        _ => (),
                    }
                    if let Some(g) = &c.guard {
                        f(g)
                    }
                    f(&c.body)
                }
            }
            Self::Selector(a, _, b) => {
                f(a);
                f(b)
            }
            Self::RawSelect(a, _) => f(a),
            Self::Invoke(a, args) => {
                f(a);
                for a in args {
                    f(a)
                }
            }
            Self::Replace(a, b, c) => {
                f(a);
                f(b);
                f(c)
            }
        }
    }
    pub fn pure(&self, f: &mut impl FnMut(&Expr) -> bool) -> bool {
        if !matches!(
            self,
            Self::Object(_) | Self::Format(_) | Self::RawSelect(..) | Self::Replace(..)
        ) {
            return false;
        }
        let mut pure = true;
        self.visit(&mut |e| pure &= f(e));
        pure
    }
}
pub(super) struct ObjectCursor {
    entries: Rc<Vec<Entry>>,
    index: usize,
    env: Env,
    depth: usize,
    map: Map,
    pairs: Vec<(String, Value)>,
    duplicate: bool,
}
pub(super) struct FormatCursor {
    parts: Vec<Part>,
    index: usize,
    text: String,
    env: Env,
    depth: usize,
}
pub(super) struct MatchCursor {
    value: Value,
    cases: Rc<Vec<MatchCase>>,
    index: usize,
    env: Env,
    depth: usize,
}
pub(super) struct ReplaceCursor {
    parts: std::vec::IntoIter<(String, Vec<Value>)>,
    tail: String,
    text: String,
    function: DebugValue,
    env: Env,
    depth: usize,
    range: SourceRange,
    index: usize,
}
enum FilterInput {
    Array(std::vec::IntoIter<Value>),
    Object(std::vec::IntoIter<(String, Value)>),
    Scalar(Option<Value>),
}
enum FilterOutput {
    Array(Vec<Value>),
    Object(Map),
    Scalar(Option<Value>),
}
pub(super) struct FilterCursor {
    input: FilterInput,
    output: FilterOutput,
    xml: Option<(String, std::vec::IntoIter<Value>, usize)>,
    index: usize,
    predicate: Expr,
    env: Env,
    depth: usize,
    bytes: usize,
}
pub(super) struct Candidate {
    value: Value,
    key: Value,
    object_key: Option<String>,
}
pub(super) enum Task {
    ObjectNext(ObjectCursor),
    ObjectGuard(ObjectCursor),
    ObjectKey(ObjectCursor),
    ObjectValue(ObjectCursor, Option<String>),
    FormatNext(FormatCursor),
    FormatValue(FormatCursor),
    MatchStart(Rc<Vec<MatchCase>>, Env, usize),
    MatchNext(MatchCursor),
    MatchExpected(MatchCursor),
    MatchGuard(MatchCursor, Env),
    Selector(u8, Expr, Env, usize),
    DynamicSelector(u8),
    RawSelect(String, Env),
    FilterNext(FilterCursor),
    FilterResult(FilterCursor, Candidate),
    Invoke(usize, Env, usize, SourceRange),
    Replace(Env, usize, SourceRange),
    ReplaceNext(ReplaceCursor),
    ReplaceValue(ReplaceCursor),
}
impl DebugSession {
    fn push_extra(&mut self, task: Task) {
        self.tasks.push(super::Task::Extra(task));
    }
    pub(super) fn start_extra(
        &mut self,
        e: Expression,
        env: Env,
        depth: usize,
        range: SourceRange,
    ) -> Result<(), DwError> {
        match e {
            Expression::Wait(input, _) => self.eval(input, env, depth + 1),
            Expression::Join(op, inputs, callbacks) => {
                self.tasks
                    .push(super::Task::Higher(super::collections::Task::JoinStart(
                        op,
                        callbacks,
                        env.clone(),
                        range,
                    )));
                self.tasks.push(super::Task::RestoreLocation(range));
                for input in inputs.into_iter().rev() {
                    self.eval(input, env.clone(), depth + 1);
                }
            }
            Expression::Object(entries) => self.push_extra(Task::ObjectNext(ObjectCursor {
                entries,
                index: 0,
                env,
                depth,
                map: Map::new(),
                pairs: vec![],
                duplicate: false,
            })),
            Expression::Format(parts) => self.push_extra(Task::FormatNext(FormatCursor {
                parts,
                index: 0,
                text: String::new(),
                env,
                depth,
            })),
            Expression::Match(value, cases) => {
                self.push_extra(Task::MatchStart(cases, env.clone(), depth));
                self.eval(value, env, depth + 1);
            }
            Expression::Selector(base, op, predicate) => {
                self.push_extra(Task::Selector(op, predicate, env.clone(), depth));
                self.eval(base, env, depth + 1);
            }
            Expression::RawSelect(base, path) => {
                self.push_extra(Task::RawSelect(path, env.clone()));
                self.eval(base, env, depth + 1);
            }
            Expression::Invoke(target, args) => {
                self.push_extra(Task::Invoke(args.len(), env.clone(), depth, range));
                for arg in args.into_iter().rev() {
                    self.eval(arg, env.clone(), depth + 1)
                }
                self.eval(target, env, depth + 1);
            }
            Expression::Replace(input, target, replacement) => {
                self.push_extra(Task::Replace(env.clone(), depth, range));
                self.eval(replacement, env.clone(), depth + 1);
                self.eval(target, env.clone(), depth + 1);
                self.eval(input, env, depth + 1);
            }
        }
        Ok(())
    }
    fn restore_extra_scope(&mut self, env: Env) {
        if let Some(frame) = self.frames.last() {
            self.tasks
                .push(super::Task::ExitBlock(env, frame.declared.clone()));
        }
    }
    fn object_entry(&mut self, cursor: ObjectCursor) {
        let e = &cursor.entries[cursor.index];
        match &e.key {
            Some(Key::Static(key)) => {
                let key = key.clone();
                let v = e.value.clone();
                let env = cursor.env.clone();
                let depth = cursor.depth;
                self.push_extra(Task::ObjectValue(cursor, Some(key)));
                self.eval(v, env, depth + 1)
            }
            Some(Key::Computed(key)) => {
                let key = key.clone();
                let env = cursor.env.clone();
                let depth = cursor.depth;
                self.push_extra(Task::ObjectKey(cursor));
                self.eval(key, env, depth + 1)
            }
            None => {
                let v = e.value.clone();
                let env = cursor.env.clone();
                let depth = cursor.depth;
                self.push_extra(Task::ObjectValue(cursor, None));
                self.eval(v, env, depth + 1)
            }
        }
    }
    fn match_body(&mut self, cursor: MatchCursor, bound: Option<Value>) -> Result<(), DwError> {
        let c = &cursor.cases[cursor.index];
        let mut bindings = (*cursor.env).clone();
        if let Some(name) = c.pattern.binding() {
            self.declare_local(name, cursor.depth + 1);
            bindings.insert(
                name.into(),
                data(bound.unwrap_or_else(|| cursor.value.clone())),
            );
        }
        let scoped = Rc::new(bindings);
        let depth = cursor.depth;
        if let Some(guard) = c.guard.clone() {
            self.push_extra(Task::MatchGuard(cursor, scoped.clone()));
            self.eval(guard, scoped, depth + 1)
        } else {
            self.eval(c.body.clone(), scoped, depth + 1)
        }
        Ok(())
    }
    fn invoke_debug(
        &mut self,
        function: DebugValue,
        args: Vec<DebugValue>,
        env: Env,
        depth: usize,
        range: SourceRange,
    ) -> Result<(), DwError> {
        if let Some(frame) = self.frames.last_mut() {
            frame.range = range;
        }
        if let DebugValue::Data(value) = &function {
            if let Some(name) = value.get("__function_ref").and_then(Value::as_str) {
                return self.call_value(name, args, env, depth, range);
            }
            return Err(error("computed call target is not a function"));
        }
        let mut bindings = (*env).clone();
        bindings.insert("__debug_call_target".into(), function);
        self.call_value("__debug_call_target", args, Rc::new(bindings), depth, range)
    }
    pub(super) fn tick_extra(&mut self, task: Task) -> Result<(), DwError> {
        match task {
            Task::ObjectNext(cursor) => {
                if cursor.index == cursor.entries.len() {
                    let value = if cursor.duplicate {
                        crate::selectors::duplicate_object_value(cursor.pairs)
                    } else {
                        Value::Object(cursor.map)
                    };
                    self.check_size(&value)?;
                    self.values.push(data(value));
                } else if let Some(guard) = cursor.entries[cursor.index].guard.clone() {
                    let env = cursor.env.clone();
                    let depth = cursor.depth;
                    self.push_extra(Task::ObjectGuard(cursor));
                    self.eval(guard, env, depth + 1)
                } else {
                    self.object_entry(cursor)
                }
            }
            Task::ObjectGuard(mut cursor) => {
                if crate::is_truthy(&self.pop_data()?) {
                    self.object_entry(cursor)
                } else {
                    cursor.index += 1;
                    self.push_extra(Task::ObjectNext(cursor))
                }
            }
            Task::ObjectKey(cursor) => {
                let key = crate::as_dataweave_string(&self.pop_data()?);
                let v = cursor.entries[cursor.index].value.clone();
                let env = cursor.env.clone();
                let depth = cursor.depth;
                self.push_extra(Task::ObjectValue(cursor, Some(key)));
                self.eval(v, env, depth + 1)
            }
            Task::ObjectValue(mut cursor, key) => {
                let value = self.pop_data()?;
                if let Some(key) = key {
                    cursor.duplicate |= cursor.map.insert(key.clone(), value.clone()).is_some();
                    cursor.pairs.push((key, value));
                } else if !crate::evaluator::extend_object_spread(
                    &mut cursor.map,
                    &mut cursor.pairs,
                    &mut cursor.duplicate,
                    value,
                )? {
                    return Err(DwError::Parse(format!(
                        "object entry missing ':' in {}",
                        cursor.entries[cursor.index].source
                    )));
                }
                cursor.index += 1;
                self.push_extra(Task::ObjectNext(cursor))
            }
            Task::FormatNext(mut cursor) => {
                if cursor.index == cursor.parts.len() {
                    self.values.push(data(Value::String(cursor.text)));
                } else {
                    match cursor.parts[cursor.index].clone() {
                        Part::Literal(s) => {
                            let value = crate::literals::evaluate_string_literal_scoped(
                                &s,
                                self.payload.as_ref(),
                                &native(&cursor.env),
                            )?
                            .unwrap();
                            cursor.text.push_str(&crate::as_dataweave_string(&value));
                            cursor.index += 1;
                            self.check_size(&Value::String(cursor.text.clone()))?;
                            self.push_extra(Task::FormatNext(cursor))
                        }
                        Part::Value(e) => {
                            let env = cursor.env.clone();
                            let depth = cursor.depth;
                            self.push_extra(Task::FormatValue(cursor));
                            self.eval(e, env, depth + 1)
                        }
                    }
                }
            }
            Task::FormatValue(mut cursor) => {
                cursor
                    .text
                    .push_str(&crate::as_dataweave_string(&self.pop_data()?));
                cursor.index += 1;
                self.check_size(&Value::String(cursor.text.clone()))?;
                self.push_extra(Task::FormatNext(cursor))
            }
            Task::MatchStart(cases, env, depth) => {
                let value = self.pop_data()?;
                self.restore_extra_scope(env.clone());
                self.push_extra(Task::MatchNext(MatchCursor {
                    value,
                    cases,
                    index: 0,
                    env,
                    depth,
                }));
            }
            Task::MatchNext(mut cursor) => {
                let Some(case) = cursor.cases.get(cursor.index) else {
                    self.values.push(data(Value::Null));
                    return Ok(());
                };
                match &case.pattern {
                    Pattern::Any(_) => self.match_body(cursor, None)?,
                    Pattern::Type(_, t) => {
                        if crate::evaluate_type_check(&cursor.value, t, &native(&cursor.env)) {
                            self.match_body(cursor, None)?
                        } else {
                            cursor.index += 1;
                            self.push_extra(Task::MatchNext(cursor))
                        }
                    }
                    Pattern::Equals(_, e) | Pattern::Regex(_, e) => {
                        let e = e.clone();
                        let env = cursor.env.clone();
                        let depth = cursor.depth;
                        self.push_extra(Task::MatchExpected(cursor));
                        self.eval(e, env, depth + 1)
                    }
                }
            }
            Task::MatchExpected(mut cursor) => {
                let expected = self.pop_data()?;
                let bound = match &cursor.cases[cursor.index].pattern {
                    Pattern::Regex(_, _) => {
                        crate::matches::regex_match_captures(&cursor.value, &expected)?
                            .map(Value::Array)
                    }
                    _ => {
                        if cursor.value == expected {
                            Some(cursor.value.clone())
                        } else {
                            None
                        }
                    }
                };
                if let Some(bound) = bound {
                    self.match_body(cursor, Some(bound))?
                } else {
                    cursor.index += 1;
                    self.push_extra(Task::MatchNext(cursor))
                }
            }
            Task::MatchGuard(mut cursor, scoped) => {
                if crate::is_truthy(&self.pop_data()?) {
                    self.eval(
                        cursor.cases[cursor.index].body.clone(),
                        scoped,
                        cursor.depth + 1,
                    )
                } else {
                    cursor.index += 1;
                    self.push_extra(Task::MatchNext(cursor))
                }
            }
            Task::RawSelect(path, env) => {
                let value = self.pop_data()?;
                let mut locals = native(&env);
                locals.insert("_debug_value".into(), value);
                let value = crate::evaluator::evaluate_index_base(
                    &format!("_debug_value{path}"),
                    self.payload.as_ref(),
                    &locals,
                )?;
                self.values.push(data(value));
            }
            Task::Selector(op, predicate, env, depth) => {
                let base = self.pop_data()?;
                if op == b'?' {
                    self.restore_extra_scope(env.clone());
                    let (input, output) = match base {
                        Value::Array(a) => (
                            FilterInput::Array(a.into_iter()),
                            FilterOutput::Array(vec![]),
                        ),
                        Value::Object(o) => (
                            FilterInput::Object(o.into_iter().collect::<Vec<_>>().into_iter()),
                            FilterOutput::Object(Map::new()),
                        ),
                        Value::Null => (FilterInput::Scalar(None), FilterOutput::Scalar(None)),
                        v => (FilterInput::Scalar(Some(v)), FilterOutput::Scalar(None)),
                    };
                    self.push_extra(Task::FilterNext(FilterCursor {
                        input,
                        output,
                        xml: None,
                        index: 0,
                        predicate,
                        env,
                        depth,
                        bytes: 0,
                    }));
                } else {
                    self.values.push(data(base));
                    self.push_extra(Task::DynamicSelector(op));
                    self.eval(predicate, env, depth + 1)
                }
            }
            Task::DynamicSelector(op) => {
                let key = crate::as_dataweave_string(&self.pop_data()?);
                let base = self.pop_data()?;
                let value = if op == b'*' {
                    crate::selectors::select_dynamic_multi_property(&base, &key)
                } else {
                    crate::selectors::select_key_value_pairs(&base, &key)
                };
                self.check_size(&value)?;
                self.values.push(data(value));
            }
            Task::FilterNext(mut cursor) => {
                let candidate = if let Some((key, items, index)) = &mut cursor.xml {
                    if let Some(value) = items.next() {
                        let value = crate::selectors::collapse_xml_like_value(&value);
                        let candidate = Candidate {
                            value,
                            key: Value::Number((*index).into()),
                            object_key: Some(key.clone()),
                        };
                        *index += 1;
                        Some(candidate)
                    } else {
                        cursor.xml = None;
                        self.push_extra(Task::FilterNext(cursor));
                        return Ok(());
                    }
                } else {
                    match &mut cursor.input {
                        FilterInput::Array(items) => items.next().map(|value| {
                            let candidate = Candidate {
                                value,
                                key: Value::Number(cursor.index.into()),
                                object_key: None,
                            };
                            cursor.index += 1;
                            candidate
                        }),
                        FilterInput::Scalar(value) => value.take().map(|value| Candidate {
                            value,
                            key: Value::Number(0.into()),
                            object_key: None,
                        }),
                        FilterInput::Object(items) => {
                            if let Some((key, value)) = items.next() {
                                if let Some(items) = crate::xml::xml_list_items(&value) {
                                    cursor.xml = Some((key, items.clone().into_iter(), 0));
                                    self.push_extra(Task::FilterNext(cursor));
                                    return Ok(());
                                }
                                Some(Candidate {
                                    value,
                                    key: Value::String(key.clone()),
                                    object_key: Some(key),
                                })
                            } else {
                                None
                            }
                        }
                    }
                };
                if let Some(candidate) = candidate {
                    let mut bindings = (*cursor.env).clone();
                    bindings.insert("$".into(), data(candidate.value.clone()));
                    bindings.insert("$$".into(), data(candidate.key.clone()));
                    self.declare_local("$", cursor.depth + 1);
                    self.declare_local("$$", cursor.depth + 1);
                    let predicate = cursor.predicate.clone();
                    let depth = cursor.depth;
                    self.push_extra(Task::FilterResult(cursor, candidate));
                    self.eval(predicate, Rc::new(bindings), depth + 1)
                } else {
                    let result = match cursor.output {
                        FilterOutput::Array(a) => {
                            if a.is_empty() {
                                Value::Null
                            } else {
                                Value::Array(a)
                            }
                        }
                        FilterOutput::Object(o) => {
                            if o.is_empty() {
                                Value::Null
                            } else {
                                Value::Object(o)
                            }
                        }
                        FilterOutput::Scalar(v) => v.unwrap_or(Value::Null),
                    };
                    self.values.push(data(result));
                }
            }
            Task::FilterResult(mut cursor, candidate) => {
                if crate::is_truthy(&self.pop_data()?) {
                    cursor.bytes = cursor
                        .bytes
                        .saturating_add(estimated_size(&candidate.value) + 32);
                    if cursor.bytes > self.options.max_materialized_bytes {
                        return Err(error("materialized value limit exceeded"));
                    }
                    match &mut cursor.output {
                        FilterOutput::Array(a) => a.push(candidate.value),
                        FilterOutput::Object(o) => {
                            o.insert(candidate.object_key.unwrap(), candidate.value);
                            cursor.xml = None;
                        }
                        FilterOutput::Scalar(v) => *v = Some(candidate.value),
                    }
                }
                self.push_extra(Task::FilterNext(cursor));
            }
            Task::Invoke(count, env, depth, range) => {
                let args = self.args(count)?;
                let function = self.pop()?;
                self.invoke_debug(function, args, env, depth, range)?
            }
            Task::Replace(env, depth, range) => {
                let replacement = self.pop()?;
                let target = self.pop_data()?;
                let input = self.pop_data()?;
                if input.is_null() {
                    self.values.push(data(Value::Null));
                } else if matches!(replacement, DebugValue::Function(_))
                    || matches!(&replacement,DebugValue::Data(v) if v.get("__function_ref").is_some())
                {
                    let (parts, tail) = crate::strings::replacement_parts(&input, &target)?;
                    self.push_extra(Task::ReplaceNext(ReplaceCursor {
                        parts: parts.into_iter(),
                        tail,
                        text: String::new(),
                        function: replacement,
                        env,
                        depth,
                        range,
                        index: 0,
                    }));
                } else {
                    let value =
                        crate::strings::replace_with(&input, &target, &replacement.data()?)?;
                    self.values.push(data(value));
                }
            }
            Task::ReplaceNext(mut cursor) => {
                if let Some((prefix, captures)) = cursor.parts.next() {
                    cursor.text.push_str(&prefix);
                    let mut args = vec![
                        data(Value::Array(captures)),
                        data(Value::Number(cursor.index.into())),
                    ];
                    match &cursor.function {
                        DebugValue::Function(f) if f.lexical => {
                            args.truncate(f.params.len().min(2))
                        }
                        DebugValue::Function(f) => {
                            let count = (0..=args.len())
                                .rev()
                                .find(|count| {
                                    std::iter::once(f)
                                        .chain(f.overloads.iter())
                                        .any(|f| accepts_arguments(f, &args[..*count]))
                                })
                                .unwrap_or(args.len());
                            args.truncate(count);
                        }
                        DebugValue::Data(f) => {
                            let native_args = args
                                .iter()
                                .map(DebugValue::data)
                                .collect::<Result<Vec<_>, _>>()?;
                            let count = crate::strings::callback_argument_count(
                                f,
                                &native(&cursor.env),
                                &native_args,
                            );
                            args.truncate(count);
                        }
                    }
                    cursor.index += 1;
                    let function = cursor.function.clone();
                    let env = cursor.env.clone();
                    let depth = cursor.depth;
                    let range = cursor.range;
                    self.push_extra(Task::ReplaceValue(cursor));
                    self.invoke_debug(function, args, env, depth + 1, range)?
                } else {
                    cursor.text.push_str(&cursor.tail);
                    self.check_size(&Value::String(cursor.text.clone()))?;
                    self.values.push(data(Value::String(cursor.text)));
                }
            }
            Task::ReplaceValue(mut cursor) => {
                cursor
                    .text
                    .push_str(&crate::as_dataweave_string(&self.pop_data()?));
                self.check_size(&Value::String(cursor.text.clone()))?;
                self.push_extra(Task::ReplaceNext(cursor));
            }
        }
        Ok(())
    }
}
