use anyhow::{Context, Result};

use crate::storage::StorageObject;

pub(crate) fn parse_list_bucket_result(xml: &str) -> Result<Vec<StorageObject>> {
    let doc =
        roxmltree::Document::parse(xml).context("failed to parse storage list response XML")?;
    let mut objects = Vec::new();
    for contents in doc
        .descendants()
        .filter(|node| node.has_tag_name("Contents"))
    {
        let Some(key) = child_text(contents, "Key") else {
            continue;
        };
        objects.push(StorageObject {
            key: key.to_string(),
            size: child_text(contents, "Size").and_then(|text| text.parse().ok()),
            last_modified: child_text(contents, "LastModified").map(str::to_string),
            content_type: None,
        });
    }
    Ok(objects)
}

fn child_text<'a>(node: roxmltree::Node<'a, 'a>, tag: &str) -> Option<&'a str> {
    node.children()
        .find(|child| child.has_tag_name(tag))
        .and_then(|child| child.text())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_list_bucket_response_objects() {
        let objects = parse_list_bucket_result(
            r#"<ListBucketResult>
              <Contents><Key>templates/a.json</Key><Size>12</Size><LastModified>today</LastModified></Contents>
            </ListBucketResult>"#,
        )
        .unwrap();
        assert_eq!(objects[0].key, "templates/a.json");
        assert_eq!(objects[0].size, Some(12));
        assert_eq!(objects[0].last_modified.as_deref(), Some("today"));
    }

    #[test]
    fn skips_contents_without_key() {
        let objects = parse_list_bucket_result(
            r#"<ListBucketResult>
              <Contents><Size>12</Size></Contents>
            </ListBucketResult>"#,
        )
        .unwrap();
        assert!(objects.is_empty());
    }
}
