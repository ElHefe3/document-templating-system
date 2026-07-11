use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use crate::builtin_templates::{classic_template, country_resume_template_by_id};
pub use crate::document_model::{document_summary, validate_document};
pub use crate::template_catalog::{
    available_templates, load_template, load_template_from_file, local_template_exists,
};
pub use crate::template_manifest_validation::{
    hydrate_template_render_files, validate_template_manifest,
};
pub use crate::workspace::{init_workspace, set_workspace_template_with_manifest, Workspace};

pub const WORKSPACE_MANIFEST: &str = "document-templating-system.json";
pub const DOCUMENT_FILE: &str = "document.json";
pub const TEMPLATE_MANIFEST_FILE: &str = "document-template.json";
pub const BUILTIN_CLASSIC_ID: &str = "classic-resume";
pub const BUILTIN_RESUME_GERMANY_ID: &str = "resume-germany";
pub const BUILTIN_RESUME_NETHERLANDS_ID: &str = "resume-netherlands";
pub const BUILTIN_RESUME_INDONESIA_ID: &str = "resume-indonesia";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateSummary {
    pub id: String,
    pub name: String,
    pub description: String,
    pub built_in: bool,
    #[serde(default = "default_template_source")]
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateManifest {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub sections: Vec<TemplateSection>,
    #[serde(default)]
    pub defaults: Value,
    pub render: TemplateRender,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateSection {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub fields: Vec<FieldDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDefinition {
    pub id: String,
    pub path: String,
    pub label: String,
    #[serde(rename = "type")]
    pub field_type: FieldType,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub rows: Option<u16>,
    #[serde(default)]
    pub item_label: Option<String>,
    #[serde(default)]
    pub fields: Vec<FieldDefinition>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldType {
    Text,
    Textarea,
    Url,
    Asset,
    List,
    ObjectList,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateRender {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub html: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub css: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub html_file: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub css_file: Option<String>,
    #[serde(default = "default_pdf_filename")]
    pub pdf_filename: String,
}

fn default_pdf_filename() -> String {
    "document.pdf".to_string()
}

fn default_template_source() -> String {
    "workspace".to_string()
}

impl TemplateManifest {
    pub fn summary_with_source(&self, built_in: bool, source: &str) -> TemplateSummary {
        TemplateSummary {
            id: self.id.clone(),
            name: self.name.clone(),
            description: self.description.clone(),
            built_in,
            source: source.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document_model::get_path;

    #[test]
    fn validates_classic_defaults() {
        let template = classic_template();
        validate_document(&template, &template.defaults).unwrap();
    }

    #[test]
    fn validates_country_resume_defaults() {
        for template_id in [
            BUILTIN_RESUME_GERMANY_ID,
            BUILTIN_RESUME_NETHERLANDS_ID,
            BUILTIN_RESUME_INDONESIA_ID,
        ] {
            let template = country_resume_template_by_id(template_id).unwrap();
            validate_document(&template, &template.defaults).unwrap();
            assert!(get_path(&template.defaults, "experience")
                .and_then(Value::as_array)
                .is_some_and(|items| !items.is_empty()));
            assert_eq!(template.render.pdf_filename, format!("{template_id}.pdf"));
        }
    }

    #[test]
    fn country_resume_templates_have_distinct_renderers() {
        let germany = country_resume_template_by_id(BUILTIN_RESUME_GERMANY_ID).unwrap();
        let netherlands = country_resume_template_by_id(BUILTIN_RESUME_NETHERLANDS_ID).unwrap();
        let indonesia = country_resume_template_by_id(BUILTIN_RESUME_INDONESIA_ID).unwrap();

        assert!(germany.render.html.contains("lebenslauf"));
        assert!(netherlands.render.html.contains("nl-cv"));
        assert!(indonesia.render.html.contains("id-cv"));
        assert_ne!(germany.render.css, netherlands.render.css);
        assert_ne!(netherlands.render.css, indonesia.render.css);
        assert_ne!(germany.render.css, indonesia.render.css);
    }

    #[test]
    fn builtin_template_pdf_css_avoids_wkhtmltopdf_unstable_layout_primitives() {
        for template in [
            classic_template(),
            country_resume_template_by_id(BUILTIN_RESUME_GERMANY_ID).unwrap(),
            country_resume_template_by_id(BUILTIN_RESUME_NETHERLANDS_ID).unwrap(),
            country_resume_template_by_id(BUILTIN_RESUME_INDONESIA_ID).unwrap(),
        ] {
            let pdf_css = template
                .render
                .css
                .split("@media screen")
                .next()
                .unwrap_or(&template.render.css)
                .to_ascii_lowercase();

            for unstable in [
                "display: grid",
                "display: flex",
                "grid-template",
                "columns:",
            ] {
                assert!(
                    !pdf_css.contains(unstable),
                    "{} PDF CSS should not contain {unstable}",
                    template.id
                );
            }
        }
    }

    #[test]
    fn template_summary_marks_builtin() {
        let summary = classic_template().summary_with_source(true, "built-in");
        assert_eq!(summary.id, BUILTIN_CLASSIC_ID);
        assert!(summary.built_in);
    }

    #[test]
    fn available_templates_includes_classic() {
        let templates = available_templates(None);
        assert!(templates
            .iter()
            .any(|template| template.id == BUILTIN_CLASSIC_ID));
        assert!(templates
            .iter()
            .any(|template| template.id == BUILTIN_RESUME_GERMANY_ID));
        assert!(templates
            .iter()
            .any(|template| template.id == BUILTIN_RESUME_NETHERLANDS_ID));
        assert!(templates
            .iter()
            .any(|template| template.id == BUILTIN_RESUME_INDONESIA_ID));
    }
}
