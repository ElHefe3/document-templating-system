use anyhow::{bail, Context, Result};
#[cfg(test)]
use serde_json::Map;
use serde_json::Value;

use crate::model::{FieldDefinition, FieldType, TemplateManifest};

pub fn validate_document(template: &TemplateManifest, document: &Value) -> Result<()> {
    for section in &template.sections {
        for field in &section.fields {
            validate_field(document, field)?;
        }
    }
    Ok(())
}

pub fn document_summary(template: &TemplateManifest, document: &Value) -> Vec<String> {
    let title = get_path(document, "profile.full_name")
        .and_then(Value::as_str)
        .or_else(|| get_path(document, "profile.candidate_name").and_then(Value::as_str))
        .or_else(|| get_path(document, "title").and_then(Value::as_str))
        .unwrap_or("Untitled document");
    let subtitle = get_path(document, "profile.title")
        .and_then(Value::as_str)
        .or_else(|| get_path(document, "profile.target_role").and_then(Value::as_str))
        .or_else(|| get_path(document, "profile.country").and_then(Value::as_str))
        .unwrap_or(&template.name);
    vec![
        format!("Document: {title}"),
        format!("Template: {}", template.name),
        format!("Subtitle: {subtitle}"),
        format!("Sections: {}", template.sections.len()),
    ]
}

pub fn get_path<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = value;
    for segment in path.split('.') {
        if segment.is_empty() {
            return None;
        }
        if let Ok(index) = segment.parse::<usize>() {
            current = current.as_array()?.get(index)?;
        } else {
            current = current.as_object()?.get(segment)?;
        }
    }
    Some(current)
}

#[cfg(test)]
pub fn set_path(value: &mut Value, path: &str, next_value: Value) -> Result<()> {
    let parts: Vec<&str> = path.split('.').filter(|part| !part.is_empty()).collect();
    if parts.is_empty() {
        bail!("path is required");
    }
    let mut current = value;
    for part in &parts[..parts.len() - 1] {
        if let Ok(index) = part.parse::<usize>() {
            let array = current
                .as_array_mut()
                .with_context(|| format!("path segment {part} expected an array"))?;
            current = array
                .get_mut(index)
                .with_context(|| format!("array index out of range: {part}"))?;
        } else {
            if !current.is_object() {
                *current = Value::Object(Map::new());
            }
            current = current
                .as_object_mut()
                .unwrap()
                .entry((*part).to_string())
                .or_insert(Value::Object(Map::new()));
        }
    }
    let final_part = parts[parts.len() - 1];
    if let Ok(index) = final_part.parse::<usize>() {
        let array = current
            .as_array_mut()
            .with_context(|| format!("path segment {final_part} expected an array"))?;
        if index >= array.len() {
            bail!("array index out of range: {final_part}");
        }
        array[index] = next_value;
    } else {
        if !current.is_object() {
            *current = Value::Object(Map::new());
        }
        current
            .as_object_mut()
            .unwrap()
            .insert(final_part.to_string(), next_value);
    }
    Ok(())
}

fn validate_field(document: &Value, field: &FieldDefinition) -> Result<()> {
    let value = get_path(document, &field.path);
    if field.required && value.map(is_blank).unwrap_or(true) {
        bail!("{} is required", field.path);
    }
    let Some(value) = value else {
        return Ok(());
    };
    match field.field_type {
        FieldType::Text | FieldType::Textarea | FieldType::Url | FieldType::Asset => {
            if !value.is_string() {
                bail!("{} must be a string", field.path);
            }
        }
        FieldType::List => {
            let items = value
                .as_array()
                .with_context(|| format!("{} must be a list", field.path))?;
            if items.iter().any(|item| !item.is_string()) {
                bail!("{} must contain only strings", field.path);
            }
        }
        FieldType::ObjectList => {
            let items = value
                .as_array()
                .with_context(|| format!("{} must be a list", field.path))?;
            for (index, item) in items.iter().enumerate() {
                if !item.is_object() {
                    bail!("{}.{} must be an object", field.path, index);
                }
                for child in &field.fields {
                    let child_path = format!("{}.{}.{}", field.path, index, child.path);
                    let mut child_field = child.clone();
                    child_field.path = child_path;
                    validate_field(document, &child_field)?;
                }
            }
        }
    }
    Ok(())
}

fn is_blank(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::String(text) => text.trim().is_empty(),
        Value::Array(items) => items.is_empty(),
        Value::Object(items) => items.is_empty(),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::classic_template;
    use serde_json::json;

    #[test]
    fn rejects_invalid_field_type() {
        let template = classic_template();
        let mut document = template.defaults.clone();
        set_path(&mut document, "profile.full_name", json!(["not a string"])).unwrap();
        let err = validate_document(&template, &document).unwrap_err();
        assert!(err
            .to_string()
            .contains("profile.full_name must be a string"));
    }

    #[test]
    fn can_set_dynamic_path() {
        let mut document = json!({"profile": {"full_name": ""}});
        set_path(&mut document, "profile.full_name", json!("A Person")).unwrap();
        assert_eq!(
            get_path(&document, "profile.full_name").and_then(Value::as_str),
            Some("A Person")
        );
    }
}
