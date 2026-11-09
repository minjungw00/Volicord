//! Bounded source bindings. Spelling and repository-wide uniqueness are never
//! binding evidence. Unsupported imports, receivers and patterns stop lookup.
use crate::{
    CodeEntity, CodeEntityKind, Language, RelationTarget, SourceRange, StructuralFact,
    UnresolvedTarget,
};
use std::collections::BTreeMap;
use tree_sitter::{Node, Parser, Tree};

#[derive(Clone, Copy, PartialEq)]
enum Namespace {
    Method,
    Value,
    Type,
}

struct Declaration {
    start: usize,
    end: usize,
    value: bool,
    ty: bool,
    entity: Option<String>,
}

#[derive(Default)]
struct Scope {
    names: BTreeMap<String, Vec<Declaration>>,
    wildcard: bool,
    opaque_bindings: bool,
}

pub(crate) struct BindingFile<'a> {
    source: &'a str,
    language: &'a Language,
    tree: Tree,
    scopes: BTreeMap<usize, Scope>,
    entities: BTreeMap<(usize, usize), &'a CodeEntity>,
    entity_ranges: BTreeMap<&'a str, (usize, usize)>,
}

impl<'a> BindingFile<'a> {
    pub(crate) fn new(
        source: &'a str,
        language: &'a Language,
        facts: &[&'a StructuralFact],
    ) -> Option<Self> {
        let grammar = match language {
            Language::Java => tree_sitter_java::LANGUAGE.into(),
            Language::TypeScript
                if facts
                    .first()
                    .is_some_and(|f| f.entity.area.path.ends_with(".tsx")) =>
            {
                tree_sitter_typescript::LANGUAGE_TSX.into()
            }
            Language::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Language::Rust => tree_sitter_rust::LANGUAGE.into(),
            _ => return None,
        };
        let mut parser = Parser::new();
        parser.set_language(&grammar).ok()?;
        let tree = parser.parse(source, None)?;
        if tree.root_node().has_error() {
            return None;
        }
        let entities: BTreeMap<_, _> = facts
            .iter()
            .filter_map(|f| {
                let range = f.entity.source_range.as_ref()?;
                let node =
                    entity_syntax(tree.root_node(), range, &f.provenance.supported_construct)?;
                Some(((node.start_byte(), node.end_byte()), &f.entity))
            })
            .collect();
        let entity_ranges = entities
            .iter()
            .map(|(range, e)| (e.identity.as_str(), *range))
            .collect();
        let mut file = Self {
            source,
            language,
            tree,
            scopes: BTreeMap::new(),
            entities,
            entity_ranges,
        };
        // Build once; lookup never rescans a whole file for every call.
        let mut scopes = BTreeMap::new();
        file.index(file.tree.root_node(), None, &mut scopes);
        file.scopes = scopes;
        Some(file)
    }

    fn text(&self, node: Node<'_>) -> &str {
        &self.source[node.byte_range()]
    }

    fn index(&self, node: Node<'_>, parent: Option<usize>, scopes: &mut BTreeMap<usize, Scope>) {
        if let Some(scope) = parent {
            let entity = self
                .entities
                .get(&(node.start_byte(), node.end_byte()))
                .copied();
            let named = node.child_by_field_name("name");
            let kind = node.kind();
            let declaration = entity
                .filter(|e| !matches!(e.kind, CodeEntityKind::File | CodeEntityKind::Package));
            if let (Some(entity), Some(name)) = (declaration, named) {
                let (value, ty) = match entity.kind {
                    CodeEntityKind::Interface | CodeEntityKind::Trait | CodeEntityKind::Type => {
                        (false, true)
                    }
                    CodeEntityKind::Class
                    | CodeEntityKind::Struct
                    | CodeEntityKind::Enum
                    | CodeEntityKind::Module => (true, true),
                    _ => (true, false),
                };
                self.insert(
                    scopes,
                    scope,
                    name,
                    value,
                    ty,
                    Some(entity.identity.clone()),
                    node,
                );
            } else if matches!(
                kind,
                "function_declaration"
                    | "function_item"
                    | "mod_item"
                    | "class_declaration"
                    | "internal_module"
            ) {
                if let Some(name) = named {
                    self.insert(scopes, scope, name, true, true, None, node);
                }
            }
            match kind {
                "variable_declarator" | "let_declaration" | "const_item" | "static_item" => {
                    if let Some(pattern) = node
                        .child_by_field_name("name")
                        .or_else(|| node.child_by_field_name("pattern"))
                    {
                        let mut binding_scope = scope;
                        if *self.language == Language::TypeScript
                            && node
                                .parent()
                                .is_some_and(|p| p.kind() == "variable_declaration")
                        {
                            let mut owner = node.parent();
                            while let Some(p) = owner {
                                if is_function(p) || p.parent().is_none() {
                                    binding_scope = p.id();
                                    break;
                                }
                                owner = p.parent();
                            }
                        }
                        self.block_pattern(scopes, binding_scope, pattern, false, node);
                    }
                }
                "required_parameter"
                | "optional_parameter"
                | "formal_parameter"
                | "spread_parameter"
                | "parameter"
                | "self_parameter"
                | "type_parameter"
                | "catch_formal_parameter" => {
                    let pattern = node
                        .child_by_field_name("pattern")
                        .or_else(|| node.child_by_field_name("name"))
                        .unwrap_or(node);
                    self.block_pattern(scopes, scope, pattern, kind == "type_parameter", node);
                }
                "import_statement" | "use_declaration" | "import_declaration" => {
                    // A module path alone proves no imported symbol/alias binding.
                    // Keep explicit import edges separately; symbol binding is unsupported.
                    self.block_pattern(scopes, scope, node, true, node);
                    if self.text(node).contains('*') {
                        scopes.entry(scope).or_default().wildcard = true;
                    }
                }
                "macro_invocation"
                    if *self.language == Language::Rust
                        && node
                            .parent()
                            .is_some_and(|p| p.kind() == "expression_statement") =>
                {
                    scopes.entry(scope).or_default().opaque_bindings = true;
                }
                "instanceof_expression" | "record_pattern" => {
                    if let Some(name) = node.child_by_field_name("name") {
                        self.block_pattern(scopes, scope, name, false, node);
                    }
                }
                "catch_clause" => {
                    if let Some(pattern) = node.child_by_field_name("parameter") {
                        self.block_pattern(scopes, node.id(), pattern, false, node);
                    }
                }
                "for_in_statement"
                | "for_expression"
                | "match_arm"
                | "let_condition"
                | "enhanced_for_statement" => {
                    if let Some(pattern) = node
                        .child_by_field_name("left")
                        .or_else(|| node.child_by_field_name("pattern"))
                        .or_else(|| node.child_by_field_name("name"))
                    {
                        let binding_scope = if kind == "let_condition" {
                            scope
                        } else {
                            node.id()
                        };
                        self.block_pattern(scopes, binding_scope, pattern, false, node);
                    }
                }
                _ => {}
            }
        }
        if is_function(node) || matches!(node.kind(), "closure_expression" | "lambda_expression") {
            if let Some(parameters) = node
                .child_by_field_name("parameters")
                .or_else(|| node.child_by_field_name("parameter"))
            {
                self.block_pattern(scopes, node.id(), parameters, false, node);
            }
        }
        let scope = if is_scope(node) {
            scopes.entry(node.id()).or_default();
            Some(node.id())
        } else {
            parent
        };
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            self.index(child, scope, scopes);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn insert(
        &self,
        scopes: &mut BTreeMap<usize, Scope>,
        scope: usize,
        name: Node<'_>,
        value: bool,
        ty: bool,
        entity: Option<String>,
        node: Node<'_>,
    ) {
        scopes
            .entry(scope)
            .or_default()
            .names
            .entry(self.text(name).to_owned())
            .or_default()
            .push(Declaration {
                start: node.start_byte(),
                end: node.end_byte(),
                value,
                ty,
                entity,
            });
    }

    fn block_pattern(
        &self,
        scopes: &mut BTreeMap<usize, Scope>,
        scope: usize,
        pattern: Node<'_>,
        ty: bool,
        declaration: Node<'_>,
    ) {
        if matches!(
            pattern.kind(),
            "identifier" | "type_identifier" | "shorthand_property_identifier_pattern" | "self"
        ) {
            self.insert(scopes, scope, pattern, true, ty, None, declaration);
            return;
        }
        let mut cursor = pattern.walk();
        for child in pattern.named_children(&mut cursor) {
            self.block_pattern(scopes, scope, child, ty, declaration);
        }
    }

    fn lexical<'s>(
        &'s self,
        at: Node<'s>,
        name: &str,
        namespace: Namespace,
    ) -> Vec<&'s Declaration> {
        let mut ancestor = Some(at);
        while let Some(node) = ancestor {
            if let Some(scope) = self.scopes.get(&node.id()) {
                if scope.opaque_bindings {
                    return Vec::new();
                }
                let skip_members = namespace == Namespace::Value
                    && match self.language {
                        Language::TypeScript => {
                            matches!(node.kind(), "class_body" | "interface_body")
                        }
                        Language::Rust => {
                            node.kind() == "declaration_list"
                                && node
                                    .parent()
                                    .is_some_and(|p| matches!(p.kind(), "impl_item" | "trait_item"))
                        }
                        _ => false,
                    };
                if !skip_members {
                    let declarations = scope
                        .names
                        .get(name)
                        .into_iter()
                        .flatten()
                        .filter(|d| match namespace {
                            Namespace::Value => d.value,
                            Namespace::Type => d.ty,
                            Namespace::Method => {
                                self.entities.get(&(d.start, d.end)).is_some_and(|e| {
                                    matches!(e.kind, CodeEntityKind::Method | CodeEntityKind::Test)
                                })
                            }
                        })
                        .collect::<Vec<_>>();
                    if !declarations.is_empty() {
                        return declarations;
                    }
                    if scope.wildcard
                        || (*self.language == Language::Java
                            && namespace == Namespace::Method
                            && node.kind() == "class_body")
                    {
                        return Vec::new();
                    }
                }
            }
            ancestor = node.parent();
        }
        Vec::new()
    }

    fn declaration_node(&self, declaration: &Declaration) -> Option<Node<'_>> {
        self.tree
            .root_node()
            .descendant_for_byte_range(declaration.start, declaration.end)
            .filter(|n| n.start_byte() == declaration.start && n.end_byte() == declaration.end)
    }

    fn select(
        &self,
        declarations: Vec<&Declaration>,
        arity: Option<usize>,
        callable: bool,
    ) -> Option<String> {
        if declarations.iter().any(|d| {
            d.entity.is_none()
                || self
                    .declaration_node(d)
                    .is_some_and(|node| self.conditional(node))
        }) {
            return None;
        }
        let mut matches = declarations.into_iter().filter(|d| {
            let Some(entity) = self.entities.get(&(d.start, d.end)) else {
                return false;
            };
            if callable
                && !matches!(
                    entity.kind,
                    CodeEntityKind::Function | CodeEntityKind::Method | CodeEntityKind::Test
                )
            {
                return false;
            }
            if let Some(arity) = arity {
                let Some(node) = self.declaration_node(d) else {
                    return false;
                };
                if self.arity(node) != Some(arity) {
                    return false;
                }
            }
            true
        });
        let first = matches.next()?;
        matches
            .next()
            .is_none()
            .then(|| first.entity.clone())
            .flatten()
    }

    fn arity(&self, node: Node<'_>) -> Option<usize> {
        let params = node.child_by_field_name("parameters")?;
        let mut cursor = params.walk();
        let parameters = params
            .named_children(&mut cursor)
            .filter(|n| n.kind() != "comment")
            .collect::<Vec<_>>();
        if parameters.iter().any(|p| {
            matches!(
                p.kind(),
                "spread_parameter" | "variadic_parameter" | "optional_parameter"
            ) || self.text(*p).contains("...")
        }) {
            return None;
        }
        Some(
            parameters
                .iter()
                .filter(|p| p.kind() != "self_parameter")
                .count(),
        )
    }

    pub(crate) fn target(
        &self,
        source: &CodeEntity,
        target: &RelationTarget,
        range: Option<&SourceRange>,
        call: bool,
    ) -> RelationTarget {
        let RelationTarget::Unresolved(unresolved) = target else {
            return self.unresolved(
                source,
                "<structural target>",
                "no supported language binding was provided",
            );
        };
        let display = unresolved.display.trim();
        let at = range.and_then(|r| exact_node(self.tree.root_node(), r));
        let resolved = at.and_then(|at| {
            if !call && !self.declared_parent(at, display) {
                return None;
            }
            let arity = if call {
                let arguments = at.child_by_field_name("arguments")?;
                let mut cursor = arguments.walk();
                let args = arguments
                    .named_children(&mut cursor)
                    .filter(|n| n.kind() != "comment")
                    .collect::<Vec<_>>();
                if args
                    .iter()
                    .any(|a| matches!(a.kind(), "spread_element" | "ERROR"))
                {
                    return None;
                }
                Some(args.len())
            } else {
                None
            };
            self.resolve(
                at,
                display,
                if call {
                    Namespace::Value
                } else {
                    Namespace::Type
                },
                arity,
                call,
            )
        });
        resolved.map(RelationTarget::ResolvedEntity).unwrap_or_else(|| self.unresolved(source, display, "binding is absent, shadowed, ambiguous, external, or unsupported; receiver/import/lexical scope cannot justify a unique target"))
    }

    fn declared_parent(&self, node: Node<'_>, display: &str) -> bool {
        if *self.language == Language::Rust {
            return node.kind() == "impl_item"
                && node
                    .child_by_field_name("trait")
                    .is_some_and(|ty| self.text(ty) == display);
        }
        fn contains_parent(file: &BindingFile<'_>, node: Node<'_>, display: &str) -> bool {
            if matches!(
                node.kind(),
                "identifier"
                    | "type_identifier"
                    | "scoped_type_identifier"
                    | "nested_type_identifier"
            ) {
                return file.text(node) == display;
            }
            if !matches!(
                node.kind(),
                "class_heritage"
                    | "extends_clause"
                    | "implements_clause"
                    | "extends_type_clause"
                    | "superclass"
                    | "super_interfaces"
                    | "extends_interfaces"
                    | "type_list"
            ) {
                return false;
            }
            let mut cursor = node.walk();
            let found = node
                .named_children(&mut cursor)
                .any(|child| contains_parent(file, child, display));
            found
        }
        let mut cursor = node.walk();
        let found = node
            .named_children(&mut cursor)
            .filter(|child| {
                matches!(
                    child.kind(),
                    "class_heritage"
                        | "extends_clause"
                        | "implements_clause"
                        | "extends_type_clause"
                        | "superclass"
                        | "super_interfaces"
                        | "extends_interfaces"
                )
            })
            .any(|child| contains_parent(self, child, display));
        found
    }

    fn unresolved(&self, source: &CodeEntity, display: &str, reason: &str) -> RelationTarget {
        RelationTarget::Unresolved(UnresolvedTarget {
            display: display.to_owned(),
            language: Some(source.language.clone()),
            locator_hint: Some(source.area.path.clone()),
            reason: reason.to_owned(),
        })
    }

    fn resolve(
        &self,
        at: Node<'_>,
        display: &str,
        namespace: Namespace,
        arity: Option<usize>,
        call: bool,
    ) -> Option<String> {
        if self.conditional(at) {
            return None;
        }
        if display
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '$')
        {
            let namespace = if *self.language == Language::Java && call {
                Namespace::Method
            } else {
                namespace
            };
            let id = self.select(self.lexical(at, display, namespace), arity, call)?;
            if *self.language == Language::Java
                && call
                && self.enclosing_method(at).is_some_and(|m| self.is_static(m))
                && self.entity_node(&id).is_some_and(|m| !self.is_static(m))
            {
                return None;
            }
            return Some(id);
        }
        if *self.language == Language::Rust && display.starts_with("self.") {
            let method = self.rust_method(at)?;
            let parameters = method.child_by_field_name("parameters")?;
            let mut cursor = parameters.walk();
            if !parameters
                .named_children(&mut cursor)
                .any(|p| p.kind() == "self_parameter")
            {
                return None;
            }
            let owner = self.owner(at)?;
            let ty = self.text(owner.child_by_field_name("type")?);
            let id = self.select(self.lexical(owner, ty, Namespace::Type), None, false)?;
            return self.rust_associated(
                self.entity_node(&id)?,
                display.strip_prefix("self.")?,
                arity,
                call,
                true,
            );
        }
        let separator = if *self.language == Language::Rust {
            "::"
        } else {
            "."
        };
        let parts = display.split(separator).collect::<Vec<_>>();
        if parts.len() != 2 || parts.iter().any(|p| p.is_empty()) {
            return None;
        }
        let (receiver, member) = (parts[0], parts[1]);
        if matches!(
            (self.language, receiver),
            (Language::TypeScript | Language::Java, "this") | (Language::Rust, "Self")
        ) {
            let owner = self.owner(at)?;
            if *self.language == Language::Rust {
                self.rust_method(at)?;
                let ty = self.text(owner.child_by_field_name("type")?);
                let id = self.select(self.lexical(owner, ty, Namespace::Type), None, false)?;
                return self.rust_associated(self.entity_node(&id)?, member, arity, call, false);
            }
            let method = self.enclosing_method(at);
            if *self.language == Language::TypeScript && method.is_none() {
                return None;
            }
            if let Some(method) = method {
                if *self.language == Language::Java && self.is_static(method) {
                    return None;
                }
                if let Some(parameters) = method.child_by_field_name("parameters") {
                    let mut cursor = parameters.walk();
                    if parameters.named_children(&mut cursor).any(|p| {
                        self.text(p)
                            .trim_start()
                            .strip_prefix("this")
                            .is_some_and(|rest| rest.trim_start().starts_with(':'))
                    }) {
                        return None;
                    }
                }
            }
            let instance = method.is_none_or(|method| !self.is_static(method));
            return self.member(owner, member, arity, call, instance);
        }
        // Rust method calls use '.', while namespace/associated paths use '::'.
        if *self.language == Language::Rust {
            return self.rust_path(at, receiver, member, arity, call);
        }
        let owner_id = self.select(self.lexical(at, receiver, Namespace::Type), None, false)?;
        // TS class receivers occupy the value namespace too: a parameter/local
        // with the same spelling blocks a class/static binding.
        if self
            .select(self.lexical(at, receiver, Namespace::Value), None, false)
            .as_ref()
            != Some(&owner_id)
        {
            return None;
        }
        let owner = self.entity_node(&owner_id)?;
        self.member(owner, member, arity, call, false)
    }

    fn entity_node(&self, id: &str) -> Option<Node<'_>> {
        let (start, end) = self.entity_ranges.get(id)?;
        self.tree
            .root_node()
            .descendant_for_byte_range(*start, *end)
    }

    fn rust_method<'t>(&self, at: Node<'t>) -> Option<Node<'t>> {
        let mut ancestor = at.parent();
        while let Some(node) = ancestor {
            if node.kind() == "function_item" {
                return node
                    .parent()
                    .and_then(|p| p.parent())
                    .filter(|p| p.kind() == "impl_item")
                    .map(|_| node);
            }
            ancestor = node.parent();
        }
        None
    }

    fn conditional(&self, at: Node<'_>) -> bool {
        if *self.language != Language::Rust {
            return false;
        }
        let mut ancestor = Some(at);
        while let Some(node) = ancestor {
            let mut previous = node.prev_named_sibling();
            while let Some(attribute) = previous.filter(|p| p.kind() == "attribute_item") {
                if self.text(attribute).contains("cfg") {
                    return true;
                }
                previous = attribute.prev_named_sibling();
            }
            ancestor = node.parent();
        }
        false
    }

    fn is_static(&self, node: Node<'_>) -> bool {
        node.child_by_field_name("name").is_some_and(|name| {
            self.source[node.start_byte()..name.start_byte()]
                .split_whitespace()
                .any(|w| w == "static")
        })
    }
    fn enclosing_method<'t>(&self, at: Node<'t>) -> Option<Node<'t>> {
        let mut ancestor = at.parent();
        while let Some(node) = ancestor {
            if matches!(node.kind(), "method_definition" | "method_declaration") {
                return Some(node);
            }
            ancestor = node.parent();
        }
        None
    }

    fn owner<'t>(&self, at: Node<'t>) -> Option<Node<'t>> {
        let mut ancestor = at.parent();
        while let Some(node) = ancestor {
            if *self.language == Language::TypeScript
                && matches!(node.kind(), "function_declaration" | "function_expression")
            {
                return None;
            }
            if matches!(
                node.kind(),
                "class_declaration" | "interface_declaration" | "impl_item"
            ) {
                return Some(node);
            }
            ancestor = node.parent();
        }
        None
    }

    fn member(
        &self,
        owner: Node<'_>,
        name: &str,
        arity: Option<usize>,
        call: bool,
        instance: bool,
    ) -> Option<String> {
        let body = owner.child_by_field_name("body")?;
        let declarations = self.scopes.get(&body.id())?.names.get(name)?;
        let declarations = declarations
            .iter()
            .filter(|d| {
                let Some(node) = self.declaration_node(d) else {
                    return false;
                };
                let is_static = self.is_static(node);
                // Java permits this.staticMethod(); TS instance and static member
                // namespaces are distinct. Java type-qualified calls must be static.
                if !instance {
                    is_static
                } else {
                    *self.language == Language::Java || !is_static
                }
            })
            .collect();
        self.select(declarations, arity, call)
    }

    fn rust_path(
        &self,
        at: Node<'_>,
        receiver: &str,
        member: &str,
        arity: Option<usize>,
        call: bool,
    ) -> Option<String> {
        let id = self.select(self.lexical(at, receiver, Namespace::Type), None, false)?;
        let owner = self.entity_node(&id)?;
        if owner.kind() == "mod_item" {
            let body = owner.child_by_field_name("body")?;
            let declarations = self
                .scopes
                .get(&body.id())?
                .names
                .get(member)?
                .iter()
                .filter(|d| {
                    self.declaration_node(d)
                        .is_some_and(|n| self.text(n).trim_start().starts_with("pub "))
                })
                .collect();
            return self.select(declarations, arity, call);
        }
        self.rust_associated(owner, member, arity, call, false)
    }

    fn rust_associated(
        &self,
        owner: Node<'_>,
        name: &str,
        arity: Option<usize>,
        call: bool,
        instance: bool,
    ) -> Option<String> {
        let owner_name = self.text(owner.child_by_field_name("name")?);
        let scope = owner.parent()?;
        let mut cursor = scope.walk();
        let mut declarations = Vec::new();
        for item in scope
            .named_children(&mut cursor)
            .filter(|n| n.kind() == "impl_item")
        {
            if item
                .child_by_field_name("type")
                .is_none_or(|n| self.text(n) != owner_name)
            {
                continue;
            }
            if let Some(trait_node) = item.child_by_field_name("trait") {
                if !instance
                    || self
                        .select(
                            self.lexical(item, self.text(trait_node), Namespace::Type),
                            None,
                            false,
                        )
                        .is_none()
                {
                    continue;
                }
            }
            let body = item.child_by_field_name("body")?;
            if let Some(members) = self.scopes.get(&body.id()).and_then(|s| s.names.get(name)) {
                declarations.extend(members.iter().filter(|d| {
                    self.declaration_node(d)
                        .and_then(|n| n.child_by_field_name("parameters"))
                        .is_some_and(|p| {
                            let mut cursor = p.walk();
                            let has_self = p
                                .named_children(&mut cursor)
                                .any(|n| n.kind() == "self_parameter");
                            has_self == instance
                        })
                }));
            }
        }
        self.select(declarations, arity, call)
    }

    pub(crate) fn member_of(&self, method: &CodeEntity, owner: &CodeEntity) -> bool {
        let Some(method_node) = self.entity_node(&method.identity) else {
            return false;
        };
        let Some(owner_node) = self.entity_node(&owner.identity) else {
            return false;
        };
        if method_node.parent().and_then(|p| p.parent()) == Some(owner_node) {
            return true;
        }
        if *self.language != Language::Rust {
            return false;
        }
        let Some(implementation) = method_node
            .parent()
            .and_then(|p| p.parent())
            .filter(|p| p.kind() == "impl_item")
        else {
            return false;
        };
        let Some(ty) = implementation.child_by_field_name("type") else {
            return false;
        };
        self.select(
            self.lexical(implementation, self.text(ty), Namespace::Type),
            None,
            false,
        )
        .as_deref()
            == Some(owner.identity.as_str())
    }

    pub(crate) fn signature(&self, entity: &CodeEntity) -> Option<Vec<String>> {
        let node = self.entity_node(&entity.identity)?;
        let name = node.child_by_field_name("name")?;
        let prefix = &self.source[node.start_byte()..name.start_byte()];
        if prefix
            .split_whitespace()
            .any(|w| matches!(w, "static" | "private"))
        {
            return None;
        }
        let parameters = node.child_by_field_name("parameters")?;
        let mut cursor = parameters.walk();
        let mut signature = parameters
            .named_children(&mut cursor)
            .filter(|p| p.kind() != "comment")
            .map(|p| {
                if p.kind() == "self_parameter" {
                    return Some(self.text(p).replace(' ', ""));
                }
                if matches!(
                    p.kind(),
                    "spread_parameter" | "variadic_parameter" | "optional_parameter"
                ) {
                    return None;
                }
                let ty = p.child_by_field_name("type")?;
                self.signature_type(entity, self.text(ty).trim_start_matches(':').trim())
            })
            .collect::<Option<Vec<_>>>()?;
        let return_type = self
            .declared_type(entity)
            .or_else(|| (*self.language == Language::Rust).then(|| "()".to_owned()))?;
        signature.push(self.signature_type(entity, &return_type)?);
        Some(signature)
    }

    fn signature_type(&self, entity: &CodeEntity, name: &str) -> Option<String> {
        if let RelationTarget::ResolvedEntity(id) = self.type_target(entity, name) {
            return Some(id);
        }
        let at = self.entity_node(&entity.identity)?;
        if !self.lexical(at, name, Namespace::Type).is_empty() {
            return None;
        }
        let builtin = match self.language {
            Language::TypeScript => matches!(name, "string" | "boolean" | "number" | "void"),
            Language::Java => {
                matches!(
                    name,
                    "void"
                        | "int"
                        | "boolean"
                        | "char"
                        | "long"
                        | "double"
                        | "float"
                        | "byte"
                        | "short"
                ) || (name == "String" && !self.scopes.values().any(|scope| scope.wildcard))
            }
            Language::Rust => matches!(
                name,
                "str" | "&str" | "bool" | "char" | "i32" | "usize" | "()"
            ),
            _ => false,
        };
        builtin.then(|| name.to_owned())
    }

    pub(crate) fn declared_type(&self, entity: &CodeEntity) -> Option<String> {
        let node = self.entity_node(&entity.identity)?;
        let ty = match self.language {
            Language::Rust => node.child_by_field_name("return_type"),
            Language::TypeScript => node
                .child_by_field_name("return_type")
                .or_else(|| node.child_by_field_name("type")),
            Language::Java => node.child_by_field_name("type"),
            _ => None,
        }?;
        Some(self.text(ty).trim_start_matches(':').trim().to_owned())
    }

    pub(crate) fn type_target(&self, entity: &CodeEntity, name: &str) -> RelationTarget {
        let id = self
            .entity_node(&entity.identity)
            .and_then(|at| self.resolve(at, name, Namespace::Type, None, false));
        id.map(RelationTarget::ResolvedEntity).unwrap_or_else(|| self.unresolved(entity, name, "type binding is builtin, external, shadowed, ambiguous, or unsupported in this lexical/module scope"))
    }
}

fn is_function(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "function_declaration"
            | "function_expression"
            | "arrow_function"
            | "function_item"
            | "function_signature_item"
            | "method_declaration"
            | "constructor_declaration"
            | "method_definition"
            | "method_signature"
    )
}
fn is_scope(node: Node<'_>) -> bool {
    node.parent().is_none()
        || is_function(node)
        || matches!(
            node.kind(),
            "statement_block"
                | "block"
                | "class_body"
                | "interface_body"
                | "declaration_list"
                | "impl_item"
                | "class_declaration"
                | "catch_clause"
                | "for_statement"
                | "for_in_statement"
                | "for_expression"
                | "lambda_expression"
                | "closure_expression"
                | "match_arm"
                | "if_expression"
                | "while_expression"
                | "enhanced_for_statement"
        )
}
fn exact_node<'t>(root: Node<'t>, range: &SourceRange) -> Option<Node<'t>> {
    let start = tree_sitter::Point::new(range.start.line as usize, range.start.column as usize);
    let end = tree_sitter::Point::new(range.end.line as usize, range.end.column as usize);
    let mut node = root.descendant_for_point_range(start, end)?;
    loop {
        if node.start_position() == start && node.end_position() == end {
            return Some(node);
        }
        node = node.parent()?;
    }
}

fn entity_syntax<'t>(root: Node<'t>, range: &SourceRange, kind: &str) -> Option<Node<'t>> {
    let start = tree_sitter::Point::new(range.start.line as usize, range.start.column as usize);
    let end = tree_sitter::Point::new(range.end.line as usize, range.end.column as usize);
    let mut node = root.descendant_for_point_range(start, end)?;
    loop {
        if node.kind() == kind && node.end_position() == end {
            return Some(node);
        }
        node = node.parent()?;
    }
}
