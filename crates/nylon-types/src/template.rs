use std::collections::HashMap;
use crate::context::NylonContext;
use nylon_error::NylonError;
use regex::Regex;
use serde_json::Value;

/// Represents a part of a JSON path
#[derive(Debug)]
enum PathPart {
    Key(String),
    Index(usize),
}

/// Represents a template expression that can be evaluated
#[derive(Debug, Clone)]
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
    parse_expr(&mut chars)
}

fn parse_expr<I: Iterator<Item = char>>(chars: &mut std::iter::Peekable<I>) -> Option<Expr> {
    skip_whitespace(chars);

    if let Some(c) = chars.peek() {
        match c {
            '\'' | '"' => parse_literal(chars),
            'a'..='z' | 'A'..='Z' | '_' => parse_func_or_var(chars),
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
        if c.is_alphanumeric() || c == '_' {
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
            "request_id" => ctx.request_id.clone().unwrap_or_default(),
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
                        "request_id" => ctx.request_id.clone().unwrap_or_default(),
                        _ => String::new(),
                    }
                } else {
                    String::new()
                }
            }
            "or" => {
                for arg in args {
                    let val = eval_expr(arg, ctx);
                    if !val.is_empty() {
                        return val;
                    }
                }
                String::new()
            }
            "env" => {
                if let Some(Expr::Var(v)) = args.first() {
                    std::env::var(v).unwrap_or_default()
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
                    } else {
                        target = map.entry(k).or_insert(Value::Object(Default::default()));
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
