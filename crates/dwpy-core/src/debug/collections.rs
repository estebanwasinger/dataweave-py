//! Live callback cursors. Each task visits one item/node; callbacks run on the
//! ordinary continuation stack, so inspection never invokes a callback twice.
use super::*;
use crate::selectors::{duplicate_object_pairs, duplicate_object_value};

#[derive(Clone, Copy, PartialEq)]
enum Shape {
    Array,
    Object,
    Duplicate,
    Text,
}
pub(super) struct Item {
    value: Value,
    key: Option<String>,
    index: usize,
}
enum Input {
    Text(String, usize),
    Array(std::vec::IntoIter<Value>),
    Object(std::vec::IntoIter<(String, Value)>),
}
impl Input {
    fn next(&mut self, index: usize) -> Option<Item> {
        match self {
            Self::Text(source, offset) => {
                let ch = source[*offset..].chars().next()?;
                *offset += ch.len_utf8();
                Some(Item {
                    value: Value::String(ch.to_string()),
                    key: None,
                    index,
                })
            }
            Self::Array(a) => a.next().map(|value| Item {
                value,
                key: None,
                index,
            }),
            Self::Object(o) => o.next().map(|(key, value)| Item {
                value,
                key: Some(key),
                index,
            }),
        }
    }
}
pub(super) struct Collection {
    op: String,
    input: Input,
    shape: Shape,
    lambda: Expr,
    env: Env,
    range: SourceRange,
    index: usize,
    output: Vec<Value>,
    other: Vec<Value>,
    object: Map,
    pairs: Vec<(String, Value)>,
    text: String,
    seen: std::collections::HashSet<String>,
    sorted: Vec<(Value, usize, Value)>,
    selected: Option<(Value, Value)>,
    total: f64,
    count: usize,
    bypass: bool,
    bytes: usize,
}
#[derive(Clone)]
pub(super) struct TreeContext {
    op: String,
    lambda: Expr,
    env: Env,
    range: SourceRange,
}
pub(super) struct TreeChildren {
    context: TreeContext,
    path: Value,
    input: Input,
    shape: Shape,
    index: usize,
    output: Vec<Value>,
    object: Map,
    bytes: usize,
}
pub(super) struct Join {
    op: String,
    left: Vec<Value>,
    right: Vec<Value>,
    callbacks: Vec<Expr>,
    env: Env,
    range: SourceRange,
    l: usize,
    r: usize,
    key: String,
    matched_left: bool,
    matched_right: Vec<bool>,
    output: Vec<Value>,
    bytes: usize,
}
pub(super) enum Task {
    Next(Collection),
    Result(Collection, Item),
    TreeRoot,
    JoinStart(String, Vec<Expr>, Env, SourceRange),
    JoinNext(Join),
    JoinLeft(Join),
    JoinRight(Join),
    JoinKey(Join),
    JoinTail(Join, usize),
    TreeVisit(TreeContext, Value, Value),
    TreeNext(TreeChildren),
    TreeChild(TreeChildren, Option<String>),
    TreeLeaf(TreeContext, Value),
    NodePredicate(TreeContext, Value, Value),
}
impl Collection {
    fn finish(self) -> Result<Value, DwError> {
        let op = self.op.as_str();
        Ok(match op {
            "then" | "onNull" => self.output.into_iter().next().unwrap_or(Value::Null),
            "some" | "someEntry" | "someCharacter" => Value::Bool(false),
            "everyEntry" => Value::Bool(true),
            "every" | "everyCharacter" => Value::Bool(self.index > 0),
            "countBy" | "countCharactersBy" => Value::Number(self.count.into()),
            "sumBy" => crate::number_result(self.total)?,
            "firstWith" => Value::Null,
            "indexWhere" => Value::Number((-1).into()),
            "maxBy" | "minBy" => self.selected.map(|(_, v)| v).unwrap_or(Value::Null),
            "groupBy" | "mapObject" | "filterObject" => Value::Object(self.object),
            "distinctBy" if self.shape == Shape::Duplicate => duplicate_object_value(self.pairs),
            "distinctBy" | "takeWhile" if self.shape == Shape::Object => Value::Object(self.object),
            "mapString" => Value::String(self.text),
            "substringBy" => {
                let mut out = self.output;
                out.push(Value::String(self.text));
                Value::Array(out)
            }
            "partition" => Value::Object(Map::from_iter([
                ("success".into(), Value::Array(self.output)),
                ("failure".into(), Value::Array(self.other)),
            ])),
            "splitWhere" => Value::Object(Map::from_iter([
                ("l".into(), Value::Array(self.output)),
                ("r".into(), Value::Array(self.other)),
            ])),
            "orderBy" => {
                let mut sorted = self.sorted;
                sorted.sort_by(|a, b| crate::compare_sort_keys(&a.0, &b.0).then(a.1.cmp(&b.1)));
                Value::Array(sorted.into_iter().map(|(_, _, v)| v).collect())
            }
            _ => Value::Array(self.output),
        })
    }
    fn account(&mut self, bytes: usize, limit: usize) -> Result<(), DwError> {
        self.bytes = self.bytes.saturating_add(bytes);
        if self.bytes > limit {
            Err(error("materialized value limit exceeded"))
        } else {
            Ok(())
        }
    }
}
impl DebugSession {
    fn higher(&mut self, task: Task) {
        self.tasks.push(super::Task::Higher(task));
    }
    /// Collection lambdas bind positional parameters and shorthand values, while
    /// retaining captures. As in normal collection evaluation, parameter defaults
    /// and annotations do not change the supplied item/key/index bindings.
    pub(super) fn collection_callback(
        &mut self,
        lambda: &Expr,
        env: Env,
        args: Vec<Value>,
        range: SourceRange,
    ) -> Result<(), DwError> {
        let Kind::Lambda(params, body) = &lambda.kind else {
            return Err(error("expected callback grammar"));
        };
        let mut bindings = (*env).clone();
        for (name, value) in ["$", "$$", "$$$"].into_iter().zip(&args) {
            bindings.insert(name.into(), data(value.clone()));
        }
        let params = params
            .iter()
            .take(args.len())
            .map(|p| Param {
                name: p.name.clone(),
                type_source: None,
                default: None,
            })
            .collect::<Vec<_>>();
        let args = args.into_iter().take(params.len()).map(data).collect();
        let f = Rc::new(DebugFunction {
            params,
            body: body.clone(),
            captures: (*env).clone(),
            name: "lambda".into(),
            lexical: true,
            callback: true,
            overloads: vec![],
        });
        if let Some(frame) = self.frames.last_mut() {
            frame.range = range;
        }
        self.tasks
            .push(super::Task::BindCall(f, args, 0, bindings, 0));
        Ok(())
    }
    pub(super) fn start_higher(
        &mut self,
        op: String,
        value: Value,
        lambda: Expr,
        env: Env,
        range: SourceRange,
    ) -> Result<(), DwError> {
        if [
            "mapLeafValues",
            "nodeExists",
            "filterArrayLeafs",
            "filterObjectLeafs",
        ]
        .contains(&op.as_str())
        {
            if op.starts_with("filter") {
                self.higher(Task::TreeRoot);
            }
            self.higher(Task::TreeVisit(
                TreeContext {
                    op,
                    lambda,
                    env,
                    range,
                },
                value,
                Value::Array(vec![]),
            ));
            return Ok(());
        }
        if op == "failIf" {
            // The normal operator evaluates its RHS with $ bound to the input.
            let mut scoped = (*env).clone();
            scoped.insert("$".into(), data(value.clone()));
            let Kind::Lambda(_, body) = &lambda.kind else {
                unreachable!()
            };
            self.higher(Task::TreeLeaf(
                TreeContext {
                    op,
                    lambda: lambda.clone(),
                    env: env.clone(),
                    range,
                },
                value,
            ));
            self.eval(body.clone(), Rc::new(scoped), 1);
            return Ok(());
        }
        if op == "then" && value.is_null() || op == "onNull" && !value.is_null() {
            self.values.push(data(value));
            return Ok(());
        }
        let (shape, input) = if ["then", "onNull"].contains(&op.as_str()) {
            (Shape::Array, Input::Array(vec![value].into_iter()))
        } else if op == "distinctBy" && duplicate_object_pairs(&value).is_some() {
            (
                Shape::Duplicate,
                Input::Object(duplicate_object_pairs(&value).unwrap().into_iter()),
            )
        } else if matches!(&value, Value::Object(_))
            && [
                "pluck",
                "mapObject",
                "filterObject",
                "groupBy",
                "distinctBy",
                "takeWhile",
                "everyEntry",
                "someEntry",
            ]
            .contains(&op.as_str())
        {
            let Value::Object(o) = value else {
                unreachable!()
            };
            (
                Shape::Object,
                Input::Object(o.into_iter().collect::<Vec<_>>().into_iter()),
            )
        } else if op == "groupBy" && matches!(value, Value::String(_))
            || [
                "countCharactersBy",
                "everyCharacter",
                "mapString",
                "someCharacter",
                "substringBy",
            ]
            .contains(&op.as_str())
                && !value.is_null()
        {
            (
                Shape::Text,
                Input::Text(crate::as_dataweave_string(&value), 0),
            )
        } else if matches!(&value, Value::Array(_))
            && ![
                "pluck",
                "mapObject",
                "filterObject",
                "everyEntry",
                "someEntry",
                "countCharactersBy",
                "everyCharacter",
                "mapString",
                "someCharacter",
                "substringBy",
            ]
            .contains(&op.as_str())
        {
            let Value::Array(a) = value else {
                unreachable!()
            };
            (Shape::Array, Input::Array(a.into_iter()))
        } else {
            self.values.push(data(match op.as_str() {
                "some" | "every" | "someEntry" | "someCharacter" | "everyCharacter" => {
                    Value::Bool(false)
                }
                "everyEntry" => Value::Bool(true),
                _ => Value::Null,
            }));
            return Ok(());
        };
        self.higher(Task::Next(Collection {
            op,
            input,
            shape,
            lambda,
            env,
            range,
            index: 0,
            output: vec![],
            other: vec![],
            object: Map::new(),
            pairs: vec![],
            text: String::new(),
            seen: Default::default(),
            sorted: vec![],
            selected: None,
            total: 0.0,
            count: 0,
            bypass: false,
            bytes: 0,
        }));
        Ok(())
    }
    pub(super) fn tick_higher(&mut self, task: Task) -> Result<(), DwError> {
        match task {
            Task::TreeRoot => {
                let Value::Array(mut pair) = self.pop_data()? else {
                    unreachable!()
                };
                let value = pair.pop().unwrap();
                let value = if crate::is_truthy(&pair[0]) {
                    value
                } else {
                    Value::Null
                };
                self.values.push(data(value));
            }
            Task::Next(mut c) => {
                if let Some(item) = c.input.next(c.index) {
                    c.index += 1;
                    if c.bypass {
                        c.account(
                            estimated_size(&item.value) + 32,
                            self.options.max_materialized_bytes,
                        )?;
                        if c.op == "splitWhere" {
                            c.other.push(item.value);
                        } else {
                            c.output.push(item.value);
                        }
                        self.higher(Task::Next(c));
                    } else {
                        let mut args = vec![
                            item.value.clone(),
                            item.key
                                .as_ref()
                                .map(|k| Value::String(k.clone()))
                                .unwrap_or(Value::Number(item.index.into())),
                        ];
                        if item.key.is_some() {
                            args.push(Value::Number(item.index.into()));
                        }
                        let lambda = c.lambda.clone();
                        let env = c.env.clone();
                        let range = c.range;
                        self.higher(Task::Result(c, item));
                        self.collection_callback(&lambda, env, args, range)?;
                    }
                } else {
                    self.values.push(data(c.finish()?));
                }
            }
            Task::Result(mut c, item) => {
                let v = self.pop_data()?;
                let truth = crate::is_truthy(&v);
                let operation = c.op.clone();
                let op = operation.as_str();
                // Account retained state, not discarded predicate results. Aggregate
                // cursors and repeated duplicate keys must not grow with visited items.
                let bytes = match op {
                    "then" | "onNull" | "pluck" => estimated_size(&v) + 32,
                    "flatMap" if !v.is_null() => estimated_size(&v) + 32,
                    "filterObject" | "takeWhile" if truth => {
                        item.key.as_ref().map_or(0, String::len) + estimated_size(&item.value) + 32
                    }
                    "dropWhile" if !truth => estimated_size(&item.value) + 32,
                    "partition" | "splitWhere" => estimated_size(&item.value) + 32,
                    "orderBy" => estimated_size(&v) + estimated_size(&item.value) + 64,
                    "mapString" => crate::as_dataweave_string(&v).len(),
                    "substringBy" => {
                        if truth {
                            32
                        } else {
                            crate::as_dataweave_string(&item.value).len()
                        }
                    }
                    _ => 0,
                };
                c.account(bytes, self.options.max_materialized_bytes)?;
                if ["maxBy", "minBy"].contains(&op) {
                    self.check_size(&v)?;
                }
                match op {
                    "then" | "onNull" => c.output.push(v),
                    "pluck" => c.output.push(v),
                    "mapObject" => match v {
                        Value::Null => (),
                        Value::Object(o) => {
                            for (key, value) in o {
                                if let Some(old) = c.object.get(&key) {
                                    c.bytes = c
                                        .bytes
                                        .saturating_sub(estimated_size(old) + key.len() + 32);
                                }
                                c.account(
                                    estimated_size(&value) + key.len() + 32,
                                    self.options.max_materialized_bytes,
                                )?;
                                c.object.insert(key, value);
                            }
                        }
                        other => {
                            return Err(DwError::UnsupportedFeature(format!(
                                "mapObject mapper returned {other:?}"
                            )))
                        }
                    },
                    "filterObject" => {
                        if truth {
                            c.object.insert(item.key.unwrap(), item.value);
                        }
                    }
                    "flatMap" => match v {
                        Value::Array(a) => c.output.extend(a),
                        Value::Null => (),
                        v => c.output.push(v),
                    },
                    "takeWhile" => {
                        if !truth {
                            self.values.push(data(c.finish()?));
                            return Ok(());
                        }
                        if let Some(k) = item.key {
                            c.object.insert(k, item.value);
                        } else {
                            c.output.push(item.value);
                        }
                    }
                    "dropWhile" => {
                        if !truth {
                            c.output.push(item.value);
                            c.bypass = true;
                        }
                    }
                    "some" | "someEntry" | "someCharacter" if truth => {
                        self.values.push(data(Value::Bool(true)));
                        return Ok(());
                    }
                    "every" | "everyEntry" | "everyCharacter" if !truth => {
                        self.values.push(data(Value::Bool(false)));
                        return Ok(());
                    }
                    "countBy" | "countCharactersBy" => {
                        if truth {
                            c.count += 1;
                        }
                    }
                    "sumBy" => c.total += crate::numeric_value(&v)?,
                    "firstWith" if truth => {
                        self.values.push(data(item.value));
                        return Ok(());
                    }
                    "indexWhere" if truth => {
                        self.values.push(data(Value::Number(item.index.into())));
                        return Ok(());
                    }
                    "partition" => {
                        if truth {
                            c.output.push(item.value)
                        } else {
                            c.other.push(item.value)
                        }
                    }
                    "splitWhere" => {
                        if truth {
                            c.bypass = true;
                            c.other.push(item.value)
                        } else {
                            c.output.push(item.value)
                        }
                    }
                    "mapString" => c.text.push_str(&crate::as_dataweave_string(&v)),
                    "substringBy" => {
                        if truth {
                            c.output.push(Value::String(std::mem::take(&mut c.text)))
                        } else {
                            c.text.push_str(&crate::as_dataweave_string(&item.value))
                        }
                    }
                    "groupBy" => {
                        let key = crate::group_key(&v);
                        let initial = if c.object.contains_key(&key) {
                            0
                        } else {
                            key.len() + 32
                        };
                        let item_bytes = match c.shape {
                            Shape::Text => crate::as_dataweave_string(&item.value).len(),
                            _ => {
                                estimated_size(&item.value)
                                    + item.key.as_ref().map_or(0, String::len)
                                    + 32
                            }
                        };
                        c.account(initial + item_bytes, self.options.max_materialized_bytes)?;
                        let bucket = c.object.entry(key).or_insert_with(|| match c.shape {
                            Shape::Object => Value::Object(Map::new()),
                            Shape::Text => Value::String(String::new()),
                            _ => Value::Array(vec![]),
                        });
                        match bucket {
                            Value::Object(o) => {
                                o.insert(item.key.unwrap(), item.value);
                            }
                            Value::String(t) => {
                                t.push_str(&crate::as_dataweave_string(&item.value))
                            }
                            Value::Array(a) => a.push(item.value),
                            _ => unreachable!(),
                        }
                    }
                    "distinctBy" => {
                        let key = crate::stable_marker(&v);
                        let size = key.len() + estimated_size(&item.value) + 64;
                        if c.seen.insert(key) {
                            c.account(size, self.options.max_materialized_bytes)?;
                            match c.shape {
                                Shape::Duplicate => c.pairs.push((item.key.unwrap(), item.value)),
                                Shape::Object => {
                                    c.object.insert(item.key.unwrap(), item.value);
                                }
                                _ => c.output.push(item.value),
                            }
                        }
                    }
                    "orderBy" => c.sorted.push((v, item.index, item.value)),
                    "maxBy" | "minBy" => {
                        let replace = c.selected.as_ref().is_none_or(|(k, _)| {
                            let ord = crate::compare_sort_keys(&v, k);
                            if op == "maxBy" {
                                ord.is_gt()
                            } else {
                                ord.is_lt()
                            }
                        });
                        if replace {
                            c.selected = Some((v, item.value));
                        }
                    }
                    _ => (),
                }
                self.higher(Task::Next(c));
            }
            Task::TreeVisit(context, value, path) => {
                if context.op == "nodeExists" {
                    self.higher(Task::NodePredicate(
                        context.clone(),
                        value.clone(),
                        path.clone(),
                    ));
                    self.collection_callback(
                        &context.lambda,
                        context.env.clone(),
                        vec![value, path],
                        context.range,
                    )?;
                } else if let Some((shape, input)) = tree_input(&value) {
                    self.higher(Task::TreeNext(TreeChildren {
                        context,
                        path,
                        input,
                        shape,
                        index: 0,
                        output: vec![],
                        object: Map::new(),
                        bytes: 0,
                    }));
                } else {
                    let target = if context.op == "filterArrayLeafs" {
                        "ARRAY_TYPE"
                    } else {
                        "OBJECT_TYPE"
                    };
                    let applicable = context.op == "mapLeafValues"
                        || path
                            .as_array()
                            .and_then(|p| p.last())
                            .and_then(|v| v.get("kind"))
                            .and_then(Value::as_str)
                            == Some(target);
                    if applicable {
                        self.higher(Task::TreeLeaf(context.clone(), value.clone()));
                        self.collection_callback(
                            &context.lambda,
                            context.env.clone(),
                            vec![value, path],
                            context.range,
                        )?;
                    } else {
                        self.values
                            .push(data(tree_result(&context.op, true, value)));
                    }
                }
            }
            Task::TreeLeaf(context, original) => {
                let result = self.pop_data()?;
                if context.op == "failIf" {
                    if crate::is_truthy(&result) {
                        return Err(DwError::UnsupportedFeature("failIf".into()));
                    }
                    self.values.push(data(original));
                } else if context.op == "mapLeafValues" {
                    self.values.push(data(result));
                } else {
                    self.values.push(data(tree_result(
                        &context.op,
                        crate::is_truthy(&result),
                        original,
                    )));
                }
            }
            Task::NodePredicate(context, value, path) => {
                if crate::is_truthy(&self.pop_data()?) {
                    self.values.push(data(Value::Bool(true)));
                } else if let Some((shape, input)) = tree_input(&value) {
                    self.higher(Task::TreeNext(TreeChildren {
                        context,
                        path,
                        input,
                        shape,
                        index: 0,
                        output: vec![],
                        object: Map::new(),
                        bytes: 0,
                    }));
                } else {
                    self.values.push(data(Value::Bool(false)));
                }
            }
            Task::TreeNext(mut c) => {
                if let Some(item) = c.input.next(c.index) {
                    c.index += 1;
                    let path = crate::collections::path_with_segment(
                        &c.path,
                        if item.key.is_some() {
                            "OBJECT_TYPE"
                        } else {
                            "ARRAY_TYPE"
                        },
                        &item.key.clone().unwrap_or(item.index.to_string()),
                    );
                    let context = c.context.clone();
                    self.higher(Task::TreeChild(c, item.key));
                    self.higher(Task::TreeVisit(context, item.value, path));
                } else {
                    let value = if c.context.op == "nodeExists" {
                        Value::Bool(false)
                    } else if c.shape == Shape::Object {
                        Value::Object(c.object)
                    } else {
                        Value::Array(c.output)
                    };
                    self.values
                        .push(data(tree_result(&c.context.op, true, value)));
                }
            }
            Task::TreeChild(mut c, key) => {
                let result = self.pop_data()?;
                if c.context.op == "nodeExists" {
                    if crate::is_truthy(&result) {
                        self.values.push(data(Value::Bool(true)));
                    } else {
                        self.higher(Task::TreeNext(c));
                    }
                } else {
                    let value = if c.context.op.starts_with("filter") {
                        let Value::Array(mut pair) = result else {
                            unreachable!()
                        };
                        let value = pair.pop().unwrap();
                        if !crate::is_truthy(&pair[0]) {
                            None
                        } else {
                            Some(value)
                        }
                    } else {
                        Some(result)
                    };
                    if let Some(v) = value {
                        c.bytes = c.bytes.saturating_add(
                            estimated_size(&v) + key.as_ref().map_or(0, String::len) + 32,
                        );
                        if c.bytes > self.options.max_materialized_bytes {
                            return Err(error("materialized value limit exceeded"));
                        }
                        if let Some(k) = key {
                            c.object.insert(k, v);
                        } else {
                            c.output.push(v);
                        }
                    }
                    self.higher(Task::TreeNext(c));
                }
            }
            Task::JoinStart(op, callbacks, env, range) => {
                let right = self.pop_data()?;
                let left = self.pop_data()?;
                if let (Value::Array(left), Value::Array(right)) = (left, right) {
                    let matched_right = vec![false; right.len()];
                    self.higher(Task::JoinNext(Join {
                        op,
                        left,
                        right,
                        callbacks,
                        env,
                        range,
                        l: 0,
                        r: 0,
                        key: String::new(),
                        matched_left: false,
                        matched_right,
                        output: vec![],
                        bytes: 0,
                    }));
                } else {
                    self.values.push(data(Value::Null));
                }
            }
            Task::JoinNext(c) => {
                if c.l == c.left.len() {
                    self.higher(Task::JoinTail(c, 0));
                } else {
                    let lambda = c.callbacks[0].clone();
                    let env = c.env.clone();
                    let range = c.range;
                    let args = vec![c.left[c.l].clone(), Value::Number(c.l.into())];
                    self.higher(Task::JoinLeft(c));
                    self.collection_callback(&lambda, env, args, range)?;
                }
            }
            Task::JoinLeft(mut c) => {
                c.key = crate::stable_marker(&self.pop_data()?);
                c.r = 0;
                c.matched_left = false;
                self.higher(Task::JoinRight(c));
            }
            Task::JoinRight(mut c) => {
                if c.r == c.right.len() {
                    if !c.matched_left && c.op != "join" {
                        push_join(
                            &mut c,
                            Some(true),
                            None,
                            self.options.max_materialized_bytes,
                        )?;
                    }
                    c.l += 1;
                    self.higher(Task::JoinNext(c));
                } else {
                    let lambda = c.callbacks[1].clone();
                    let env = c.env.clone();
                    let range = c.range;
                    let args = vec![c.right[c.r].clone(), Value::Number(c.r.into())];
                    self.higher(Task::JoinKey(c));
                    self.collection_callback(&lambda, env, args, range)?;
                }
            }
            Task::JoinKey(mut c) => {
                if c.key == crate::stable_marker(&self.pop_data()?) {
                    c.matched_left = true;
                    c.matched_right[c.r] = true;
                    push_join(
                        &mut c,
                        Some(true),
                        Some(true),
                        self.options.max_materialized_bytes,
                    )?;
                }
                c.r += 1;
                self.higher(Task::JoinRight(c));
            }
            Task::JoinTail(mut c, index) => {
                if c.op != "outerJoin" || index == c.right.len() {
                    self.values.push(data(Value::Array(c.output)));
                } else {
                    c.r = index;
                    if !c.matched_right[index] {
                        push_join(
                            &mut c,
                            None,
                            Some(true),
                            self.options.max_materialized_bytes,
                        )?;
                    }
                    self.higher(Task::JoinTail(c, index + 1));
                }
            }
        }
        Ok(())
    }
}
fn tree_input(v: &Value) -> Option<(Shape, Input)> {
    match v {
        Value::Array(a) => Some((Shape::Array, Input::Array(a.clone().into_iter()))),
        Value::Object(o) => Some((
            Shape::Object,
            Input::Object(o.clone().into_iter().collect::<Vec<_>>().into_iter()),
        )),
        _ => None,
    }
}
fn tree_result(op: &str, keep: bool, v: Value) -> Value {
    if op.starts_with("filter") {
        Value::Array(vec![Value::Bool(keep), v])
    } else {
        v
    }
}
fn push_join(
    c: &mut Join,
    left: Option<bool>,
    right: Option<bool>,
    limit: usize,
) -> Result<(), DwError> {
    let mut o = Map::new();
    if left.is_some() {
        o.insert("l".into(), c.left[c.l].clone());
    }
    if right.is_some() {
        o.insert("r".into(), c.right[c.r].clone());
    }
    let v = Value::Object(o);
    c.bytes = c.bytes.saturating_add(estimated_size(&v) + 32);
    if c.bytes > limit {
        return Err(error("materialized value limit exceeded"));
    }
    c.output.push(v);
    Ok(())
}
