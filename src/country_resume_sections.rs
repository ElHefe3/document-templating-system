use crate::{
    model::{FieldType, TemplateSection},
    template_fields::{field, list, object_list, section, textarea},
};

pub(crate) fn country_resume_sections() -> Vec<TemplateSection> {
    vec![
        section(
            "profile",
            "Profile",
            "Core CV identity and contact details.",
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
                    "Professional title",
                    FieldType::Text,
                    true,
                ),
                field(
                    "location",
                    "profile.location",
                    "Location",
                    FieldType::Text,
                    false,
                ),
                field("email", "profile.email", "Email", FieldType::Text, false),
                field("phone", "profile.phone", "Phone", FieldType::Text, false),
                field(
                    "linkedin",
                    "profile.linkedin",
                    "LinkedIn",
                    FieldType::Url,
                    false,
                ),
                field(
                    "portfolio",
                    "profile.portfolio",
                    "Portfolio / GitHub",
                    FieldType::Url,
                    false,
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
            "personal",
            "Personal Details",
            "Country-specific optional personal details.",
            vec![object_list(
                "personal_details",
                "personal_details",
                "Personal details",
                "Detail",
                vec![
                    field("label", "label", "Label", FieldType::Text, true),
                    field("value", "value", "Value", FieldType::Text, false),
                ],
            )],
        ),
        section(
            "summary",
            "Summary",
            "Professional profile summary.",
            vec![textarea("summary", "summary", "Summary", false, 8)],
        ),
        section(
            "experience",
            "Experience",
            "Roles and achievements.",
            vec![object_list(
                "experience",
                "experience",
                "Experience",
                "Role",
                vec![
                    field("company", "company", "Company", FieldType::Text, true),
                    field("role", "role", "Role", FieldType::Text, true),
                    field("location", "location", "Location", FieldType::Text, false),
                    field("date", "date", "Date range", FieldType::Text, false),
                    list("bullets", "bullets", "Bullets", false),
                ],
            )],
        ),
        section(
            "skills",
            "Skills",
            "Skills and technologies.",
            vec![
                list("skills", "skills", "Skills", false),
                list("languages", "languages", "Languages", false),
            ],
        ),
        section(
            "education",
            "Education",
            "Degrees and qualifications.",
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
                    field("degree", "degree", "Degree", FieldType::Text, true),
                    field("location", "location", "Location", FieldType::Text, false),
                    field("date", "date", "Date range", FieldType::Text, false),
                ],
            )],
        ),
        section(
            "certifications",
            "Certifications",
            "Certifications and licences.",
            vec![list(
                "certifications",
                "certifications",
                "Certifications",
                false,
            )],
        ),
        section(
            "projects",
            "Projects",
            "Selected projects.",
            vec![object_list(
                "projects",
                "projects",
                "Projects",
                "Project",
                vec![
                    field("name", "name", "Name", FieldType::Text, true),
                    field("url", "url", "URL", FieldType::Url, false),
                    textarea("description", "description", "Description", false, 4),
                ],
            )],
        ),
    ]
}
