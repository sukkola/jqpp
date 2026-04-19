use anyhow::{Context, Result};
use jaq_core::{Compiler, Ctx, Vars, data, load, unwrap_valr};
use jaq_json::Val;
use num_traits::ToPrimitive;
use serde_json::Value;

pub struct Executor {
    pub raw_input: Vec<u8>,
    pub json_input: Value,
    pub source_label: String,
}

impl Executor {
    pub fn execute(query: &str, input: &Value) -> Result<Vec<Value>> {
        let defs = jaq_core::defs()
            .chain(jaq_std::defs())
            .chain(jaq_json::defs());
        let funs = jaq_core::funs()
            .chain(jaq_std::funs())
            .chain(jaq_json::funs());

        let loader = load::Loader::new(defs);
        let arena = load::Arena::default();

        let program = load::File {
            code: query,
            path: (),
        };

        let modules = loader.load(&arena, program).map_err(|e| {
            let msgs: Vec<String> = e
                .iter()
                .map(|(_file, load_err)| {
                    use jaq_core::load::Error as LE;
                    match load_err {
                        LE::Lex(lex_errs) => {
                            let parts: Vec<String> = lex_errs
                                .iter()
                                .map(|(_, src)| {
                                    let preview: String = src.chars().take(30).collect();
                                    let ellipsis =
                                        if src.chars().count() > 30 { "…" } else { "" };
                                    format!("unexpected token {:?}{}", preview, ellipsis)
                                })
                                .collect();
                            if parts.is_empty() {
                                "lex error".to_string()
                            } else {
                                parts.join("; ")
                            }
                        }
                        other => format!("{other:?}"),
                    }
                })
                .collect();
            anyhow::anyhow!("Parse error: {}", msgs.join("; "))
        })?;

        let filter = Compiler::default()
            .with_funs(funs)
            .compile(modules)
            .map_err(|e| {
                let msgs: Vec<String> = e
                    .iter()
                    .map(|(_, undefs)| {
                        let names: Vec<String> =
                            undefs.iter().map(|(name, _)| name.to_string()).collect();
                        format!("undefined: {}", names.join(", "))
                    })
                    .collect();
                anyhow::anyhow!("Compile error: {}", msgs.join("; "))
            })?;

        let val_input: Val = serde_json::from_value(input.clone())
            .context("Failed to convert input to jaq value")?;

        let ctx = Ctx::<data::JustLut<Val>>::new(&filter.lut, Vars::new([]));
        let out = filter.id.run((ctx, val_input)).map(unwrap_valr);

        const MAX_RESULTS: usize = 10_000;
        let mut results = Vec::new();
        for res in out {
            if results.len() >= MAX_RESULTS {
                break;
            }
            match res {
                Ok(val) => results.push(val_to_json(val)),
                Err(e) => return Err(anyhow::anyhow!("Runtime error: {}", e)),
            }
        }
        Ok(results)
    }

    pub fn strip_format_op(query: &str) -> Option<(String, &'static str)> {
        let t = query.trim_end();
        for op in &["@csv", "@tsv"] {
            if let Some(rest) = t.strip_suffix(op) {
                let rest = rest.trim_end();
                if let Some(stripped) = rest.strip_suffix('|') {
                    let base = stripped.trim_end().to_string();
                    return Some((base, op));
                }
            }
        }
        None
    }

    pub fn execute_query(query: &str, input: &Value) -> Result<(Vec<Value>, bool)> {
        if let Some((base, op)) = Self::strip_format_op(query) {
            let base_results = Self::execute(&base, input)?;
            let formatted: Result<Vec<Value>> = base_results
                .iter()
                .map(|v| {
                    let s = match op {
                        "@csv" => format_csv(v)?,
                        "@tsv" => format_tsv(v)?,
                        _ => unreachable!(),
                    };
                    Ok(Value::String(s))
                })
                .collect();
            Ok((formatted?, true))
        } else {
            Ok((Self::execute(query, input)?, false))
        }
    }

    pub fn format_results(results: &[Value], raw: bool) -> String {
        results
            .iter()
            .map(|v| {
                if raw && let Some(s) = v.as_str() {
                    return s.to_string();
                }
                serde_json::to_string_pretty(v).unwrap_or_else(|_| "null".to_string())
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn status_line(&self) -> String {
        let size = self.raw_input.len();
        let size_str = if size < 1024 {
            format!("{} B", size)
        } else {
            format!("{:.1} KB", size as f64 / 1024.0)
        };
        format!("{} | {}", self.source_label, size_str)
    }
}

pub fn val_to_json(val: Val) -> Value {
    match val {
        Val::Null => Value::Null,
        Val::Bool(b) => Value::Bool(b),
        Val::Num(n) => match n {
            jaq_json::Num::Int(i) => Value::Number(serde_json::Number::from(i as i64)),
            jaq_json::Num::BigInt(i) => {
                if let Some(i64_val) = i.to_i64() {
                    Value::Number(serde_json::Number::from(i64_val))
                } else if let Some(f64_val) = i.to_f64() {
                    Value::Number(
                        serde_json::Number::from_f64(f64_val)
                            .unwrap_or(serde_json::Number::from(0)),
                    )
                } else {
                    Value::Number(serde_json::Number::from(0))
                }
            }
            jaq_json::Num::Float(f) => Value::Number(
                serde_json::Number::from_f64(f).unwrap_or(serde_json::Number::from(0)),
            ),
            jaq_json::Num::Dec(s) => {
                if let Ok(f) = s.parse::<f64>() {
                    Value::Number(
                        serde_json::Number::from_f64(f).unwrap_or(serde_json::Number::from(0)),
                    )
                } else {
                    Value::Number(serde_json::Number::from(0))
                }
            }
        },
        Val::BStr(b) | Val::TStr(b) => Value::String(String::from_utf8_lossy(&b).to_string()),
        Val::Arr(a) => Value::Array(a.iter().cloned().map(val_to_json).collect()),
        Val::Obj(o) => {
            let mut map = serde_json::Map::new();
            for (k, v) in o.iter() {
                let key = match k {
                    Val::BStr(b) | Val::TStr(b) => String::from_utf8_lossy(b).into_owned(),
                    _ => k.to_string(),
                };
                map.insert(key, val_to_json(v.clone()));
            }
            Value::Object(map)
        }
    }
}

fn csv_field(v: &Value) -> Result<String> {
    match v {
        Value::Null => Ok(String::new()),
        Value::Bool(b) => Ok(b.to_string()),
        Value::Number(n) => Ok(n.to_string()),
        Value::String(s) => {
            let escaped = s.replace('"', "\"\"");
            Ok(format!("\"{escaped}\""))
        }
        _ => Err(anyhow::anyhow!(
            "Runtime error: @csv does not support nested arrays or objects"
        )),
    }
}

fn format_csv(v: &Value) -> Result<String> {
    let arr = v.as_array().ok_or_else(|| {
        anyhow::anyhow!(
            "Runtime error: string ({}) cannot be csv-formatted, only array",
            serde_json::to_string(v).unwrap_or_default()
        )
    })?;
    let fields: Result<Vec<String>> = arr.iter().map(csv_field).collect();
    Ok(fields?.join(","))
}

fn format_tsv(v: &Value) -> Result<String> {
    let arr = v.as_array().ok_or_else(|| {
        anyhow::anyhow!(
            "Runtime error: {} cannot be tsv-formatted, only array",
            serde_json::to_string(v).unwrap_or_default()
        )
    })?;
    let mut fields = Vec::with_capacity(arr.len());
    for item in arr {
        let field = match item {
            Value::Null => String::new(),
            Value::Bool(b) => b.to_string(),
            Value::Number(n) => n.to_string(),
            Value::String(s) => {
                if s.contains('\t') || s.contains('\n') {
                    return Err(anyhow::anyhow!(
                        "Runtime error: @tsv string contains tab or newline: {:?}",
                        s
                    ));
                }
                s.clone()
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "Runtime error: @tsv does not support nested arrays or objects"
                ));
            }
        };
        fields.push(field);
    }
    Ok(fields.join("\t"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_valid_query() {
        let input = json!({"a": 1, "b": 2});
        let res = Executor::execute(".a", &input).unwrap();
        assert_eq!(res, vec![json!(1)]);
    }

    #[test]
    fn test_multi_value_output() {
        let input = json!([1, 2, 3]);
        let res = Executor::execute(".[]", &input).unwrap();
        assert_eq!(res, vec![json!(1), json!(2), json!(3)]);
    }

    #[test]
    fn test_parse_error() {
        let input = json!({});
        let res = Executor::execute(".", &input);
        assert!(res.is_ok());
        let res = Executor::execute("(", &input);
        assert!(res.is_err());
    }

    #[test]
    fn test_complex_object_query() {
        let input = json!({
            "generated_at_utc": "2026-04-11T07:17:40",
            "config": {
                "lighting_weights": {
                    "luminance_shift": 0.55
                }
            }
        });
        let res = Executor::execute(".config.lighting_weights.luminance_shift", &input).unwrap();
        assert_eq!(res, vec![json!(0.55)]);
    }

    #[test]
    fn complex_user_query_matches_jq() {
        let input = json!({
            "users": [
                {"id": 1, "name": "Alice",   "orders": [{"total": 50}, {"total": 20}]},
                {"id": 2, "name": "Bob",     "orders": [{"total": 200}]},
                {"id": 3, "name": "Charlie", "orders": []}
            ]
        });
        let query = ".users | map({name,total_spent:([.orders[].total] | add // 0)}) | map(select(.total_spent>50))";
        let (results, raw) = Executor::execute_query(query, &input).unwrap();
        assert!(!raw);
        assert_eq!(
            results,
            vec![json!([
                {"name": "Alice",   "total_spent": 70},
                {"name": "Bob",     "total_spent": 200},
            ])]
        );
    }

    #[test]
    fn fromjson_parses_embedded_string() {
        let input = json!({"log": "{\"event\":\"login\",\"user\":\"alice\"}"});
        let (res, _) = Executor::execute_query(".log | fromjson | .user", &input).unwrap();
        assert_eq!(res, vec![json!("alice")]);
    }

    #[test]
    fn tojson_round_trips() {
        let input = json!({"x": 1, "y": 2});
        let (res, _) = Executor::execute_query(". | tojson | fromjson | .x", &input).unwrap();
        assert_eq!(res, vec![json!(1)]);
    }

    #[test]
    fn sort_by_reverse_first() {
        let input = json!({"users": [
            {"id": 1, "name": "Alice",   "score": 85},
            {"id": 2, "name": "Bob",     "score": 72},
            {"id": 3, "name": "Charlie", "score": 91}
        ]});
        let (res, _) =
            Executor::execute_query(".users | sort_by(.score) | reverse | .[0].name", &input)
                .unwrap();
        assert_eq!(res, vec![json!("Charlie")]);
    }

    #[test]
    fn map_select_even() {
        let input = json!({"items": [1, 2, 3, 4, 5, 6, 7, 8]});
        let (res, _) =
            Executor::execute_query(".items | [.[] | select(. % 2 == 0)]", &input).unwrap();
        assert_eq!(res, vec![json!([2, 4, 6, 8])]);
    }

    #[test]
    fn arithmetic_mean() {
        let input = json!({"scores": [10, 20, 30, 40, 50]});
        let (res, _) = Executor::execute_query(".scores | add / length", &input).unwrap();
        assert_eq!(res, vec![json!(30.0)]);
    }

    #[test]
    fn to_from_entries_filter_by_type() {
        let input = json!({"name": "Alice", "age": 30, "active": true});
        let (res, _) = Executor::execute_query(
            "to_entries | map(select(.value | type == \"number\")) | from_entries",
            &input,
        )
        .unwrap();
        assert_eq!(res, vec![json!({"age": 30})]);
    }

    #[test]
    fn reduce_sum() {
        let input = json!([1, 2, 3, 4, 5]);
        let (res, _) = Executor::execute_query("reduce .[] as $x (0; . + $x)", &input).unwrap();
        assert_eq!(res, vec![json!(15)]);
    }

    #[test]
    fn group_by_aggregate() {
        let input = json!([
            {"k": "a", "v": 1},
            {"k": "b", "v": 2},
            {"k": "a", "v": 3}
        ]);
        let (res, _) = Executor::execute_query(
            "[group_by(.k)[] | {key:.[0].k, value:(map(.v)|add)}] | from_entries",
            &input,
        )
        .unwrap();
        assert_eq!(res, vec![json!({"a": 4, "b": 2})]);
    }

    #[test]
    fn unique_by_dedup() {
        let input = json!([
            {"id": 1, "name": "Alice"},
            {"id": 2, "name": "Bob"},
            {"id": 1, "name": "Alice2"}
        ]);
        let (res, _) = Executor::execute_query("unique_by(.id)", &input).unwrap();
        assert_eq!(
            res,
            vec![json!([
                {"id": 1, "name": "Alice"},
                {"id": 2, "name": "Bob"}
            ])]
        );
    }

    #[test]
    fn limit_range() {
        let (res, _) = Executor::execute_query("[limit(3; range(10))]", &json!(null)).unwrap();
        assert_eq!(res, vec![json!([0, 1, 2])]);
    }

    #[test]
    fn any_all_array() {
        let input = json!([1, 2, 3, 4, 5]);
        let (res, _) = Executor::execute_query("any(. > 4), all(. > 0)", &input).unwrap();
        assert_eq!(res, vec![json!(true), json!(true)]);
    }

    #[test]
    fn try_catch_type_error() {
        let input = json!("not-a-number");
        let (res, _) =
            Executor::execute_query("try tonumber catch \"not a number\"", &input).unwrap();
        assert_eq!(res, vec![json!("not a number")]);
    }

    #[test]
    fn variable_binding() {
        let input = json!({"teams": [
            {"name": "A", "members": [{"name": "x", "age": 30}, {"name": "y", "age": 20}]},
            {"name": "B", "members": [{"name": "z", "age": 40}]}
        ]});
        let (res, _) = Executor::execute_query(
            ".teams | map(. as $team | {team:$team.name, avg_age:($team.members | map(.age) | add / length), members_over_25:($team.members | map(select(.age > 25)) | map(.name))})",
            &input,
        ).unwrap();
        assert_eq!(
            res,
            vec![json!([
                {"team": "A", "avg_age": 25.0, "members_over_25": ["x"]},
                {"team": "B", "avg_age": 40.0, "members_over_25": ["z"]}
            ])]
        );
    }

    #[test]
    fn zip_arrays_to_objects() {
        let input = json!([[1, 2], [3, 4], [5, 6]]);
        let (res, _) =
            Executor::execute_query("[.[] | {a:.[0], b:.[1], sum:(.[0]+.[1])}]", &input).unwrap();
        assert_eq!(
            res,
            vec![json!([
                {"a": 1, "b": 2, "sum": 3},
                {"a": 3, "b": 4, "sum": 7},
                {"a": 5, "b": 6, "sum": 11}
            ])]
        );
    }

    #[test]
    fn sort_by_last() {
        let input = json!({"prices": {"apple": 1.5, "banana": 0.5, "cherry": 3.0}});
        let (res, _) = Executor::execute_query(
            ".prices | to_entries | sort_by(.value) | last | .key",
            &input,
        )
        .unwrap();
        assert_eq!(res, vec![json!("cherry")]);
    }

    #[test]
    fn single_quoted_string_is_parse_error() {
        let err = Executor::execute("'hello'", &json!(null)).unwrap_err();
        assert!(err.to_string().contains("Parse error"), "{err}");
    }

    #[test]
    fn csv_basic_types() {
        let v = json!([1, 2.5, true, null, "foo"]);
        assert_eq!(format_csv(&v).unwrap(), r#"1,2.5,true,,"foo""#);
    }

    #[test]
    fn csv_string_with_comma_and_quote() {
        let v = json!(["a,b", "say \"hi\""]);
        assert_eq!(format_csv(&v).unwrap(), "\"a,b\",\"say \"\"hi\"\"\"");
    }

    #[test]
    fn csv_non_array_is_error() {
        assert!(format_csv(&json!("hello")).is_err());
        assert!(format_csv(&json!(42)).is_err());
    }

    #[test]
    fn csv_nested_array_is_error() {
        assert!(format_csv(&json!([[1, 2]])).is_err());
    }

    #[test]
    fn tsv_basic_types() {
        let v = json!([1, 2.5, true, null, "foo"]);
        assert_eq!(format_tsv(&v).unwrap(), "1\t2.5\ttrue\t\tfoo");
    }

    #[test]
    fn tsv_tab_in_value_is_error() {
        assert!(format_tsv(&json!(["a\tb"])).is_err());
    }

    #[test]
    fn tsv_newline_in_value_is_error() {
        assert!(format_tsv(&json!(["a\nb"])).is_err());
    }

    #[test]
    fn strip_format_op_csv() {
        let (base, op) = Executor::strip_format_op("[1,2] | @csv").unwrap();
        assert_eq!(base, "[1,2]");
        assert_eq!(op, "@csv");
    }

    #[test]
    fn strip_format_op_tsv_no_spaces() {
        let (base, op) = Executor::strip_format_op(".foo|@tsv").unwrap();
        assert_eq!(base, ".foo");
        assert_eq!(op, "@tsv");
    }

    #[test]
    fn strip_format_op_none_for_plain_query() {
        assert!(Executor::strip_format_op(".foo | length").is_none());
    }

    #[test]
    fn strip_format_op_none_standalone() {
        assert!(Executor::strip_format_op("@csv").is_none());
    }

    #[test]
    fn execute_query_csv_end_to_end() {
        let input = json!({"row": [1, 2, "three"]});
        let (results, raw) = Executor::execute_query(".row | @csv", &input).unwrap();
        assert!(raw, "raw_output must be true for @csv");
        assert_eq!(results, vec![json!(r#"1,2,"three""#)]);
    }

    #[test]
    fn execute_query_tsv_end_to_end() {
        let input = json!({"row": ["a", "b", "c"]});
        let (results, raw) = Executor::execute_query(".row | @tsv", &input).unwrap();
        assert!(raw);
        assert_eq!(results, vec![json!("a\tb\tc")]);
    }

    #[test]
    fn execute_query_no_format_op_raw_false() {
        let input = json!({"a": 1});
        let (_, raw) = Executor::execute_query(".a", &input).unwrap();
        assert!(!raw);
    }

    #[test]
    fn tsv_on_object_fails_gracefully() {
        let input = json!({"a": 1, "b": 2});
        let res = Executor::execute_query(". | @tsv", &input);
        assert!(res.is_err());
        let err = res.unwrap_err().to_string();
        assert!(
            err.contains("cannot be tsv-formatted, only array"),
            "Error was: {}",
            err
        );
    }

    #[test]
    fn tsv_on_array_of_objects_fails() {
        let input = json!([{"a": 1}, {"b": 2}]);
        let res = Executor::execute_query(". | @tsv", &input);
        assert!(res.is_err());
        let err = res.unwrap_err().to_string();
        assert!(
            err.contains("does not support nested arrays or objects"),
            "Error was: {}",
            err
        );
    }

    #[test]
    fn format_results_raw_strips_quotes() {
        let results = vec![json!("a,b,c")];
        assert_eq!(Executor::format_results(&results, true), "a,b,c");
        assert_eq!(Executor::format_results(&results, false), "\"a,b,c\"");
    }
}
