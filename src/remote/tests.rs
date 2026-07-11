use serde_json::{json, Value};

use crate::{
    model::{TemplateManifest, TemplateRender, BUILTIN_CLASSIC_ID},
    remote::templates::RemoteTemplateStore,
    storage::memory::MemoryStorageProvider,
    storage::StorageProvider,
    templates::bundle::{TemplateBundle, TemplateBundleLayout},
    templates::files::TemplateBundleFile,
};

#[test]
fn remote_store_uploads_lists_loads_and_deletes_folder_bundle() {
    let provider = MemoryStorageProvider::default();
    let store = RemoteTemplateStore::new(Box::new(provider.clone()), "templates");
    let bundle = folder_bundle("test-template");

    let written = store.upload(&bundle).unwrap();

    assert_eq!(written.id, "test-template");
    assert_eq!(
        written.key,
        "templates/test-template/document-template.json"
    );
    assert_eq!(
        provider
            .download("templates/test-template/template.html")
            .unwrap(),
        b"<main>{{profile.full_name}}</main>"
    );

    let summaries = store.list_summaries().unwrap();
    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].id, "test-template");
    assert!(store.exists("test-template").unwrap());

    let loaded = store.load("test-template").unwrap().unwrap();
    assert_eq!(loaded.render.html, "<main>{{profile.full_name}}</main>");
    assert_eq!(loaded.render.css, "body { color: black; }");

    store.delete("test-template").unwrap();
    assert!(!store.exists("test-template").unwrap());
}

#[test]
fn remote_store_prefers_folder_manifest_over_single_file_json() {
    let provider = MemoryStorageProvider::default();
    provider
        .upload(
            "templates/dupe.json",
            &manifest_body("dupe", "Single File"),
            "application/json; charset=utf-8",
        )
        .unwrap();
    provider
        .upload(
            "templates/dupe/document-template.json",
            &manifest_body("dupe", "Canonical"),
            "application/json; charset=utf-8",
        )
        .unwrap();

    let store = RemoteTemplateStore::new(Box::new(provider), "templates/");
    let summaries = store.list_summaries().unwrap();

    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].id, "dupe");
    assert_eq!(summaries[0].name, "Canonical");
}

#[test]
fn remote_store_rejects_manifest_file_escape_paths() {
    let provider = MemoryStorageProvider::default();
    let mut manifest = template_manifest("escape");
    manifest.render.html.clear();
    manifest.render.html_file = Some("../template.html".to_string());
    provider
        .upload(
            "templates/escape/document-template.json",
            &serde_json::to_vec_pretty(&manifest).unwrap(),
            "application/json; charset=utf-8",
        )
        .unwrap();

    let store = RemoteTemplateStore::new(Box::new(provider), "templates/");
    let err = store.load("escape").unwrap_err();

    assert!(err
        .to_string()
        .contains("remote template file path must stay inside the template folder"));
}

fn folder_bundle(id: &str) -> TemplateBundle {
    let mut template = template_manifest(id);
    template.render.html.clear();
    template.render.css.clear();
    template.render.html_file = Some("template.html".to_string());
    template.render.css_file = Some("style.css".to_string());
    TemplateBundle {
        manifest_body: serde_json::to_vec_pretty(&template).unwrap(),
        template,
        layout: TemplateBundleLayout::Folder,
        files: vec![
            TemplateBundleFile {
                name: "template.html".to_string(),
                body: b"<main>{{profile.full_name}}</main>".to_vec(),
                content_type: "text/html; charset=utf-8".to_string(),
            },
            TemplateBundleFile {
                name: "style.css".to_string(),
                body: b"body { color: black; }".to_vec(),
                content_type: "text/css; charset=utf-8".to_string(),
            },
        ],
    }
}

fn template_manifest(id: &str) -> TemplateManifest {
    TemplateManifest {
        schema_version: 1,
        id: id.to_string(),
        name: display_name(id),
        version: Some("0.1.0".to_string()),
        description: String::new(),
        sections: Vec::new(),
        defaults: Value::Object(serde_json::Map::new()),
        render: TemplateRender {
            html: "<main></main>".to_string(),
            css: String::new(),
            html_file: None,
            css_file: None,
            pdf_filename: format!("{id}.pdf"),
        },
    }
}

fn manifest_body(id: &str, name: &str) -> Vec<u8> {
    serde_json::to_vec_pretty(&json!({
        "schema_version": 1,
        "id": id,
        "name": name,
        "render": {
            "html": "<main></main>",
            "pdf_filename": format!("{id}.pdf")
        },
        "sections": [],
        "defaults": {}
    }))
    .unwrap()
}

fn display_name(id: &str) -> String {
    if id == BUILTIN_CLASSIC_ID {
        "Classic Resume".to_string()
    } else {
        id.split(['-', '_'])
            .filter(|part| !part.is_empty())
            .map(|part| {
                let mut chars = part.chars();
                match chars.next() {
                    Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
                    None => String::new(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}
