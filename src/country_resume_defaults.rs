use serde_json::{json, Value};

use crate::country_resume_spec::CountryResumeSpec;

pub(crate) fn country_resume_defaults(spec: &CountryResumeSpec) -> Value {
    json!({
        "profile": {
            "document_title": spec.document_title,
            "full_name": "Your Name",
            "title": spec.title,
            "country": spec.country,
            "location": spec.location,
            "email": "you@example.com",
            "phone": "",
            "linkedin": "",
            "portfolio": "",
            "photo": "",
            "photo_alt": ""
        },
        "personal_details": personal_detail_values(spec.personal_details),
        "summary": spec.summary,
        "experience": [
            {
                "company": "Company Name",
                "role": spec.title,
                "location": spec.location,
                "date": "2024 - Present",
                "bullets": [
                    "Describe a measurable result, business impact, or technical improvement.",
                    "Name the technologies, systems, or stakeholders involved.",
                    "Keep each bullet concise and outcome-focused."
                ]
            }
        ],
        "skills": string_values(spec.skills),
        "languages": string_values(spec.languages),
        "education": [
            {
                "institution": "Institution Name",
                "degree": "Degree or qualification",
                "location": "",
                "date": ""
            }
        ],
        "certifications": [],
        "projects": [
            {
                "name": "Selected Project",
                "url": "",
                "description": "Briefly describe the project, your role, and the result."
            }
        ],
        "template_guidance": spec.guidance
    })
}

fn personal_detail_values(items: &[(&str, &str)]) -> Value {
    Value::Array(
        items
            .iter()
            .map(|(label, value)| {
                json!({
                    "label": *label,
                    "value": *value
                })
            })
            .collect(),
    )
}

fn string_values(items: &[&str]) -> Value {
    Value::Array(
        items
            .iter()
            .map(|item| Value::String((*item).to_string()))
            .collect(),
    )
}
