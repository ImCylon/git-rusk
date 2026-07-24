use anyhow::{Context, Result};
use include_dir::{include_dir, Dir};

static TEMPLATES: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/templates");

/// Render the README template, substituting the project name.
pub fn render_readme(project_name: &str) -> Result<String> {
    let file = TEMPLATES
        .get_file("readme.md")
        .context("README template not found in embedded templates")?;
    let contents = file
        .contents_utf8()
        .context("README template is not valid UTF-8")?;
    Ok(contents.replace("{{PROJECT_NAME}}", project_name))
}

/// Render a `.gitignore` template for the given language.
///
/// Returns `Ok(None)` if `language` is `"none"` (skip generation).
/// Returns `Ok(Some(content))` if a template exists for the language.
/// Returns `Err` if the language is unrecognized.
pub fn render_gitignore(language: &str) -> Result<Option<String>> {
    if language == "none" {
        return Ok(None);
    }
    let filename = format!("gitignore/{language}.gitignore");
    let file = TEMPLATES
        .get_file(&filename)
        .with_context(|| format!("No .gitignore template for language '{language}'"))?;
    let contents = file
        .contents_utf8()
        .context("gitignore template is not valid UTF-8")?;
    Ok(Some(contents.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_readme_substitutes_name() {
        let result = render_readme("my-app").unwrap();
        assert!(
            result.contains("# my-app"),
            "expected '# my-app' in output, got: {result}"
        );
    }

    #[test]
    fn test_render_gitignore_rust() {
        let result = render_gitignore("rust").unwrap().unwrap();
        assert!(
            result.contains("/target"),
            "expected '/target' in rust gitignore, got: {result}"
        );
    }

    #[test]
    fn test_render_gitignore_none() {
        let result = render_gitignore("none").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_render_gitignore_invalid() {
        let result = render_gitignore("brainfuck");
        assert!(result.is_err());
    }
}
