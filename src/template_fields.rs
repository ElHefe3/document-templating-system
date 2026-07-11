use crate::model::{FieldDefinition, FieldType, TemplateSection};

pub(crate) fn section(
    id: &str,
    label: &str,
    description: &str,
    fields: Vec<FieldDefinition>,
) -> TemplateSection {
    TemplateSection {
        id: id.to_string(),
        label: label.to_string(),
        description: description.to_string(),
        fields,
    }
}

pub(crate) fn field(
    id: &str,
    path: &str,
    label: &str,
    field_type: FieldType,
    required: bool,
) -> FieldDefinition {
    FieldDefinition {
        id: id.to_string(),
        path: path.to_string(),
        label: label.to_string(),
        field_type,
        required,
        rows: None,
        item_label: None,
        fields: vec![],
    }
}

pub(crate) fn textarea(
    id: &str,
    path: &str,
    label: &str,
    required: bool,
    rows: u16,
) -> FieldDefinition {
    FieldDefinition {
        rows: Some(rows),
        ..field(id, path, label, FieldType::Textarea, required)
    }
}

pub(crate) fn list(id: &str, path: &str, label: &str, required: bool) -> FieldDefinition {
    field(id, path, label, FieldType::List, required)
}

pub(crate) fn object_list(
    id: &str,
    path: &str,
    label: &str,
    item_label: &str,
    fields: Vec<FieldDefinition>,
) -> FieldDefinition {
    FieldDefinition {
        id: id.to_string(),
        path: path.to_string(),
        label: label.to_string(),
        field_type: FieldType::ObjectList,
        required: false,
        rows: None,
        item_label: Some(item_label.to_string()),
        fields,
    }
}
