use mago_allocator::LocalArena;
use mago_docblock::document::{Document, TagKind};
use mago_docblock::tag::parse_property_tag;
use mago_syntax::cst::{Program, Trivia, TriviaKind};

use crate::ir::PhpDocField;

#[derive(Debug, Clone, Default)]
pub(crate) struct ClassDoc {
    pub properties: Vec<PhpDocField>,
    pub reads: Vec<PhpDocField>,
}

pub(crate) fn class_doc_between(
    arena: &LocalArena,
    program: &Program<'_>,
    from: u32,
    to: u32,
) -> ClassDoc {
    let Some(trivia) = docblock_between(program, from, to) else {
        return ClassDoc::default();
    };
    parse_class_doc(arena, trivia).unwrap_or_default()
}

fn docblock_between<'a>(program: &'a Program<'a>, from: u32, to: u32) -> Option<&'a Trivia<'a>> {
    program
        .trivia
        .iter()
        .filter(|trivia| trivia.kind == TriviaKind::DocBlockComment)
        .filter(|trivia| trivia.span.start.offset >= from && trivia.span.end.offset <= to)
        .max_by_key(|trivia| trivia.span.end.offset)
}

fn parse_class_doc(arena: &LocalArena, trivia: &Trivia<'_>) -> Option<ClassDoc> {
    #[allow(deprecated)]
    let document = mago_docblock::parse_trivia(arena, trivia).ok()?;
    Some(doc_from_document(&document))
}

fn doc_from_document(document: &Document<'_>) -> ClassDoc {
    let mut doc = ClassDoc::default();
    for tag in document.get_tags() {
        let (is_read, is_write, is_property) = match tag.kind {
            TagKind::Property => (false, false, true),
            TagKind::PropertyWrite => (false, true, true),
            TagKind::PropertyRead => (true, false, false),
            _ => (false, false, false),
        };
        if !is_property && !is_read {
            continue;
        }
        let Ok(parsed) =
            parse_property_tag(tag.description, tag.description_span, is_read, is_write)
        else {
            continue;
        };
        let Some(field) = property_to_field(&parsed) else {
            continue;
        };
        if is_read {
            doc.reads.push(field);
        } else {
            doc.properties.push(field);
        }
    }
    doc
}

fn property_to_field(tag: &mago_docblock::tag::PropertyTag) -> Option<PhpDocField> {
    let name = std::str::from_utf8(&tag.variable.name)
        .ok()?
        .trim()
        .trim_start_matches('$');
    if name.is_empty() {
        return None;
    }
    let raw_type = tag
        .type_string
        .as_ref()
        .and_then(|t| std::str::from_utf8(&t.value).ok())
        .unwrap_or("mixed")
        .trim()
        .to_string();
    let (ty, nullable, array_type, key_type) = parse_phpdoc_type(&raw_type);
    Some(PhpDocField {
        name: name.to_string(),
        r#type: ty,
        nullable,
        array_type,
        key_type,
    })
}

pub(crate) fn parse_phpdoc_type(raw: &str) -> (String, bool, Option<String>, Option<String>) {
    let nullable =
        raw.ends_with("|null") || raw.starts_with("null|") || raw == "null" || raw.contains('?');
    let mut ty = raw
        .replace("|null", "")
        .replace("null|", "")
        .replace('?', "");
    ty = ty.trim().trim_start_matches('\\').to_string();

    let mut array_type = None;
    let mut key_type = None;

    if ty.starts_with("Collection<") && ty.ends_with('>') {
        array_type = Some("Collection".into());
        let inner = &ty[11..ty.len() - 1];
        if let Some((key, rest)) = inner.split_once(',') {
            key_type = Some(key.trim().to_string());
            ty = rest.trim().trim_start_matches('\\').to_string();
        } else {
            ty = inner.trim().trim_start_matches('\\').to_string();
        }
    } else if ty.starts_with("array<") && ty.ends_with('>') {
        array_type = Some("array".into());
        let inner = &ty[6..ty.len() - 1];
        if let Some((key, rest)) = inner.split_once(',') {
            key_type = Some(key.trim().to_string());
            ty = rest.trim().trim_start_matches('\\').to_string();
        } else {
            ty = inner.trim().trim_start_matches('\\').to_string();
        }
    } else if ty.starts_with("list<") && ty.ends_with('>') {
        array_type = Some("array".into());
        ty = ty[5..ty.len() - 1]
            .trim()
            .trim_start_matches('\\')
            .to_string();
    } else if ty.ends_with("[]") {
        array_type = Some("array".into());
        ty = ty[..ty.len() - 2]
            .trim()
            .trim_start_matches('\\')
            .to_string();
    }

    if ty.is_empty() {
        ty = "mixed".into();
    }
    (ty, nullable, array_type, key_type)
}
pub(crate) fn resolve_phpdoc_class(
    ty: &str,
    uses: &std::collections::BTreeMap<String, String>,
    namespace: &str,
) -> String {
    let trimmed = ty.trim().trim_start_matches('\\');
    if trimmed.is_empty() || !is_probably_class(trimmed) {
        return trimmed.to_string();
    }
    if trimmed.contains('\\') {
        return trimmed.to_string();
    }
    if let Some(fqcn) = uses.get(trimmed) {
        return fqcn.clone();
    }
    if namespace.is_empty() {
        trimmed.to_string()
    } else {
        format!("{namespace}\\{trimmed}")
    }
}

fn is_probably_class(ty: &str) -> bool {
    let lower = ty.to_ascii_lowercase();
    let builtins = [
        "string", "bool", "boolean", "int", "integer", "float", "double", "mixed", "object",
        "array", "callable", "iterable", "void", "null", "resource", "true", "false", "self",
        "static", "$this",
    ];
    if builtins.contains(&lower.as_str()) {
        return false;
    }
    ty.contains('\\') || ty.chars().next().is_some_and(|c| c.is_uppercase())
}
