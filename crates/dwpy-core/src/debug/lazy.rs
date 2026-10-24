//! Cooperative counterpart of compiled sequence evaluation. Batches preserve the
//! engine's per-source-item stage order, including flatMap fan-out and indices.
use super::*;
struct StageState {
    op: String,
    lambda: Expr,
    index: usize,
    finished: bool,
    seen: std::collections::HashSet<String>,
}
pub(super) struct Sequence {
    source: Cursor,
    stages: Vec<StageState>,
    env: Env,
    range: SourceRange,
    output: Vec<Value>,
    accumulator: Option<Value>,
    total: f64,
    count: usize,
    saw: bool,
    stopped: bool,
    bytes: usize,
    state_bytes: usize,
    pending_bytes: usize,
}
pub(super) enum Task {
    Next(Sequence),
    Batch(Sequence, usize, std::vec::IntoIter<Value>, Vec<Value>),
    Result(
        Sequence,
        usize,
        std::vec::IntoIter<Value>,
        Vec<Value>,
        Value,
    ),
    Emit(Sequence, std::vec::IntoIter<Value>),
}
impl Sequence {
    fn finish(self) -> Result<Value, DwError> {
        Ok(
            match self.stages.last().map(|s| s.op.as_str()).unwrap_or("") {
                "reduce" => self.accumulator.unwrap_or(Value::Null),
                "some" => Value::Bool(false),
                "every" => Value::Bool(self.saw),
                "firstWith" => Value::Null,
                "indexWhere" => Value::Number((-1).into()),
                "countBy" => Value::Number(self.count.into()),
                "sumBy" => crate::number_result(self.total)?,
                _ => Value::Array(self.output),
            },
        )
    }
    fn account(&mut self, value: &Value, limit: usize) -> Result<(), DwError> {
        self.bytes = self.bytes.saturating_add(estimated_size(value) + 32);
        if self.bytes > limit {
            Err(error("materialized value limit exceeded"))
        } else {
            Ok(())
        }
    }
}
impl DebugSession {
    fn lazy_task(&mut self, t: Task) {
        self.tasks.push(super::Task::Lazy(t));
    }
    pub(super) fn start_lazy(
        &mut self,
        source: Cursor,
        expressions: Vec<(String, Expr)>,
        env: Env,
        seed: Option<Value>,
    ) -> Result<(), DwError> {
        let range = self
            .frames
            .last()
            .map(|f| f.range)
            .unwrap_or(expressions[0].1.range);
        let stages = expressions
            .into_iter()
            .map(|(op, lambda)| StageState {
                op,
                lambda,
                index: 0,
                finished: false,
                seen: Default::default(),
            })
            .collect();
        self.lazy_task(Task::Next(Sequence {
            source,
            stages,
            env,
            range,
            output: vec![],
            accumulator: seed,
            total: 0.0,
            count: 0,
            saw: false,
            stopped: false,
            bytes: 0,
            state_bytes: 0,
            pending_bytes: 0,
        }));
        Ok(())
    }
    pub(super) fn tick_lazy(&mut self, t: Task) -> Result<(), DwError> {
        match t {
            Task::Next(mut s) => {
                if s.stopped {
                    self.values.push(data(s.finish()?));
                } else if let Some(item) = s.source.next() {
                    self.lazy_task(Task::Batch(s, 0, vec![item].into_iter(), vec![]));
                } else {
                    self.values.push(data(s.finish()?));
                }
            }
            Task::Batch(mut s, index, mut items, mut next) => {
                if index == s.stages.len() {
                    self.lazy_task(Task::Emit(s, items));
                    return Ok(());
                }
                if let Some(item) = items.next() {
                    let stage = &mut s.stages[index];
                    let secondary = if stage.op == "reduce" {
                        s.saw = true;
                        if let Some(acc) = s.accumulator.take() {
                            acc
                        } else {
                            s.accumulator = Some(item);
                            self.lazy_task(Task::Batch(s, index, items, next));
                            return Ok(());
                        }
                    } else {
                        Value::Number(stage.index.into())
                    };
                    stage.index += 1;
                    if stage.op == "takeWhile" && stage.finished {
                        self.lazy_task(Task::Batch(s, index, items, next));
                        return Ok(());
                    }
                    if stage.op == "dropWhile" && stage.finished {
                        next.push(item);
                        self.lazy_task(Task::Batch(s, index, items, next));
                        return Ok(());
                    }
                    let lambda = stage.lambda.clone();
                    let env = s.env.clone();
                    let range = s.range;
                    self.lazy_task(Task::Result(s, index, items, next, item.clone()));
                    self.collection_callback(&lambda, env, vec![item, secondary], range)?;
                } else if next.is_empty() {
                    s.pending_bytes = 0;
                    self.lazy_task(Task::Next(s));
                } else {
                    s.pending_bytes = 0;
                    self.lazy_task(Task::Batch(s, index + 1, next.into_iter(), vec![]));
                }
            }
            Task::Result(mut s, index, items, mut next, item) => {
                let result = self.pop_data()?;
                let truth = crate::is_truthy(&result);
                let before = next.len();
                let stage = &mut s.stages[index];
                match stage.op.as_str() {
                    "map" => next.push(result),
                    "filter" => {
                        if truth {
                            next.push(item)
                        }
                    }
                    "flatMap" => match result {
                        Value::Array(a) => next.extend(a),
                        Value::Null => (),
                        v => next.push(v),
                    },
                    "distinctBy" => {
                        let key = crate::stable_marker(&result);
                        let size = key.len() + 32;
                        if stage.seen.insert(key) {
                            s.state_bytes = s.state_bytes.saturating_add(size);
                            next.push(item)
                        }
                    }
                    "takeWhile" => {
                        if truth {
                            next.push(item)
                        } else {
                            stage.finished = true;
                            s.stopped = true
                        }
                    }
                    "dropWhile" => {
                        if !truth {
                            stage.finished = true;
                            next.push(item)
                        }
                    }
                    "reduce" => {
                        s.accumulator = Some(result);
                        s.saw = true;
                    }
                    "some" => {
                        s.saw = true;
                        if truth {
                            self.values.push(data(Value::Bool(true)));
                            return Ok(());
                        }
                    }
                    "every" => {
                        s.saw = true;
                        if !truth {
                            self.values.push(data(Value::Bool(false)));
                            return Ok(());
                        }
                    }
                    "firstWith" => {
                        s.saw = true;
                        if truth {
                            self.values.push(data(item));
                            return Ok(());
                        }
                    }
                    "indexWhere" => {
                        s.saw = true;
                        if truth {
                            self.values
                                .push(data(Value::Number((stage.index - 1).into())));
                            return Ok(());
                        }
                    }
                    "countBy" => {
                        s.saw = true;
                        if truth {
                            s.count += 1;
                        }
                    }
                    "sumBy" => {
                        s.saw = true;
                        s.total += crate::numeric_value(&result)?;
                    }
                    _ => return Err(error("unrecognized compiled debug stage")),
                }
                // Intermediate batches are values, never replayable callback plans.
                for v in &next[before..] {
                    s.pending_bytes = s.pending_bytes.saturating_add(estimated_size(v) + 32);
                }
                if s.pending_bytes
                    .saturating_add(s.state_bytes)
                    .saturating_add(s.bytes)
                    > self.options.max_materialized_bytes
                {
                    return Err(error("materialized value limit exceeded"));
                }
                self.lazy_task(Task::Batch(s, index, items, next));
            }
            Task::Emit(mut s, mut items) => {
                if let Some(item) = items.next() {
                    s.account(&item, self.options.max_materialized_bytes)?;
                    s.output.push(item);
                    self.lazy_task(Task::Emit(s, items));
                } else {
                    self.lazy_task(Task::Next(s));
                }
            }
        }
        Ok(())
    }
}
