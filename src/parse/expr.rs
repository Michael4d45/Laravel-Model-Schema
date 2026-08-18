use mago_names::ResolvedNames;
use mago_syntax::cst::{
    Access, Argument, ArrayElement, Call, ClassLikeConstantSelector, ClassLikeMemberSelector,
    Expression, Hint, Literal, PropertyItem, Statement,
};

use super::names::{bytes_to_string, owned_name, resolve_identifier};

pub(crate) fn php_string(bytes: &[u8], raw: &[u8]) -> String {
    if let Some(s) = owned_name(bytes) {
        return s;
    }
    let raw_s = String::from_utf8_lossy(raw);
    let trimmed = raw_s.trim();
    if (trimmed.starts_with('\'') && trimmed.ends_with('\''))
        || (trimmed.starts_with('"') && trimmed.ends_with('"'))
    {
        trimmed[1..trimmed.len() - 1].to_string()
    } else {
        trimmed.to_string()
    }
}

pub(crate) fn literal_string(expr: &Expression<'_>) -> Option<String> {
    match expr {
        Expression::Literal(Literal::String(string)) => {
            Some(php_string(string.value.unwrap_or(string.raw), string.raw))
        }
        Expression::Parenthesized(inner) => literal_string(inner.expression),
        _ => None,
    }
}

pub(crate) fn first_arg_string(call: &Call<'_>) -> Option<String> {
    let args = call.get_argument_list();
    let first = args.arguments.first()?;
    match first {
        Argument::Positional(positional) => literal_string(positional.value),
        Argument::Named(named) => literal_string(named.value),
    }
}

pub(crate) fn nth_arg<'a>(call: &'a Call<'a>, index: usize) -> Option<&'a Expression<'a>> {
    let args = call.get_argument_list();
    args.arguments.iter().nth(index).and_then(|arg| match arg {
        Argument::Positional(positional) => Some(positional.value),
        Argument::Named(named) => Some(named.value),
    })
}

pub(crate) fn selector_name(selector: &ClassLikeMemberSelector<'_>) -> Option<String> {
    match selector {
        ClassLikeMemberSelector::Identifier(ident) => owned_name(ident.value),
        _ => None,
    }
}

pub(crate) fn is_class_keyword(selector: &ClassLikeConstantSelector<'_>) -> bool {
    match selector {
        ClassLikeConstantSelector::Identifier(ident) => ident.value.eq_ignore_ascii_case(b"class"),
        _ => false,
    }
}

pub(crate) fn class_name_from_expr(
    expr: &Expression<'_>,
    resolved: &ResolvedNames<'_>,
    namespace: Option<&str>,
) -> Option<String> {
    match expr {
        Expression::Identifier(ident) => Some(resolve_identifier(resolved, ident, namespace)),
        Expression::Parenthesized(inner) => {
            class_name_from_expr(inner.expression, resolved, namespace)
        }
        _ => None,
    }
}

pub(crate) fn class_const_name(
    expr: &Expression<'_>,
    resolved: &ResolvedNames<'_>,
    namespace: Option<&str>,
) -> Option<String> {
    match expr {
        Expression::Access(Access::ClassConstant(access)) if is_class_keyword(&access.constant) => {
            class_name_from_expr(access.class, resolved, namespace)
        }
        Expression::Literal(Literal::String(string)) => {
            Some(php_string(string.value.unwrap_or(string.raw), string.raw))
        }
        Expression::Parenthesized(inner) => class_const_name(inner.expression, resolved, namespace),
        _ => None,
    }
}

pub(crate) fn array_string_values(expr: &Expression<'_>) -> Vec<String> {
    let elements = match expr {
        Expression::Array(array) => &array.elements,
        Expression::LegacyArray(array) => &array.elements,
        Expression::Parenthesized(inner) => return array_string_values(inner.expression),
        _ => return Vec::new(),
    };
    let mut values = Vec::new();
    for element in elements.iter() {
        match element {
            ArrayElement::Value(value) => {
                if let Some(s) = literal_string(value.value) {
                    values.push(s);
                }
            }
            ArrayElement::KeyValue(kv) => {
                if let Some(s) = literal_string(kv.value) {
                    values.push(s);
                }
            }
            _ => {}
        }
    }
    values
}

pub(crate) fn array_string_map(
    expr: &Expression<'_>,
    resolved: &ResolvedNames<'_>,
    namespace: Option<&str>,
) -> Vec<(String, String)> {
    let elements = match expr {
        Expression::Array(array) => &array.elements,
        Expression::LegacyArray(array) => &array.elements,
        Expression::Parenthesized(inner) => {
            return array_string_map(inner.expression, resolved, namespace)
        }
        _ => return Vec::new(),
    };
    let mut pairs = Vec::new();
    for element in elements.iter() {
        let ArrayElement::KeyValue(kv) = element else {
            continue;
        };
        let Some(key) = literal_string(kv.key) else {
            continue;
        };
        let value = class_const_name(kv.value, resolved, namespace)
            .or_else(|| literal_string(kv.value))
            .unwrap_or_else(|| "mixed".into());
        pairs.push((key, value));
    }
    pairs
}

pub(crate) fn property_value<'a>(item: &'a PropertyItem<'a>) -> Option<&'a Expression<'a>> {
    match item {
        PropertyItem::Concrete(concrete) => Some(concrete.value),
        PropertyItem::Abstract(_) => None,
    }
}

pub(crate) fn variable_name(bytes: &[u8]) -> String {
    owned_name(bytes)
        .unwrap_or_else(|| bytes_to_string(bytes))
        .trim_start_matches('$')
        .to_string()
}

pub(crate) fn hint_to_string(
    hint: &Hint<'_>,
    resolved: &ResolvedNames<'_>,
    namespace: Option<&str>,
) -> String {
    match hint {
        Hint::Integer(_) => "int".into(),
        Hint::Float(_) => "float".into(),
        Hint::String(_) => "string".into(),
        Hint::Bool(_) => "bool".into(),
        Hint::Array(_) => "array".into(),
        Hint::Mixed(_) => "mixed".into(),
        Hint::Null(_) => "null".into(),
        Hint::True(_) | Hint::False(_) => "bool".into(),
        Hint::Void(_) => "void".into(),
        Hint::Never(_) => "never".into(),
        Hint::Object(_) => "object".into(),
        Hint::Iterable(_) => "iterable".into(),
        Hint::Callable(_) => "callable".into(),
        Hint::Nullable(inner) => hint_to_string(inner.hint, resolved, namespace),
        Hint::Parenthesized(inner) => hint_to_string(inner.hint, resolved, namespace),
        Hint::Identifier(ident) => resolve_identifier(resolved, ident, namespace),
        Hint::Self_(_) => "self".into(),
        Hint::Static(_) => "static".into(),
        Hint::Parent(_) => "parent".into(),
        Hint::Union(union) => {
            let left = hint_to_string(union.left, resolved, namespace);
            let right = hint_to_string(union.right, resolved, namespace);
            if left == "null" {
                right
            } else if right == "null" {
                left
            } else {
                format!("{left}|{right}")
            }
        }
        Hint::Intersection(inner) => hint_to_string(inner.left, resolved, namespace),
    }
}

pub(crate) fn walk_statements<'a>(
    statements: impl IntoIterator<Item = &'a Statement<'a>>,
    visit: &mut impl FnMut(&'a Expression<'a>),
) {
    for statement in statements {
        walk_statement(statement, visit);
    }
}

fn walk_statement<'a>(statement: &'a Statement<'a>, visit: &mut impl FnMut(&'a Expression<'a>)) {
    match statement {
        Statement::Namespace(ns) => walk_statements(ns.statements().iter(), visit),
        Statement::Block(block) => walk_statements(block.statements.iter(), visit),
        Statement::Class(class) => {
            for member in class.members.iter() {
                if let mago_syntax::cst::ClassLikeMember::Method(method) = member {
                    if let mago_syntax::cst::MethodBody::Concrete(body) = &method.body {
                        walk_statements(body.statements.iter(), visit);
                    }
                }
            }
        }
        Statement::Expression(expr) => walk_expression(expr.expression, visit),
        Statement::Return(ret) => {
            if let Some(value) = ret.value {
                walk_expression(value, visit);
            }
        }
        _ => {}
    }
}

pub(crate) fn walk_expression<'a>(
    expr: &'a Expression<'a>,
    visit: &mut impl FnMut(&'a Expression<'a>),
) {
    visit(expr);
    match expr {
        Expression::Parenthesized(inner) => walk_expression(inner.expression, visit),
        Expression::Call(call) => match call {
            Call::Function(f) => {
                walk_expression(f.function, visit);
                for arg in f.argument_list.arguments.iter() {
                    walk_expression(arg_value(arg), visit);
                }
            }
            Call::Method(m) => {
                walk_expression(m.object, visit);
                for arg in m.argument_list.arguments.iter() {
                    walk_expression(arg_value(arg), visit);
                }
            }
            Call::NullSafeMethod(m) => {
                walk_expression(m.object, visit);
                for arg in m.argument_list.arguments.iter() {
                    walk_expression(arg_value(arg), visit);
                }
            }
            Call::StaticMethod(m) => {
                walk_expression(m.class, visit);
                for arg in m.argument_list.arguments.iter() {
                    walk_expression(arg_value(arg), visit);
                }
            }
        },
        Expression::Closure(closure) => walk_statements(closure.body.statements.iter(), visit),
        Expression::Instantiation(instantiation) => {
            walk_expression(instantiation.class, visit);
            if let Some(args) = &instantiation.argument_list {
                for arg in args.arguments.iter() {
                    walk_expression(arg_value(arg), visit);
                }
            }
        }
        Expression::AnonymousClass(class) => {
            for member in class.members.iter() {
                if let mago_syntax::cst::ClassLikeMember::Method(method) = member {
                    if let mago_syntax::cst::MethodBody::Concrete(body) = &method.body {
                        walk_statements(body.statements.iter(), visit);
                    }
                }
            }
        }
        Expression::Array(array) => {
            for element in array.elements.iter() {
                walk_array_element(element, visit);
            }
        }
        Expression::LegacyArray(array) => {
            for element in array.elements.iter() {
                walk_array_element(element, visit);
            }
        }
        _ => {}
    }
}

fn walk_array_element<'a>(
    element: &'a ArrayElement<'a>,
    visit: &mut impl FnMut(&'a Expression<'a>),
) {
    match element {
        ArrayElement::Value(value) => walk_expression(value.value, visit),
        ArrayElement::KeyValue(kv) => {
            walk_expression(kv.key, visit);
            walk_expression(kv.value, visit);
        }
        ArrayElement::Variadic(v) => walk_expression(v.value, visit),
        ArrayElement::Missing(_) => {}
    }
}

fn arg_value<'a>(arg: &'a Argument<'a>) -> &'a Expression<'a> {
    match arg {
        Argument::Positional(positional) => positional.value,
        Argument::Named(named) => named.value,
    }
}

pub(crate) fn method_chain_names(expr: &Expression<'_>) -> Vec<String> {
    let mut names = Vec::new();
    let mut current = expr;
    loop {
        match current {
            Expression::Call(Call::Method(m)) => {
                if let Some(name) = selector_name(&m.method) {
                    names.push(name);
                }
                current = m.object;
            }
            Expression::Call(Call::NullSafeMethod(m)) => {
                if let Some(name) = selector_name(&m.method) {
                    names.push(name);
                }
                current = m.object;
            }
            Expression::Parenthesized(inner) => current = inner.expression,
            _ => break,
        }
    }
    names
}

pub(crate) fn innermost_call<'a>(expr: &'a Expression<'a>) -> Option<&'a Call<'a>> {
    let mut current = expr;
    loop {
        match current {
            Expression::Call(Call::Method(m)) => current = m.object,
            Expression::Call(Call::NullSafeMethod(m)) => current = m.object,
            Expression::Parenthesized(inner) => current = inner.expression,
            Expression::Call(call) => return Some(call),
            _ => return None,
        }
    }
}

pub(crate) fn is_variable(expr: &Expression<'_>, name: &str) -> bool {
    match expr {
        Expression::Variable(mago_syntax::cst::Variable::Direct(var)) => {
            variable_name(var.name) == name
        }
        Expression::Parenthesized(inner) => is_variable(inner.expression, name),
        _ => false,
    }
}

pub(crate) fn static_call_class_method<'a>(
    call: &'a Call<'a>,
) -> Option<(&'a Expression<'a>, String)> {
    match call {
        Call::StaticMethod(m) => Some((m.class, selector_name(&m.method)?)),
        _ => None,
    }
}

pub(crate) fn identifier_short(expr: &Expression<'_>) -> Option<String> {
    match expr {
        Expression::Identifier(ident) => owned_name(ident.last_segment()),
        Expression::Parenthesized(inner) => identifier_short(inner.expression),
        _ => None,
    }
}
