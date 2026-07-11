use serde_json::{json, Value};

use crate::{
    model::{FieldType, TemplateManifest, TemplateRender, TemplateSection, BUILTIN_CLASSIC_ID},
    template_fields::{field, list, object_list, section, textarea},
};

pub fn classic_template() -> TemplateManifest {
    TemplateManifest {
        schema_version: 1,
        id: BUILTIN_CLASSIC_ID.to_string(),
        name: "Classic Resume".to_string(),
        version: None,
        description: "Two-column resume template with contact, sidebar, experience, projects, education, and activities.".to_string(),
        sections: classic_sections(),
        defaults: classic_defaults(),
        render: TemplateRender {
            html: CLASSIC_HTML.to_string(),
            css: CLASSIC_CSS.to_string(),
            html_file: None,
            css_file: None,
            pdf_filename: "resume.pdf".to_string(),
        },
    }
}

fn classic_sections() -> Vec<TemplateSection> {
    vec![
        section(
            "profile",
            "Profile",
            "Core identity and profile image.",
            vec![
                field(
                    "document_title",
                    "profile.document_title",
                    "Document title",
                    FieldType::Text,
                    true,
                ),
                field(
                    "full_name",
                    "profile.full_name",
                    "Full name",
                    FieldType::Text,
                    true,
                ),
                field(
                    "title",
                    "profile.title",
                    "Resume title",
                    FieldType::Text,
                    true,
                ),
                field("photo", "profile.photo", "Photo", FieldType::Asset, false),
                field(
                    "photo_alt",
                    "profile.photo_alt",
                    "Photo alt text",
                    FieldType::Text,
                    false,
                ),
            ],
        ),
        section(
            "contact",
            "Contact",
            "Links and icon assets.",
            vec![object_list(
                "contact",
                "contact",
                "Contact items",
                "Contact",
                vec![
                    field("label", "label", "Label", FieldType::Text, true),
                    field("text", "text", "Display text", FieldType::Text, true),
                    field("url", "url", "URL", FieldType::Url, false),
                    field("icon", "icon", "Icon", FieldType::Asset, false),
                    field("alt", "alt", "Icon alt text", FieldType::Text, false),
                    field("target", "target", "Link target", FieldType::Text, false),
                ],
            )],
        ),
        section(
            "sidebar",
            "Sidebar",
            "Languages, skills, and strengths.",
            vec![
                list("languages", "sidebar.languages", "Languages", false),
                list("skills", "sidebar.skills", "Skills", false),
                list("strengths", "sidebar.strengths", "Strengths", false),
            ],
        ),
        section(
            "summary",
            "Summary",
            "Main professional summary.",
            vec![textarea("summary", "summary", "Summary", false, 9)],
        ),
        section(
            "experience",
            "Experience",
            "Roles and bullet points.",
            vec![object_list(
                "experience",
                "experience",
                "Experience",
                "Role",
                vec![
                    field("company", "company", "Company", FieldType::Text, true),
                    field("role", "role", "Role", FieldType::Text, true),
                    field("date", "date", "Date range", FieldType::Text, false),
                    list("bullets", "bullets", "Bullets", false),
                ],
            )],
        ),
        section(
            "projects",
            "Projects",
            "Project list.",
            vec![list("projects", "projects", "Projects", false)],
        ),
        section(
            "education",
            "Education",
            "Institutions and qualifications.",
            vec![object_list(
                "education",
                "education",
                "Education",
                "Education",
                vec![
                    field(
                        "institution",
                        "institution",
                        "Institution",
                        FieldType::Text,
                        true,
                    ),
                    field("date", "date", "Date range", FieldType::Text, false),
                    field("degree", "degree", "Degree", FieldType::Text, true),
                ],
            )],
        ),
        section(
            "activities",
            "Activities",
            "Additional activities.",
            vec![list("activities", "activities", "Activities", false)],
        ),
    ]
}

fn classic_defaults() -> Value {
    json!({
        "profile": {
            "document_title": "Resume",
            "full_name": "Your Name",
            "title": "Your Professional Title",
            "photo": "",
            "photo_alt": ""
        },
        "contact": [],
        "sidebar": {
            "languages": [],
            "skills": [],
            "strengths": []
        },
        "summary": "",
        "experience": [],
        "projects": [],
        "education": [],
        "activities": []
    })
}

const CLASSIC_HTML: &str = include_str!("../templates/classic-resume.html");
const CLASSIC_CSS: &str = include_str!("../templates/classic-resume.css");
