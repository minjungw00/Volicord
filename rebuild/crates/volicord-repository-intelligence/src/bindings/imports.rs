//! Explicit imports into verified local files, never repository name fallback.
use super::{BindingFile, Namespace};
use crate::{InventoryEntry, Language};
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

pub(crate) struct ImportIndex {
    paths: BTreeSet<String>,
    java: BTreeMap<String, Vec<(String, String)>>,
}

impl ImportIndex {
    pub(crate) fn new(
        files: &BTreeMap<&str, BindingFile<'_>>,
        inventory: &[InventoryEntry],
        supported_context: bool,
    ) -> Self {
        let mut java = BTreeMap::<String, Vec<(String, String)>>::new();
        for (path, file) in files {
            if !supported_context || *file.language != Language::Java {
                continue;
            }
            let mut cursor = file.tree.root_node().walk();
            let package = file
                .tree
                .root_node()
                .named_children(&mut cursor)
                .find(|node| node.kind() == "package_declaration")
                .and_then(|node| file.text(node).strip_prefix("package"))
                .and_then(|text| text.trim().strip_suffix(';'))
                .map(str::trim);
            let Some(package) = package.filter(|name| qualified(name)) else {
                continue;
            };
            for entity in file.entities.values() {
                let Some(node) = file.entity_node(&entity.identity) else {
                    continue;
                };
                if node.parent() != Some(file.tree.root_node()) || !public(node) {
                    continue;
                }
                if !matches!(
                    node.kind(),
                    "class_declaration" | "interface_declaration" | "enum_declaration"
                ) {
                    continue;
                }
                let Some(name) = node.child_by_field_name("name").map(|n| file.text(n)) else {
                    continue;
                };
                java.entry(format!("{package}.{name}"))
                    .or_default()
                    .push(((*path).into(), entity.identity.clone()));
            }
        }
        Self {
            paths: if supported_context
                && files
                    .values()
                    .any(|file| *file.language == Language::TypeScript)
            {
                inventory.iter().map(|e| e.area.path.clone()).collect()
            } else {
                BTreeSet::new()
            },
            java,
        }
    }

    pub(super) fn resolve(
        &self,
        file: &BindingFile<'_>,
        files: &BTreeMap<&str, BindingFile<'_>>,
        at: Node<'_>,
        display: &str,
        arity: Option<usize>,
        call: bool,
    ) -> Option<String> {
        if file.conditional(at) {
            return None;
        }
        let parts = display.split('.').collect::<Vec<_>>();
        if parts.len() > 2 || parts.iter().any(|p| !identifier(p)) {
            return None;
        }
        let namespace = if call && parts.len() == 1 {
            Namespace::Value
        } else {
            Namespace::Type
        };
        let declarations = file.lexical(at, parts[0], namespace);
        let [declaration] = declarations.as_slice() else {
            return None;
        };
        if declaration.entity.is_some() {
            return None;
        }
        let import = file.declaration_node(declaration)?;
        match file.language {
            Language::Java if (!call && parts.len() == 1) || (call && parts.len() == 2) => {
                if import.kind() != "import_declaration" {
                    return None;
                }
                let name = file
                    .text(import)
                    .strip_prefix("import")?
                    .trim()
                    .strip_suffix(';')?
                    .trim();
                if !qualified(name) || name.rsplit('.').next()? != parts[0] {
                    return None;
                }
                let [(path, id)] = self.java.get(name)?.as_slice() else {
                    return None;
                };
                let destination = files.get(path.as_str())?;
                if !call {
                    return Some(id.clone());
                }
                // A value binding may shadow a type-qualified static receiver.
                let values = file.lexical(at, parts[0], Namespace::Value);
                if values.len() != 1
                    || values[0].start != declaration.start
                    || values[0].end != declaration.end
                {
                    return None;
                }
                let owner = destination.entity_node(id)?;
                let target = destination.member(owner, parts[1], arity, true, false)?;
                public(destination.entity_node(&target)?).then_some(target)
            }
            Language::TypeScript if parts.len() == 1 => {
                if import.kind() != "import_statement" {
                    return None;
                }
                let mut cursor = import.walk();
                let clause = import
                    .named_children(&mut cursor)
                    .find(|n| n.kind() == "import_clause")?;
                let mut cursor = clause.walk();
                let named = clause
                    .named_children(&mut cursor)
                    .find(|n| n.kind() == "named_imports")?;
                let mut cursor = named.walk();
                let specifiers = named
                    .named_children(&mut cursor)
                    .filter(|n| n.kind() == "import_specifier")
                    .filter(|n| {
                        n.child_by_field_name("name")
                            .is_some_and(|n| file.text(n) == parts[0])
                    })
                    .collect::<Vec<_>>();
                let [specifier] = specifiers.as_slice() else {
                    return None;
                };
                // Aliases and namespace/default/re-export resolution stay unsupported.
                if specifier.child_by_field_name("alias").is_some() {
                    return None;
                }
                if call && (has_token(import, "type") || has_token(*specifier, "type")) {
                    return None;
                }
                let source = import.child_by_field_name("source")?;
                let text = file.text(source);
                let module = text
                    .strip_prefix('"')
                    .and_then(|s| s.strip_suffix('"'))
                    .or_else(|| text.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')))?;
                let path = self.module_path(file, module)?;
                let destination = files.get(path.as_str())?;
                let root = destination.tree.root_node();
                let candidates = destination.lexical(
                    root,
                    parts[0],
                    if call {
                        Namespace::Value
                    } else {
                        Namespace::Type
                    },
                );
                let id = destination.select(candidates, arity, call)?;
                let node = destination.entity_node(&id)?;
                let export = node.parent()?;
                (export.kind() == "export_statement"
                    && export.parent() == Some(root)
                    && !has_token(export, "default"))
                .then_some(id)
            }
            _ => None,
        }
    }

    fn module_path(&self, file: &BindingFile<'_>, module: &str) -> Option<String> {
        if !module.starts_with('.') || module.contains('\\') {
            return None;
        }
        let path = &file.entities.values().next()?.area.path;
        let parent = std::path::Path::new(path).parent()?;
        let mut normalized = std::path::PathBuf::new();
        for part in parent.join(module).components() {
            match part {
                std::path::Component::Normal(name) => normalized.push(name),
                std::path::Component::CurDir => {}
                std::path::Component::ParentDir if normalized.pop() => {}
                _ => return None,
            }
        }
        let joined = normalized.to_str()?;
        let stem = joined
            .strip_suffix(".js")
            .or_else(|| joined.strip_suffix(".ts"))
            .unwrap_or(joined);
        if self.paths.contains(&format!("{stem}.js")) || self.paths.contains(&format!("{stem}.tsx"))
        {
            return None;
        }
        let candidate = format!("{stem}.ts");
        self.paths.contains(&candidate).then_some(candidate)
    }
}

fn identifier(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '$')
}
fn qualified(name: &str) -> bool {
    name.split('.').all(identifier)
}
fn has_token(node: Node<'_>, token: &str) -> bool {
    let mut cursor = node.walk();
    let found = node.children(&mut cursor).any(|n| n.kind() == token);
    found
}
fn public(node: Node<'_>) -> bool {
    let mut cursor = node.walk();
    let found = node
        .named_children(&mut cursor)
        .find(|n| n.kind() == "modifiers")
        .is_some_and(|modifiers| has_token(modifiers, "public"));
    found
}
