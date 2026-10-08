//! Authored large diagnostic basis shared by HTML and real-browser consumers.
use crate::reading_fixture::Fixture;
use volicord_context::*;

pub fn add_code_diagnostics(f: &Fixture) -> Result<Vec<SourceId>, Box<dyn std::error::Error>> {
    let directory = f.repository.join("outside_work");
    std::fs::create_dir_all(&directory)?;
    for (name, body) in [
        ("example.rs", "pub fn read(v: i32) -> i32 { v + 1 }"),
        (
            "example.ts",
            "export function read(v: number): number { return v + 1; }",
        ),
        ("example.js", "export function read(v) { return v + 1; }"),
        (
            "example.go",
            "package example\nfunc Read(v int) int { return v + 1 }",
        ),
        (
            "Example.java",
            "class Example { int read(int v) { return v + 1; } }",
        ),
        ("example.c", "int read(int v) { return v + 1; }"),
        ("example.py", "def read(v):\n    return v + 1\n"),
        ("broken.cpp", "void broken( {\n"),
    ] {
        std::fs::write(directory.join(name), body)?;
    }
    let canonical = f.operations.canonical_basis(f.project)?;
    let mut store = Store::open(f.operations.layout().canonical_store())?;
    let mut sources = Vec::new();
    for n in 0..30_u128 {
        sources.push(store.record_source(OperationId::from_bytes((300_000 + n).to_le_bytes()), f.project, SourceDraft {
            expected_project_revision: canonical.project.revision,
            payload: SourcePayload::CurrentHostUserTurn { host: "fixture".into(), session: "large-diagnostics".into(), turn: format!("Authored unavailable Source {n}; 긴 한국어 진단 및 English diagnostic evidence") },
            actor: Principal { kind: PrincipalKind::User, identity: "fixture".into() }, observer: None, availability: Availability::Unavailable,
        })?.value.id);
    }
    drop(store);
    Ok(sources)
}
