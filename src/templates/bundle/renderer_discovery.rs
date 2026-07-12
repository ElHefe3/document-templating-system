use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};

use crate::templates::files::is_html_extension;

pub(crate) struct RendererOnlyFiles {
    pub html_path: PathBuf,
    pub css_path: Option<PathBuf>,
}

pub(crate) fn find_renderer_only_files(path: &Path) -> Result<Option<RendererOnlyFiles>> {
    let entries = fs::read_dir(path)
        .with_context(|| format!("failed to read {}", path.display()))?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.is_file())
        .collect::<Vec<_>>();
    let html_files = entries
        .iter()
        .filter(|path| {
            path.extension()
                .and_then(|value| value.to_str())
                .is_some_and(is_html_extension)
        })
        .cloned()
        .collect::<Vec<_>>();
    let html_path = match html_files.as_slice() {
        [only] => only.clone(),
        [] => return Ok(None),
        _ => bail!(
            "template folder has multiple HTML renderer files under {}; keep one .html/.htm file or add template.json (found: {})",
            path.display(),
            display_path_names(&html_files)
        ),
    };

    let css_files = entries
        .iter()
        .filter(|path| {
            path.extension()
                .and_then(|value| value.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("css"))
        })
        .cloned()
        .collect::<Vec<_>>();
    let html_stem = file_stem_string(&html_path);
    let css_path = match css_files.as_slice() {
        [] => None,
        [only] => Some(only.clone()),
        _ => {
            let matching_css = html_stem.as_ref().and_then(|stem| {
                css_files
                    .iter()
                    .find(|path| {
                        path.file_stem()
                            .and_then(|value| value.to_str())
                            .is_some_and(|value| value.eq_ignore_ascii_case(stem))
                    })
                    .cloned()
            });
            match matching_css {
                Some(path) => Some(path),
                None => bail!(
                    "template folder has multiple CSS files under {}; keep one .css file or name it to match the HTML file (found: {})",
                    path.display(),
                    display_path_names(&css_files)
                ),
            }
        }
    };

    Ok(Some(RendererOnlyFiles {
        html_path,
        css_path,
    }))
}

pub(crate) fn file_stem_string(path: &Path) -> Option<String> {
    path.file_stem()
        .and_then(|value| value.to_str())
        .map(str::to_string)
}

fn display_path_names(paths: &[PathBuf]) -> String {
    paths
        .iter()
        .filter_map(|path| path.file_name().and_then(|value| value.to_str()))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::temp_dir;

    #[test]
    fn renderer_only_discovery_prefers_css_matching_html_stem() {
        let root = temp_dir("renderer-css-match");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("document.html"), "<main></main>").unwrap();
        fs::write(root.join("document.css"), "body {}").unwrap();
        fs::write(root.join("extra.css"), "body {}").unwrap();

        let files = find_renderer_only_files(&root).unwrap().unwrap();

        assert_eq!(files.html_path.file_name().unwrap(), "document.html");
        assert_eq!(files.css_path.unwrap().file_name().unwrap(), "document.css");

        fs::remove_dir_all(root).unwrap();
    }
}
