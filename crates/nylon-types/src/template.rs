use crate::context::NylonContext;
use chrono::Utc;
use nylon_error::NylonError;
use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

/// Represents a part of a JSON path
#[derive(Debug)]
enum PathPart {
    Key(String),
    Index(usize),
}

/// Represents a template expression that can be evaluated
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// A literal string value
    Literal(String),
    /// A variable reference
    Var(String),
    /// A function call with name and arguments
    Func { name: String, args: Vec<Expr> },
}

/// Parse a template expression string into an Expr
pub fn parse_expression(input: &str) -> Option<Expr> {
    let mut chars = input.chars().peekable();
    let expr_option = parse_expr(&mut chars);

    if let Some(ref _expr) = expr_option {
        skip_whitespace(&mut chars);
        if chars.peek().is_none() {
            expr_option
        } else {
            None
        }
    } else {
        expr_option
    }
}

fn parse_expr<I: Iterator<Item = char>>(chars: &mut std::iter::Peekable<I>) -> Option<Expr> {
    skip_whitespace(chars);

    if let Some(c) = chars.peek() {
        match c {
            '\'' | '"' => parse_literal(chars),
            'a'..='z' | 'A'..='Z' | '_' | '-' => parse_func_or_var(chars),
            _ => None,
        }
    } else {
        None
    }
}

fn parse_literal<I: Iterator<Item = char>>(chars: &mut std::iter::Peekable<I>) -> Option<Expr> {
    let quote = chars.next()?; // ' or "
    let mut val = String::new();
    while let Some(&c) = chars.peek() {
        chars.next();
        if c == quote {
            break;
        }
        val.push(c);
    }
    Some(Expr::Literal(val))
}

fn parse_func_or_var<I: Iterator<Item = char>>(chars: &mut std::iter::Peekable<I>) -> Option<Expr> {
    let mut name = String::new();
    while let Some(&c) = chars.peek() {
        if c.is_alphanumeric() || c == '_' || c == '-' {
            name.push(c);
            chars.next();
        } else {
            break;
        }
    }

    skip_whitespace(chars);

    if chars.peek() == Some(&'(') {
        chars.next(); // consume '('
        let mut args = vec![];
        loop {
            skip_whitespace(chars);
            if let Some(&')') = chars.peek() {
                chars.next();
                break;
            }
            if let Some(expr) = parse_expr(chars) {
                args.push(expr);
            }
            skip_whitespace(chars);
            if chars.peek() == Some(&',') {
                chars.next();
            }
        }
        Some(Expr::Func { name, args })
    } else {
        Some(Expr::Var(name))
    }
}

fn skip_whitespace<I: Iterator<Item = char>>(chars: &mut std::iter::Peekable<I>) {
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
        } else {
            break;
        }
    }
}

/// Extract and parse template expressions from a string
pub fn extract_and_parse_templates(input: &str) -> Result<Vec<Expr>, NylonError> {
    let re = Regex::new(r"\$\{([^}]+)\}")
        .map_err(|e| NylonError::ConfigError(format!("Invalid regex: {e}")))?;

    let mut result = Vec::new();
    let mut last = 0;

    for cap in re.captures_iter(input) {
        let whole_match = cap.get(0).unwrap();
        let expr_str = &cap[1];

        // Push literal (if any)
        if whole_match.start() > last {
            let literal = &input[last..whole_match.start()];
            if !literal.is_empty() {
                result.push(Expr::Literal(literal.to_string()));
            }
        }

        // Parse expression
        if let Some(expr) = parse_expression(expr_str) {
            result.push(expr);
        }

        last = whole_match.end();
    }

    // Trailing literal
    if last < input.len() {
        result.push(Expr::Literal(input[last..].to_string()));
    }

    Ok(result)
}

/// Evaluate a template expression in the given context
pub fn eval_expr(expr: &Expr, ctx: &NylonContext) -> String {
    match expr {
        Expr::Literal(s) => s.clone(),
        Expr::Var(name) => match name.as_str() {
            "client_ip" => ctx.client_ip.clone(),
            _ => String::new(), // fallback
        },
        Expr::Func { name, args } => match name.as_str() {
            "header" => {
                if let Some(Expr::Var(h)) = args.first() {
                    ctx.headers.get(h).cloned().unwrap_or_default()
                } else {
                    String::new()
                }
            }
            "var" => {
                if let Some(Expr::Var(v)) = args.first() {
                    match v.as_str() {
                        "client_ip" => ctx.client_ip.clone(),
                        _ => String::new(),
                    }
                } else {
                    String::new()
                }
            }
            "env" => {
                if let Some(Expr::Var(v)) = args.first() {
                    std::env::var(v).unwrap_or_default()
                } else {
                    String::new()
                }
            }
            "or" => {
                // Or
                for arg in args {
                    let val = eval_expr(arg, ctx);
                    if !val.is_empty() {
                        return val;
                    }
                }
                String::new()
            }
            "eq" => {
                // Equal
                if args.len() >= 2 {
                    let val1 = eval_expr(&args[0], ctx);
                    let val2 = eval_expr(&args[1], ctx);

                    if val1 == val2 {
                        // If a third argument is provided, evaluate and return it as the result of eq.
                        if let Some(value_if_equal) = args.get(2) {
                            eval_expr(value_if_equal, ctx)
                        } else {
                            // If no third argument, return the common value.
                            // This makes 'eq(A, B)' usable in 'or' constructs,
                            // returning the value of A (and B) if they are equal.
                            val1
                        }
                    } else {
                        // Not equal, return an empty string.
                        String::new()
                    }
                } else {
                    // Not enough arguments for comparison, return an empty string.
                    String::new()
                }
            }
            "neq" => {
                // Not Equal
                if args.len() >= 2 {
                    let val1 = eval_expr(&args[0], ctx);
                    let val2 = eval_expr(&args[1], ctx);

                    if val1 != val2 {
                        if let Some(value_if_not_equal) = args.get(2) {
                            eval_expr(value_if_not_equal, ctx)
                        } else {
                            val1 // true
                        }
                    } else {
                        String::new() // false
                    }
                } else {
                    String::new()
                }
            }
            "upper" => {
                // Convert to uppercase: upper('someString')
                if let Some(arg_expr) = args.first() {
                    eval_expr(arg_expr, ctx).to_uppercase()
                } else {
                    String::new()
                }
            }
            "lower" => {
                // Convert to lowercase: lower('SomeString')
                if let Some(arg_expr) = args.first() {
                    eval_expr(arg_expr, ctx).to_lowercase()
                } else {
                    String::new()
                }
            }

            "len" => {
                // Get length of a string: len('abc') -> "3"
                if let Some(arg_expr) = args.first() {
                    eval_expr(arg_expr, ctx).len().to_string()
                } else {
                    String::new()
                }
            }
            "if_cond" => {
                // Conditional: if_cond(condition_expr, then_expr, else_expr)
                if args.len() == 3 {
                    let condition = eval_expr(&args[0], ctx);
                    if !condition.is_empty() {
                        // true
                        eval_expr(&args[1], ctx)
                    } else {
                        eval_expr(&args[2], ctx)
                    }
                } else {
                    String::new() // Incorrect number of arguments
                }
            }
            "timestamp" => Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            "uuid" => {
                // uuid(v4), uuid(v7)
                if let Some(Expr::Var(v)) = args.first() {
                    if v == "v4" {
                        Uuid::new_v4().to_string()
                    } else if v == "v7" {
                        Uuid::now_v7().to_string()
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                }
            }
            _ => String::new(),
        },
    }
}

/// Render a template string by evaluating all expressions in the given context
pub fn render_template_string(expr: &[Expr], ctx: &NylonContext) -> String {
    let mut result = String::new();
    for expr in expr {
        result.push_str(&eval_expr(expr, ctx));
    }
    result
}

/// Walk through a JSON value and visit each path
pub fn walk_json(value: &Value, path: String, visit: &mut impl FnMut(String, &Value)) {
    match value {
        Value::Object(map) => {
            for (k, v) in map {
                let new_path = if path.is_empty() {
                    k.clone()
                } else {
                    format!("{}.{}", path, k)
                };
                walk_json(v, new_path, visit);
            }
        }
        Value::Array(arr) => {
            for (i, v) in arr.iter().enumerate() {
                let new_path = format!("{}[{}]", path, i);
                walk_json(v, new_path, visit);
            }
        }
        _ => {
            visit(path, value);
        }
    }
}

fn parse_path(path: &str) -> Vec<PathPart> {
    let mut result = Vec::new();
    let mut key = String::new();
    let mut chars = path.chars().peekable();

    while let Some(&c) = chars.peek() {
        match c {
            '.' => {
                if !key.is_empty() {
                    result.push(PathPart::Key(key.clone()));
                    key.clear();
                }
                chars.next();
            }
            '[' => {
                if !key.is_empty() {
                    result.push(PathPart::Key(key.clone()));
                    key.clear();
                }
                chars.next();
                let mut index_str = String::new();
                while let Some(&d) = chars.peek() {
                    if d == ']' {
                        break;
                    }
                    index_str.push(d);
                    chars.next();
                }
                chars.next(); // skip ']'
                if let Ok(n) = index_str.parse::<usize>() {
                    result.push(PathPart::Index(n));
                }
            }
            _ => {
                key.push(c);
                chars.next();
            }
        }
    }

    if !key.is_empty() {
        result.push(PathPart::Key(key));
    }

    result
}

fn set_json_value(root: &mut Value, path: &str, new_val: Value) {
    let mut target = root;
    let parts = parse_path(path);

    for (i, part) in parts.iter().enumerate() {
        match part {
            PathPart::Key(k) => {
                if let Value::Object(map) = target {
                    if i == parts.len() - 1 {
                        map.insert(k.clone(), new_val);
                        return;
                    } else if parts.iter().any(|p| matches!(p, PathPart::Index(_))) {
                        target = map
                            .entry(k.clone())
                            .or_insert(Value::Array(Default::default()));
                    } else {
                        target = map
                            .entry(k.clone())
                            .or_insert(Value::Object(Default::default()));
                    }
                } else {
                    return;
                }
            }
            PathPart::Index(n) => {
                if let Value::Array(arr) = target {
                    if *n >= arr.len() {
                        arr.resize(*n + 1, Value::Null);
                    }
                    if i == parts.len() - 1 {
                        arr[*n] = new_val;
                        return;
                    } else {
                        target = &mut arr[*n];
                    }
                } else {
                    return;
                }
            }
        }
    }
}

/// Apply template expressions to a JSON value
pub fn apply_payload_ast(
    value: &mut Value,
    payload_ast: &HashMap<String, Vec<Expr>>,
    ctx: &NylonContext,
) {
    for (path, exprs) in payload_ast {
        let rendered = render_template_string(exprs, ctx);
        set_json_value(value, path, Value::String(rendered));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn mock_ctx() -> NylonContext {
        let mut headers = HashMap::new();
        headers.insert("X-Test-Header".to_string(), "HeaderValue".to_string());
        headers.insert("Host".to_string(), "example.com".to_string());

        NylonContext {
            headers,
            ..Default::default()
        }
    }

    fn eval_str(expr_str: &str, ctx: &NylonContext) -> String {
        let expr = parse_expression(expr_str)
            .unwrap_or_else(|| panic!("Failed to parse test expression: {}", expr_str));
        eval_expr(&expr, ctx)
    }

    #[test]
    fn test_eval_literal() {
        let ctx = mock_ctx();
        assert_eq!(eval_str("'hello literal'", &ctx), "hello literal");
        assert_eq!(
            eval_str("\"double quote literal\"", &ctx),
            "double quote literal"
        );
    }

    #[test]
    fn test_eval_var_direct() {
        let ctx = mock_ctx();
        assert_eq!(eval_str("client_ip", &ctx), "127.0.0.1");
        assert_eq!(
            eval_str("request_id", &ctx),
            "",
            "Expr::Var(\"request_id\") should be empty per current code"
        );
        assert_eq!(eval_str("unknown_variable", &ctx), "");
    }

    #[test]
    fn test_eval_func_var_function() {
        let ctx = mock_ctx();
        assert_eq!(eval_str("var(client_ip)", &ctx), "127.0.0.1");
        assert_eq!(
            eval_str("var(request_id)", &ctx),
            "",
            "var(request_id) should be empty per current code"
        );
        assert_eq!(eval_str("var(something_else)", &ctx), "");
    }

    #[test]
    fn test_eval_func_header() {
        let ctx = mock_ctx();
        assert_eq!(eval_str("header(X-Test-Header)", &ctx), "HeaderValue");
        assert_eq!(eval_str("header(Host)", &ctx), "example.com");
        assert_eq!(eval_str("header(NonExistentHeader)", &ctx), "");
        assert_eq!(
            eval_str("header(host)", &ctx),
            "",
            "Header keys are case-sensitive in HashMap"
        );
    }

    #[test]
    fn test_eval_func_env() {
        let ctx = mock_ctx();
        unsafe {
            std::env::set_var("MY_TEST_ENV_VAR", "EnvTestValue");
        }
        assert_eq!(eval_str("env(MY_TEST_ENV_VAR)", &ctx), "EnvTestValue");
        assert_eq!(eval_str("env(NON_EXISTENT_ENV_VAR)", &ctx), "");
        unsafe {
            std::env::remove_var("MY_TEST_ENV_VAR");
        }
    }

    #[test]
    fn test_eval_func_or() {
        let ctx = mock_ctx();
        assert_eq!(eval_str("or('val1', 'val2')", &ctx), "val1");
        assert_eq!(eval_str("or('', 'val2')", &ctx), "val2");
        assert_eq!(eval_str("or(unknown, 'val2')", &ctx), "val2");
        assert_eq!(eval_str("or('', '', 'val3')", &ctx), "val3");
        assert_eq!(eval_str("or('', '', '')", &ctx), "");
        assert_eq!(
            eval_str("or(or('', client_ip), 'fallback')", &ctx),
            "127.0.0.1"
        );
    }

    #[test]
    fn test_eval_func_eq() {
        let ctx = mock_ctx();
        assert_eq!(eval_str("eq('a', 'a')", &ctx), "a");
        assert_eq!(eval_str("eq('a', 'a', 'EQUAL')", &ctx), "EQUAL");
        assert_eq!(eval_str("eq('a', 'b')", &ctx), "");
        assert_eq!(eval_str("eq('a', 'b', 'EQUAL')", &ctx), "");
        assert_eq!(
            eval_str("eq(client_ip, '127.0.0.1', 'local')", &ctx),
            "local"
        );
    }

    #[test]
    fn test_eval_func_neq() {
        let ctx = mock_ctx();
        assert_eq!(eval_str("neq('a', 'b')", &ctx), "a");
        assert_eq!(eval_str("neq('a', 'b', 'NOT_EQUAL')", &ctx), "NOT_EQUAL");
        assert_eq!(eval_str("neq('a', 'a')", &ctx), "");
        assert_eq!(eval_str("neq('a', 'a', 'NOT_EQUAL')", &ctx), "");
        assert_eq!(
            eval_str("neq(client_ip, '1.1.1.1', 'remote')", &ctx),
            "remote"
        );
    }

    #[test]
    fn test_eval_func_upper_lower_len() {
        let ctx = mock_ctx();
        assert_eq!(eval_str("upper('hello world')", &ctx), "HELLO WORLD");
        assert_eq!(eval_str("lower('HELLO WORLD')", &ctx), "hello world");
        assert_eq!(eval_str("len('hello')", &ctx), "5");
        assert_eq!(eval_str("len(client_ip)", &ctx), "9");
        assert_eq!(eval_str("upper('')", &ctx), "");
        assert_eq!(eval_str("lower('')", &ctx), "");
        assert_eq!(eval_str("len('')", &ctx), "0");
    }

    #[test]
    fn test_eval_func_if_cond() {
        let ctx = mock_ctx();
        assert_eq!(
            eval_str("if_cond('condition_true', 'then_val', 'else_val')", &ctx),
            "then_val"
        );
        assert_eq!(
            eval_str(
                "if_cond(eq(client_ip, '127.0.0.1'), 'local_ip', 'remote_ip')",
                &ctx
            ),
            "local_ip"
        );
        assert_eq!(
            eval_str("if_cond('', 'then_val', 'else_val')", &ctx),
            "else_val"
        ); // Empty string is false
        assert_eq!(
            eval_str(
                "if_cond(eq(client_ip, 'other_ip'), 'local_ip', 'remote_ip')",
                &ctx
            ),
            "remote_ip"
        );
    }

    #[test]
    fn test_eval_func_timestamp() {
        let ctx = mock_ctx();
        let ts = eval_str("timestamp()", &ctx);
        assert!(
            ts.contains('T') && ts.contains('Z'),
            "Timestamp format basic check failed"
        );
        assert!(
            ts.starts_with(&Utc::now().format("%Y-%m-%d").to_string()),
            "Timestamp year-month-day check failed"
        );
    }

    #[test]
    fn test_eval_func_uuid() {
        let ctx = mock_ctx();
        let uuid_v4 = eval_str("uuid(v4)", &ctx);
        assert_eq!(uuid_v4.len(), 36, "UUID v4 length incorrect");
        assert_eq!(
            uuid_v4.chars().nth(14),
            Some('4'),
            "UUID v4 version char incorrect"
        );

        let uuid_v7 = eval_str("uuid(v7)", &ctx);
        assert_eq!(uuid_v7.len(), 36, "UUID v7 length incorrect");
        assert_eq!(
            uuid_v7.chars().nth(14),
            Some('7'),
            "UUID v7 version char incorrect"
        );

        assert_eq!(eval_str("uuid(vx)", &ctx), "", "uuid with invalid version");
        assert_eq!(eval_str("uuid()", &ctx), "", "uuid with no argument");
    }

    #[test]
    fn test_parse_expression() {
        assert_eq!(
            parse_expression("'text'"),
            Some(Expr::Literal("text".to_string()))
        );
        assert_eq!(
            parse_expression("myVar"),
            Some(Expr::Var("myVar".to_string()))
        );
        assert_eq!(
            parse_expression("do()"),
            Some(Expr::Func {
                name: "do".to_string(),
                args: vec![]
            })
        );
        assert_eq!(
            parse_expression("  do  (  'arg1'  ,  argVar )  "),
            Some(Expr::Func {
                name: "do".to_string(),
                args: vec![
                    Expr::Literal("arg1".to_string()),
                    Expr::Var("argVar".to_string())
                ]
            })
        );
        assert_eq!(
            parse_expression("outer(inner(var), 'lit')"),
            Some(Expr::Func {
                name: "outer".to_string(),
                args: vec![
                    Expr::Func {
                        name: "inner".to_string(),
                        args: vec![Expr::Var("var".to_string())]
                    },
                    Expr::Literal("lit".to_string())
                ]
            })
        );
        assert_eq!(parse_expression("invalid-char()"), None);
        assert_eq!(parse_expression("func('unterminated literal"), None);
        assert_eq!(parse_expression("func(arg1,"), None);
    }

    #[test]
    fn test_extract_and_parse_templates() {
        let res1 = extract_and_parse_templates("Hello ${upper(world_var)} from ${client_ip}!");
        assert!(res1.is_ok());
        let exprs1 = res1.unwrap();
        assert_eq!(exprs1.len(), 5);
        assert_eq!(exprs1[0], Expr::Literal("Hello ".to_string()));
        assert_eq!(exprs1[2], Expr::Literal(" from ".to_string()));
        assert_eq!(exprs1[4], Expr::Literal("!".to_string()));
        if let Expr::Func { name, args } = &exprs1[1] {
            assert_eq!(name, "upper");
            assert_eq!(args.len(), 1);
            assert_eq!(args[0], Expr::Var("world_var".to_string()));
        } else {
            panic!("Expected Func 'upper'");
        }
        assert_eq!(exprs1[3], Expr::Var("client_ip".to_string()));

        let res2 = extract_and_parse_templates("No templates.").unwrap();
        assert_eq!(res2, vec![Expr::Literal("No templates.".to_string())]);

        let res3 = extract_and_parse_templates("${var1}${var2}").unwrap();
        assert_eq!(
            res3,
            vec![Expr::Var("var1".to_string()), Expr::Var("var2".to_string())]
        );

        let res4 = extract_and_parse_templates("").unwrap();
        assert_eq!(
            res4,
            Vec::<Expr>::new(),
            "Empty input should result in empty Vec<Expr>"
        );

        let res5 = extract_and_parse_templates("${func(arg)}").unwrap();
        assert_eq!(res5.len(), 1);
    }

    #[test]
    fn test_render_template_string() {
        let ctx = mock_ctx();
        let exprs = extract_and_parse_templates(
            "IP: ${client_ip}. Host: ${upper(header(Host))}. UUID: ${uuid(v4)}.",
        )
        .unwrap();
        let rendered = render_template_string(&exprs, &ctx);
        assert!(rendered.starts_with("IP: 127.0.0.1. Host: EXAMPLE.COM. UUID: "));
        assert_eq!(
            rendered.matches('-').count(),
            4,
            "Rendered UUID should have 4 hyphens"
        );
    }

    #[test]
    fn test_parse_path_debug_compare() {
        assert_eq!(
            format!("{:?}", parse_path("key1.key2")),
            format!(
                "{:?}",
                vec![
                    PathPart::Key("key1".to_string()),
                    PathPart::Key("key2".to_string())
                ]
            )
        );
        assert_eq!(
            format!("{:?}", parse_path("key[0].sub")),
            format!(
                "{:?}",
                vec![
                    PathPart::Key("key".to_string()),
                    PathPart::Index(0),
                    PathPart::Key("sub".to_string())
                ]
            )
        );
        assert_eq!(
            format!("{:?}", parse_path("arr[10]")),
            format!(
                "{:?}",
                vec![PathPart::Key("arr".to_string()), PathPart::Index(10)]
            )
        );
        assert_eq!(
            format!("{:?}", parse_path("[0][1]")),
            format!("{:?}", vec![PathPart::Index(0), PathPart::Index(1)])
        );
    }

    #[test]
    fn test_set_json_value() {
        let mut root_val = json!({});

        set_json_value(&mut root_val, "name", json!("Nylon"));
        assert_eq!(root_val, json!({"name": "Nylon"}));

        set_json_value(&mut root_val, "config.version", json!("1.0"));
        assert_eq!(root_val["config"]["version"], json!("1.0"));

        set_json_value(&mut root_val, "features[0]", json!("templating"));
        assert_eq!(root_val["features"], json!(["templating"]));

        set_json_value(&mut root_val, "name", json!("Nylon Proxy"));
        assert_eq!(root_val["name"], json!("Nylon Proxy"));

        root_val["obj_in_arr"] = json!([{}]);
        set_json_value(&mut root_val, "obj_in_arr[0].type", json!("gateway"));
        assert_eq!(root_val["obj_in_arr"][0]["type"], json!("gateway"));
    }

    #[test]
    fn test_apply_payload_ast() {
        let ctx = mock_ctx();
        let mut data = json!({
            "user_info": {
                "id": "old_id",
                "status": "active"
            },
            "system_load": 0.5
        });

        let mut ast_map = HashMap::new();
        let expr1 = extract_and_parse_templates("IP-${client_ip}").unwrap();
        let expr2 = extract_and_parse_templates("Status: ${upper('pending')}_${uuid(v7)}").unwrap();
        let expr3 = extract_and_parse_templates("New Load: ${len('moderate')}").unwrap();

        ast_map.insert("user_info.id".to_string(), expr1);
        ast_map.insert("user_info.status".to_string(), expr2);
        ast_map.insert("new_metrics.load_desc".to_string(), expr3);

        apply_payload_ast(&mut data, &ast_map, &ctx);

        assert_eq!(data["user_info"]["id"], json!("IP-127.0.0.1"));
        let status_val = data["user_info"]["status"].as_str().unwrap();
        assert!(status_val.starts_with("Status: PENDING_"));
        assert_eq!(status_val.len(), "Status: PENDING_".len() + 36);
        assert_eq!(data["new_metrics"]["load_desc"], json!("New Load: 8"));
        assert_eq!(data["system_load"], json!(0.5));
    }
}
