use crate::{
    model::{TemplateManifest, TemplateRender},
    templates::country_resume::defaults::country_resume_defaults,
    templates::country_resume::sections::country_resume_sections,
    templates::country_resume::spec::{country_resume_specs, CountryResumeSpec},
};

pub fn country_resume_template_by_id(template_id: &str) -> Option<TemplateManifest> {
    country_resume_specs()
        .into_iter()
        .find(|spec| spec.id == template_id)
        .map(country_resume_template)
}

pub(crate) fn country_resume_templates() -> impl Iterator<Item = TemplateManifest> {
    country_resume_specs()
        .into_iter()
        .map(country_resume_template)
}

fn country_resume_template(spec: &CountryResumeSpec) -> TemplateManifest {
    TemplateManifest {
        schema_version: 1,
        id: spec.id.to_string(),
        name: spec.name.to_string(),
        version: None,
        description: spec.description.to_string(),
        sections: country_resume_sections(),
        defaults: country_resume_defaults(spec),
        render: TemplateRender {
            html: spec.html.to_string(),
            css: spec.css.to_string(),
            html_file: None,
            css_file: None,
            pdf_filename: format!("{}.pdf", spec.id),
        },
    }
}
