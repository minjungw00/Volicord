//! Independent language-binding expectations; no unique-name fallback oracle.
use std::{error::Error, fs};
use tempfile::tempdir;
use volicord_repository_intelligence::*;
mod support;

fn analyze(files: &[(&str, &str)]) -> Result<AnalysisSnapshot, Box<dyn Error>> {
    let root = tempdir()?;
    for (path, body) in files {
        let path = root.path().join(path);
        fs::create_dir_all(path.parent().ok_or("parent")?)?;
        fs::write(path, body)?;
    }
    let grounding = support::repository_grounding(0x91, 0x92)?;
    let request = InventoryRequest::new(
        root.path(),
        &grounding.grounding,
        grounding.source_id,
        1_725_000_000_000_000,
    )?;
    Ok(analyze_repository_semantics(SemanticAnalysisRequest::new(
        StructuralAnalysisRequest::new(request),
    ))?
    .1)
}

fn references(analysis: &AnalysisSnapshot, main: &str, target: Option<&str>) {
    let calls = analysis
        .semantic_results
        .iter()
        .filter(|r| {
            r.relation.kind == SemanticRelationKind::References
                && analysis.structural_facts.iter().any(|f| {
                    f.entity.identity == r.relation.source_entity
                        && f.entity.area.path == main
                        && f.entity.display_name.as_deref() == Some("run")
                })
        })
        .collect::<Vec<_>>();
    assert_eq!(calls.len(), 1, "one native call expected in {main}");
    let call = calls[0];
    assert!(call.relation.supporting_range.is_some());
    let grounding = support::repository_grounding(0x91, 0x92).expect("fixture grounding");
    let basis =
        grounded_explanation_basis(analysis, analysis.repository_snapshot, &grounding.grounding)
            .expect("public grounding");
    let hits = search_local(
        analysis,
        "references",
        analysis.repository_snapshot,
        100,
        &grounding.grounding,
    )
    .expect("public search");
    let hits = hits
        .iter()
        .filter(|hit| {
            hit.result_kind == SearchResultKind::SemanticRelation
                && hit.source_range == call.relation.supporting_range
                && hit.relation_target.as_ref() == Some(&call.relation.target)
        })
        .collect::<Vec<_>>();
    assert_eq!(
        hits.len(),
        1,
        "exact imported call must survive public search"
    );
    let hit = hits[0];
    let evidence = basis
        .evidence
        .iter()
        .find(|e| {
            e.source_range == hit.source_range
                && e.statement_class == GroundingStatementClass::SemanticResult
                && e.relation_target == hit.relation_target
        })
        .expect("exact grounding evidence");
    assert_eq!(evidence.uncertainty, hit.uncertainty);
    assert_eq!(evidence.semantic_provenance, hit.semantic_provenance);
    assert_eq!(evidence.source, hit.source);
    match (&call.relation.target, target) {
        (RelationTarget::ResolvedEntity(id), Some(path)) => {
            let entity = analysis
                .structural_facts
                .iter()
                .find(|f| &f.entity.identity == id)
                .expect("target entity");
            assert_eq!(entity.entity.area.path, path);
            assert_eq!(entity.entity.display_name.as_deref(), Some("normalize"));
            assert_eq!(entity.entity.analysis_snapshot, analysis.identity);
            assert!(call.relation.uncertainty.reasons.is_empty());
        }
        (RelationTarget::Unresolved(_), None) => {
            assert!(!call.relation.uncertainty.reasons.is_empty());
            assert!(!call.relation.diagnostics.is_empty());
        }
        (actual, expected) => panic!("{main}: expected {expected:?}, actual {actual:?}"),
    }
}

#[test]
fn java_explicit_package_and_static_owner_bindings_reject_shadowing_and_inaccessible_targets(
) -> Result<(), Box<dyn Error>> {
    let public = "package lib; public class Names { public static String normalize(String x) { return x; } }";
    let main = "package app; import lib.Names; class Main { static String run() { return Names.normalize(\"ok\"); } }";
    references(
        &analyze(&[("app/Main.java", main), ("lib/Names.java", public)])?,
        "app/Main.java",
        Some("lib/Names.java"),
    );
    for (label, main, other) in [
        ("unimported", "package app; class Main { static String run() { return Names.normalize(\"ok\"); } }",public),
        ("wrong-package",main,"package different; public class Names { public static String normalize(String x) { return x; } }"),
        ("shadowed-receiver","package app; import lib.Names; class Main { static String run(Object Names) { return Names.normalize(\"ok\"); } }",public),
        ("wildcard","package app; import lib.*; class Main { static String run() { return Names.normalize(\"ok\"); } }",public),
        ("private-method",main,"package lib; public class Names { private static String normalize(String x) { return x; } }"),
        ("instance-method",main,"package lib; public class Names { public String normalize(String x) { return x; } }"),
        ("nonpublic-owner",main,"package lib; class Names { public static String normalize(String x) { return x; } }"),
        ("annotation-is-not-access",main,"package lib; public class Names { @Label(\"public\") private static String normalize(String x) { return x; } }"),
    ] {
        println!("Java negative: {label}");
        references(&analyze(&[("app/Main.java",main),("lib/Names.java",other)])?,"app/Main.java",None);
    }
    references(
        &analyze(&[
            ("app/Main.java", main),
            ("a/Names.java", public),
            ("b/Names.java", public),
        ])?,
        "app/Main.java",
        None,
    );
    references(
        &analyze(&[
            ("app/Main.java", main),
            ("lib/Names.java", public),
            ("lib/module-info.java", "module hidden { } "),
        ])?,
        "app/Main.java",
        None,
    );
    Ok(())
}

#[test]
fn typescript_named_relative_exports_require_value_authority_and_unique_module(
) -> Result<(), Box<dyn Error>> {
    let exported = "export function normalize(value: string) { return value.trim(); }";
    let main =
        "import { normalize } from './names.js'; export function run() { return normalize('ok'); }";
    references(
        &analyze(&[("main.ts", main), ("names.ts", exported)])?,
        "main.ts",
        Some("names.ts"),
    );
    for (label, main, other) in [
        ("unimported","export function run() { return normalize('ok'); }",exported),
        ("wrong-module","import { normalize } from './missing.js'; export function run() { return normalize('ok'); }",exported),
        ("private-declaration",main,"function normalize(value: string) { return value.trim(); }"),
        ("type-only-import","import type { normalize } from './names.js'; export function run() { return normalize('ok'); }",exported),
        ("type-only-specifier","import { type normalize } from './names.js'; export function run() { return normalize('ok'); }",exported),
        ("shadowed-value","import { normalize } from './names.js'; export function run(normalize: (x: string) => string) { return normalize('ok'); }",exported),
        ("unsupported-alias","import { normalize as local } from './names.js'; export function run() { return local('ok'); }",exported),
        ("default-is-not-named",main,"export default function normalize(value: string) { return value.trim(); }"),
        ("re-export-is-not-declaration",main,"function normalize(value: string) { return value.trim(); } export { normalize };"),
    ] {
        println!("TypeScript negative: {label}");
        references(&analyze(&[("main.ts",main),("names.ts",other)])?,"main.ts",None);
    }
    for competing in ["names.js", "names.tsx"] {
        references(
            &analyze(&[
                ("main.ts", main),
                ("names.ts", exported),
                (competing, "export const alternative = 1;"),
            ])?,
            "main.ts",
            None,
        );
    }
    for configuration in [
        r#"{"extends":"./unobserved.json"}"#,
        r#"{"compilerOptions":{"moduleSuffixes":[".native", ""]}}"#,
        r#"{"compilerOptions":{"rootDirs":["src", "generated"]}}"#,
        r#"{"compilerOptions":{"moduleResolution":"unrecognized"}}"#,
        "{ unsupported JSONC /* inherited options unknown */ }",
    ] {
        let analysis = analyze(&[
            ("main.ts", main),
            ("names.ts", exported),
            ("tsconfig.json", configuration),
        ])?;
        references(&analysis, "main.ts", None);
        let imports = analysis
            .semantic_results
            .iter()
            .filter(|r| r.relation.kind == SemanticRelationKind::ResolvesTo)
            .collect::<Vec<_>>();
        assert!(!imports.is_empty());
        assert!(
            imports.iter().all(
                |r| matches!(r.relation.target, RelationTarget::Unresolved(_))
                    && !r.relation.diagnostics.is_empty()
            ),
            "unsupported configuration cannot certify a module edge"
        );
    }
    Ok(())
}
