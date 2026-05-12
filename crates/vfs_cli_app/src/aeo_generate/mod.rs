// Where: crates/vfs_cli_app/src/aeo_generate/mod.rs
// What: Local dry-run generator for frontend-surface AEO wiki artifacts.
// Why: Repo push automation needs deterministic artifacts before server-side LLM publish exists.

mod collect;
mod output;
mod types;

pub use types::{AeoGenerateArgs, AeoGenerationReport};

use anyhow::{Context, Result, bail};
use collect::{collect_frontend_sources, validate_source_pack};
use output::{build_manifest, build_outputs, slugify, write_outputs};
use std::fs;

pub fn run_aeo_generate(args: AeoGenerateArgs) -> Result<AeoGenerationReport> {
    let repo = args
        .repo
        .canonicalize()
        .with_context(|| format!("repo path does not exist: {}", args.repo.display()))?;
    if !repo.is_dir() {
        bail!("repo path is not a directory: {}", repo.display());
    }
    let project_name = args.project_name.trim();
    if project_name.is_empty() {
        bail!("project name must not be empty");
    }
    let project_slug = slugify(project_name);
    if project_slug.is_empty() {
        bail!("project name must contain at least one alphanumeric character");
    }

    let sources = collect_frontend_sources(&repo)?;
    let validation = validate_source_pack(&sources, &repo);
    let outputs = build_outputs(project_name, &project_slug, &sources);

    fs::create_dir_all(args.out.join("wiki"))?;
    fs::create_dir_all(args.out.join("answers"))?;
    write_outputs(&args.out, project_name, &project_slug, &outputs)?;

    let manifest = build_manifest(project_name, &project_slug, &outputs);
    fs::write(
        args.out.join("manifest.json"),
        serde_json::to_string_pretty(&manifest)? + "\n",
    )?;
    fs::write(
        args.out.join("validation.json"),
        serde_json::to_string_pretty(&validation)? + "\n",
    )?;

    Ok(AeoGenerationReport {
        project_name: project_name.to_string(),
        project_slug,
        framework: "nextjs_app_router".to_string(),
        sources,
        outputs,
        validation,
    })
}

#[cfg(test)]
mod tests {
    use super::{AeoGenerateArgs, run_aeo_generate};
    use crate::aeo_generate::types::SourceKind;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn next_app_router_fixture_generates_expected_artifacts() {
        let dir = tempdir().expect("tempdir should exist");
        let repo = dir.path().join("repo");
        fs::create_dir_all(repo.join("app/features")).expect("dirs should write");
        fs::create_dir_all(repo.join("docs")).expect("docs should write");
        fs::write(repo.join("README.md"), "# Demo\nVisible product.").expect("readme should write");
        fs::write(
            repo.join("app/page.tsx"),
            "export default function Page(){return <main>Demo home</main>}",
        )
        .expect("page should write");
        fs::write(
            repo.join("app/features/page.tsx"),
            "export default function Page(){return <main>Features</main>}",
        )
        .expect("feature page should write");
        fs::write(repo.join("docs/public.md"), "# Public docs").expect("doc should write");

        let out = dir.path().join("out");
        let report = run_aeo_generate(AeoGenerateArgs {
            repo,
            out: out.clone(),
            project_name: "Demo App".to_string(),
        })
        .expect("generation should pass");

        assert!(report.validation.passed);
        assert!(out.join("wiki/overview.md").is_file());
        assert!(out.join("answers/what-is-demo-app.md").is_file());
        assert!(out.join("manifest.json").is_file());
        assert!(
            report
                .sources
                .iter()
                .any(|source| source.path == "app/page.tsx"
                    && source.kind == SourceKind::NextAppPage)
        );
        assert!(
            report
                .sources
                .iter()
                .any(|source| source.path == "docs/public.md"
                    && source.kind == SourceKind::PublicDoc)
        );
    }

    #[test]
    fn generator_excludes_backend_tests_hidden_admin_and_env_files() {
        let dir = tempdir().expect("tempdir should exist");
        let repo = dir.path().join("repo");
        fs::create_dir_all(repo.join("app/admin")).expect("admin dirs should write");
        fs::create_dir_all(repo.join("app")).expect("app dirs should write");
        fs::create_dir_all(repo.join("tests")).expect("tests dirs should write");
        fs::write(
            repo.join("app/page.tsx"),
            "export default function Page(){return null}",
        )
        .expect("page should write");
        fs::write(
            repo.join("app/admin/page.tsx"),
            "export default function Page(){return null}",
        )
        .expect("admin page should write");
        fs::write(repo.join("tests/page.test.tsx"), "test('x',()=>{})").expect("test should write");
        fs::write(repo.join(".env"), "SECRET_KEY=value").expect("env should write");

        let report = run_aeo_generate(AeoGenerateArgs {
            repo,
            out: dir.path().join("out"),
            project_name: "Demo".to_string(),
        })
        .expect("generation should pass");

        let paths = report
            .sources
            .iter()
            .map(|source| source.path.as_str())
            .collect::<Vec<_>>();
        assert!(paths.contains(&"app/page.tsx"));
        assert!(!paths.contains(&"app/admin/page.tsx"));
        assert!(!paths.contains(&"tests/page.test.tsx"));
        assert!(!paths.contains(&".env"));
    }

    #[test]
    fn missing_page_source_fails_validation() {
        let dir = tempdir().expect("tempdir should exist");
        let repo = dir.path().join("repo");
        fs::create_dir_all(&repo).expect("repo should write");
        fs::write(repo.join("README.md"), "# Demo").expect("readme should write");

        let report = run_aeo_generate(AeoGenerateArgs {
            repo,
            out: dir.path().join("out"),
            project_name: "Demo".to_string(),
        })
        .expect("generation should produce validation report");

        assert!(!report.validation.passed);
        assert!(
            report
                .validation
                .errors
                .iter()
                .any(|error| error.contains("missing Next.js App Router page source"))
        );
    }
}
