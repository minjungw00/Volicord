//! Keep graph locality stable when a new observation rebinds entity identities.
use crate::model::{Language, RelationTarget, SemanticAnalysisResult, SourceRange, StructuralFact};
use std::collections::HashMap;

type RangeKey<'a> = Option<(&'a str, u64, u64, u64, u64)>;

fn range_key(range: &Option<SourceRange>) -> RangeKey<'_> {
    range.as_ref().map(|range| {
        (
            range.locator.as_str(),
            range.start.line,
            range.start.column,
            range.end.line,
            range.end.column,
        )
    })
}

#[derive(Eq, Ord, PartialEq, PartialOrd)]
enum TargetKey<'a> {
    Resolved(usize),
    Unresolved(&'a str, &'a Option<Language>, &'a Option<String>, &'a str),
    Missing(&'a str),
}

fn target_key<'a>(target: &'a RelationTarget, ranks: &HashMap<String, usize>) -> TargetKey<'a> {
    match target {
        RelationTarget::ResolvedEntity(identity) => ranks
            .get(identity)
            .map_or(TargetKey::Missing(identity), |rank| {
                TargetKey::Resolved(*rank)
            }),
        RelationTarget::Unresolved(target) => TargetKey::Unresolved(
            &target.display,
            &target.language,
            &target.locator_hint,
            &target.reason,
        ),
    }
}

fn entity_ranks(facts: &[StructuralFact]) -> HashMap<String, usize> {
    facts
        .iter()
        .enumerate()
        .map(|(rank, fact)| (fact.entity.identity.clone(), rank))
        .collect()
}

pub(crate) fn order_structural_facts(facts: &mut [StructuralFact]) {
    facts.sort_by(|left, right| {
        let left = &left.entity;
        let right = &right.entity;
        (
            &left.area,
            &left.language,
            range_key(&left.source_range),
            &left.kind,
            &left.display_name,
            &left.qualified_name,
            &left.identity,
        )
            .cmp(&(
                &right.area,
                &right.language,
                range_key(&right.source_range),
                &right.kind,
                &right.display_name,
                &right.qualified_name,
                &right.identity,
            ))
    });
    let ranks = entity_ranks(facts);
    for fact in facts {
        fact.relations.sort_by(|left, right| {
            (
                &left.kind,
                range_key(&left.supporting_range),
                target_key(&left.target, &ranks),
                &left.identity,
            )
                .cmp(&(
                    &right.kind,
                    range_key(&right.supporting_range),
                    target_key(&right.target, &ranks),
                    &right.identity,
                ))
        });
    }
}

pub(crate) fn order_semantic_results(
    facts: &[StructuralFact],
    results: &mut [SemanticAnalysisResult],
) {
    let ranks = entity_ranks(facts);
    results.sort_by(|left, right| {
        let left = &left.relation;
        let right = &right.relation;
        (
            ranks.get(&left.source_entity),
            &left.kind,
            range_key(&left.supporting_range),
            target_key(&left.target, &ranks),
            &left.identity,
        )
            .cmp(&(
                ranks.get(&right.source_entity),
                &right.kind,
                range_key(&right.supporting_range),
                target_key(&right.target, &ranks),
                &right.identity,
            ))
    });
}
