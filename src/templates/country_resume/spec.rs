use crate::model::{
    BUILTIN_RESUME_GERMANY_ID, BUILTIN_RESUME_INDONESIA_ID, BUILTIN_RESUME_NETHERLANDS_ID,
};

pub(crate) struct CountryResumeSpec {
    pub(crate) id: &'static str,
    pub(crate) name: &'static str,
    pub(crate) description: &'static str,
    pub(crate) country: &'static str,
    pub(crate) document_title: &'static str,
    pub(crate) title: &'static str,
    pub(crate) location: &'static str,
    pub(crate) summary: &'static str,
    pub(crate) personal_details: &'static [(&'static str, &'static str)],
    pub(crate) skills: &'static [&'static str],
    pub(crate) languages: &'static [&'static str],
    pub(crate) guidance: &'static str,
    pub(crate) html: &'static str,
    pub(crate) css: &'static str,
}

pub(crate) fn country_resume_specs() -> [&'static CountryResumeSpec; 3] {
    [&GERMANY_RESUME, &NETHERLANDS_RESUME, &INDONESIA_RESUME]
}

const GERMANY_RESUME: CountryResumeSpec = CountryResumeSpec {
    id: BUILTIN_RESUME_GERMANY_ID,
    name: "Germany Resume",
    description: "German Lebenslauf-style resume with optional photo and personal-details block.",
    country: "Germany",
    document_title: "Lebenslauf",
    title: "Software Engineer",
    location: "Berlin, Germany",
    summary: "Software engineer with experience building reliable production systems, backend services, and cross-functional delivery.",
    personal_details: &[
        ("Work authorization", ""),
        ("Nationality", ""),
        ("Date of birth", ""),
        ("Availability", ""),
    ],
    skills: &[
        "Backend development",
        "Distributed systems",
        "Cloud platforms",
        "CI/CD",
        "Databases",
    ],
    languages: &["English - Professional", "German - Add level"],
    guidance: "German CVs are commonly reverse chronological and often use a clear tabular structure. A professional photo and personal details are optional and context-dependent; include only what you are comfortable sharing and what fits the employer.",
    html: GERMANY_RESUME_HTML,
    css: GERMANY_RESUME_CSS,
};

const NETHERLANDS_RESUME: CountryResumeSpec = CountryResumeSpec {
    id: BUILTIN_RESUME_NETHERLANDS_ID,
    name: "Netherlands Resume",
    description: "Concise Dutch-market CV focused on profile, experience, skills, and work authorization.",
    country: "Netherlands",
    document_title: "Curriculum Vitae",
    title: "Software Engineer",
    location: "Amsterdam, Netherlands",
    summary: "Software engineer focused on pragmatic delivery, maintainable systems, and clear collaboration across product and engineering teams.",
    personal_details: &[
        ("Work authorization", ""),
        ("Availability", ""),
        ("Preferred work mode", "Hybrid / remote"),
    ],
    skills: &[
        "Backend development",
        "Full-stack delivery",
        "Cloud platforms",
        "Automated testing",
        "Observability",
    ],
    languages: &["English - Professional", "Dutch - Add level if applicable"],
    guidance: "Dutch CVs are usually concise, direct, and achievement-focused. Personal details and photos are generally optional; prioritize relevant experience, technologies, outcomes, and links.",
    html: NETHERLANDS_RESUME_HTML,
    css: NETHERLANDS_RESUME_CSS,
};

const INDONESIA_RESUME: CountryResumeSpec = CountryResumeSpec {
    id: BUILTIN_RESUME_INDONESIA_ID,
    name: "Indonesia Resume",
    description: "Indonesia-market CV with contact details, optional photo, language, and work-status fields.",
    country: "Indonesia",
    document_title: "Curriculum Vitae",
    title: "Software Engineer",
    location: "Jakarta, Indonesia",
    summary: "Software engineer with experience delivering practical product features, backend systems, and maintainable technical foundations.",
    personal_details: &[
        ("Work authorization", ""),
        ("Nationality", ""),
        ("Current location", ""),
        ("Availability", ""),
    ],
    skills: &[
        "Backend development",
        "Full-stack development",
        "Mobile or web platforms",
        "APIs",
        "Databases",
    ],
    languages: &["English - Professional", "Bahasa Indonesia - Add level"],
    guidance: "Indonesian CVs often include a fuller personal profile and may include a photo, especially for local employers. For international tech roles, keep the CV achievement-focused and avoid personal details that are not relevant.",
    html: INDONESIA_RESUME_HTML,
    css: INDONESIA_RESUME_CSS,
};

const GERMANY_RESUME_HTML: &str = include_str!("../../../templates/resume-germany.html");
const GERMANY_RESUME_CSS: &str = include_str!("../../../templates/resume-germany.css");
const NETHERLANDS_RESUME_HTML: &str = include_str!("../../../templates/resume-netherlands.html");
const NETHERLANDS_RESUME_CSS: &str = include_str!("../../../templates/resume-netherlands.css");
const INDONESIA_RESUME_HTML: &str = include_str!("../../../templates/resume-indonesia.html");
const INDONESIA_RESUME_CSS: &str = include_str!("../../../templates/resume-indonesia.css");
