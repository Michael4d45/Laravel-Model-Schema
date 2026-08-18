use mago_names::ResolvedNames;
use mago_span::HasSpan;
use mago_syntax::cst::{Identifier, LocalIdentifier};

pub(crate) fn owned_name(bytes: &[u8]) -> Option<String> {
    let s = std::str::from_utf8(bytes).ok()?.trim();
    if s.is_empty() {
        return None;
    }
    let trimmed = s.trim_start_matches('\\');
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

pub(crate) fn bytes_to_string(bytes: &[u8]) -> String {
    owned_name(bytes).unwrap_or_else(|| String::from_utf8_lossy(bytes).into_owned())
}

pub(crate) fn short_name(fqcn: &str) -> &str {
    fqcn.rsplit('\\').next().unwrap_or(fqcn)
}

pub(crate) fn join_fqcn(prefix: &str, name: &str) -> String {
    let prefix = prefix.trim_start_matches('\\').trim_end_matches('\\');
    let name = name.trim_start_matches('\\');
    if prefix.is_empty() {
        name.to_string()
    } else if name.is_empty() {
        prefix.to_string()
    } else {
        format!("{prefix}\\{name}")
    }
}

pub(crate) fn resolve_local(
    resolved: &ResolvedNames<'_>,
    name: &LocalIdentifier<'_>,
    namespace: Option<&str>,
) -> Option<String> {
    if let Some(bytes) = resolved.resolve(&name.span.start) {
        return owned_name(bytes);
    }
    let local = owned_name(name.value)?;
    match namespace {
        Some(ns) if !ns.is_empty() => Some(format!("{ns}\\{local}")),
        _ => Some(local),
    }
}

pub(crate) fn resolve_identifier(
    resolved: &ResolvedNames<'_>,
    ident: &Identifier<'_>,
    namespace: Option<&str>,
) -> String {
    let bytes = resolved
        .resolve(&ident.span().start)
        .unwrap_or_else(|| ident.value());
    let name = owned_name(bytes).unwrap_or_else(|| bytes_to_string(ident.value()));
    if name.contains('\\') {
        name
    } else if let Some(ns) = namespace.filter(|ns| !ns.is_empty()) {
        format!("{ns}\\{name}")
    } else {
        name
    }
}

pub(crate) fn snake_case(name: &str) -> String {
    let mut out = String::new();
    for (i, ch) in name.chars().enumerate() {
        if ch.is_uppercase() {
            if i > 0 {
                out.push('_');
            }
            out.extend(ch.to_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}

pub(crate) fn camel_case(name: &str) -> String {
    let mut out = String::new();
    let mut upper = false;
    for (i, ch) in name.chars().enumerate() {
        if ch == '_' {
            upper = true;
            continue;
        }
        if i == 0 {
            out.extend(ch.to_lowercase());
        } else if upper {
            out.extend(ch.to_uppercase());
            upper = false;
        } else {
            out.push(ch);
        }
    }
    out
}

pub(crate) fn pluralize(word: &str) -> String {
    let lower = word.to_ascii_lowercase();
    if lower.ends_with("ies")
        || lower.ends_with("ses")
        || lower.ends_with("xes")
        || lower.ends_with("zes")
        || lower.ends_with("ches")
        || lower.ends_with("shes")
    {
        return word.to_string();
    }
    if lower.ends_with('y')
        && word
            .chars()
            .nth_back(1)
            .is_some_and(|c| !matches!(c, 'a' | 'e' | 'i' | 'o' | 'u'))
    {
        let mut s = word[..word.len() - 1].to_string();
        s.push_str("ies");
        return s;
    }
    if lower.ends_with("s")
        || lower.ends_with("x")
        || lower.ends_with("z")
        || lower.ends_with("ch")
        || lower.ends_with("sh")
    {
        return format!("{word}es");
    }
    format!("{word}s")
}

pub(crate) fn guess_table_name(class_fqcn: &str) -> String {
    pluralize(&snake_case(short_name(class_fqcn)))
}
