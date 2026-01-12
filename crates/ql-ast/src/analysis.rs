use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::rows::TableBatch;

pub fn second_pass(batch: &mut TableBatch) {
    resolve_has_test(batch);
    resolve_implements(batch);
    resolve_comment_attachments(batch);
}

fn resolve_has_test(batch: &mut TableBatch) {
    let mut test_keys_by_package: HashMap<String, HashSet<String>> = HashMap::new();

    for function in &batch.functions {
        if let Some(key) = test_key(&function.name) {
            test_keys_by_package
                .entry(package_key(&function.file))
                .or_default()
                .insert(key);
        }
    }

    for function in &mut batch.functions {
        let package = package_key(&function.file);
        let Some(test_keys) = test_keys_by_package.get(&package) else {
            continue;
        };

        if test_keys.contains(&function_key(&function.name)) {
            function.has_test = true;
        }
    }
}

fn resolve_implements(batch: &mut TableBatch) {
    for row in &mut batch.structs {
        row.implements = normalize_csv_list(&row.implements);
    }
}

fn resolve_comment_attachments(batch: &mut TableBatch) {
    for comment in &mut batch.comments {
        let nearest_function = batch
            .functions
            .iter()
            .filter(|row| row.file == comment.file && row.line > comment.line)
            .min_by_key(|row| row.line);
        let nearest_struct = batch
            .structs
            .iter()
            .filter(|row| row.file == comment.file && row.line > comment.line)
            .min_by_key(|row| row.line);

        comment.attached_to = match (nearest_function, nearest_struct) {
            (Some(function), None) => function.name.clone(),
            (None, Some(struct_row)) => struct_row.name.clone(),
            (Some(function), Some(struct_row)) if function.line <= struct_row.line => {
                function.name.clone()
            }
            (_, Some(struct_row)) => struct_row.name.clone(),
            _ => String::new(),
        };
    }
}

fn package_key(file: &str) -> String {
    Path::new(file)
        .parent()
        .map(|parent| parent.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn function_key(name: &str) -> String {
    name.to_ascii_lowercase()
}

fn test_key(name: &str) -> Option<String> {
    if let Some(stripped) = name.strip_prefix("test_") {
        return Some(function_key(stripped));
    }
    if let Some(stripped) = name.strip_suffix("_test") {
        return Some(function_key(stripped));
    }
    if let Some(stripped) = name.strip_prefix("Test") {
        if stripped.is_empty() {
            return None;
        }
        return Some(function_key(stripped));
    }
    None
}

fn normalize_csv_list(value: &str) -> String {
    let mut items = Vec::new();
    let mut seen = HashSet::new();

    for item in value.split(',') {
        let item = item.trim();
        if item.is_empty() {
            continue;
        }
        if seen.insert(item.to_string()) {
            items.push(item.to_string());
        }
    }

    items.join(",")
}

#[cfg(test)]
mod tests {
    use crate::rows::{CommentRow, FunctionRow, StructRow, TableBatch};

    use super::second_pass;

    #[test]
    fn resolves_has_test_comments_and_implements() {
        let mut batch = TableBatch::new("");
        batch.functions.push(FunctionRow {
            file: "src/lib.rs".to_string(),
            line: 5,
            name: "add".to_string(),
            visibility: "private".to_string(),
            param_count: 0,
            return_type: String::new(),
            complexity: 1,
            has_test: false,
        });
        batch.functions.push(FunctionRow {
            file: "src/lib.rs".to_string(),
            line: 20,
            name: "test_add".to_string(),
            visibility: "private".to_string(),
            param_count: 0,
            return_type: String::new(),
            complexity: 1,
            has_test: false,
        });
        batch.structs.push(StructRow {
            file: "src/lib.rs".to_string(),
            line: 10,
            name: "User".to_string(),
            field_count: 2,
            visibility: "public".to_string(),
            implements: "Display, Display".to_string(),
        });
        batch.comments.push(CommentRow {
            file: "src/lib.rs".to_string(),
            line: 1,
            text: "// docs".to_string(),
            attached_to: String::new(),
            is_doc: true,
        });

        second_pass(&mut batch);

        assert!(batch.functions[0].has_test);
        assert_eq!(batch.structs[0].implements, "Display");
        assert_eq!(batch.comments[0].attached_to, "add");
    }
}
