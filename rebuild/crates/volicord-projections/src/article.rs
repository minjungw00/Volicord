//! Reader structure over admitted units, preserving their exact citation indices.
//! Roles select independent reading layers; they never author section names.
use crate::ExplanationQuestion;
use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ArticleLayer {
    Article,
    Verification,
    Limits,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ArticleSection {
    pub layer: ArticleLayer,
    pub heading_index: Option<usize>,
    pub unit_indices: Vec<usize>,
}

/// Shared by admission and every reader. Short, unheaded articles are valid.
/// Changing layer ends a section; headings cannot float away from their content.
pub fn article_sections(
    roles: impl IntoIterator<Item = ExplanationQuestion>,
) -> Result<Vec<ArticleSection>, String> {
    use ExplanationQuestion::*;
    let mut sections: Vec<ArticleSection> = Vec::new();
    let mut heading = None;
    for (index, role) in roles.into_iter().enumerate() {
        if role == SectionHeading {
            if heading.replace(index).is_some() {
                return Err("section heading requires content before another heading".into());
            }
            continue;
        }
        if matches!(role, Heading | Synopsis) {
            if heading.is_some() {
                return Err(
                    "section heading must precede article, verification or limits content".into(),
                );
            }
            continue;
        }
        let layer = match role {
            Verification => ArticleLayer::Verification,
            Limits => ArticleLayer::Limits,
            _ => ArticleLayer::Article,
        };
        if heading.is_some() || sections.last().is_none_or(|s| s.layer != layer) {
            sections.push(ArticleSection {
                layer,
                heading_index: heading.take(),
                unit_indices: Vec::new(),
            });
        }
        if let Some(section) = sections.last_mut() {
            section.unit_indices.push(index);
        }
    }
    if heading.is_some() {
        return Err("section heading requires supported content; omit empty sections".into());
    }
    Ok(sections)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ExplanationQuestion::*;

    #[test]
    fn author_order_and_independent_layers_preserve_original_indices() -> Result<(), String> {
        let sections = article_sections([
            Heading,
            Synopsis,
            Narrative,
            SectionHeading,
            Narrative,
            FlowStep,
            Verification,
            SectionHeading,
            Limits,
        ])?;
        assert_eq!(
            sections,
            vec![
                ArticleSection {
                    layer: ArticleLayer::Article,
                    heading_index: None,
                    unit_indices: vec![2]
                },
                ArticleSection {
                    layer: ArticleLayer::Article,
                    heading_index: Some(3),
                    unit_indices: vec![4, 5]
                },
                ArticleSection {
                    layer: ArticleLayer::Verification,
                    heading_index: None,
                    unit_indices: vec![6]
                },
                ArticleSection {
                    layer: ArticleLayer::Limits,
                    heading_index: Some(7),
                    unit_indices: vec![8]
                },
            ]
        );
        assert!(article_sections([Narrative]).is_ok());
        assert!(article_sections([SectionHeading]).is_err());
        assert!(article_sections([SectionHeading, SectionHeading, Narrative]).is_err());
        assert!(article_sections([SectionHeading, Synopsis]).is_err());
        Ok(())
    }
}
