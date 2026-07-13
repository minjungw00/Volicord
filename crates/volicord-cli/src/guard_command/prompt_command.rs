use volicord_types::EvidenceRelevanceStatus;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum PromptCommandDetection {
    NoCommand,
    Command(PromptUserActionCommand),
    Blocked(PromptCommandBlock),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PromptUserActionCommand {
    pub(super) chat_id: String,
    pub(super) user_action_request_id: String,
    pub(super) resolution: PromptUserActionResolution,
    pub(super) verification_code: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum PromptUserActionResolution {
    Choice {
        selector: String,
        note: Option<String>,
    },
    EvidenceObservation {
        target: PromptEvidenceTarget,
        artifact_ids: Vec<String>,
        summary: String,
        relevance_status: EvidenceRelevanceStatus,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum PromptEvidenceTarget {
    AcceptanceCriterion(String),
    SupplementalClaim(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PromptCommandBlock {
    pub(super) code: &'static str,
    pub(super) message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RecordedPromptUserAction {
    pub(super) chat_id: String,
    pub(super) verification_code: String,
    pub(super) action_type: &'static str,
    pub(super) selected_option_id: Option<String>,
    pub(super) selected_target: Option<String>,
    pub(super) artifact_ids: Vec<String>,
    pub(super) relevance_status: Option<String>,
    pub(super) note_text_omitted: bool,
    pub(super) summary_text_omitted: bool,
    pub(super) replayed: bool,
    pub(super) model_context: String,
}

pub(super) fn parse_prompt_user_action_command(prompt: &str) -> PromptCommandDetection {
    let command_lines = prompt
        .lines()
        .filter_map(|line| line.trim_start().strip_prefix("Volicord:").map(str::trim))
        .collect::<Vec<_>>();
    if command_lines.is_empty() {
        return PromptCommandDetection::NoCommand;
    }

    let mut parsed = Vec::new();
    for line in command_lines {
        match parse_prompt_user_action_command_line(line) {
            Ok(command) => parsed.push(command),
            Err(message) => {
                return PromptCommandDetection::Blocked(PromptCommandBlock {
                    code: "malformed_user_action_command",
                    message,
                });
            }
        }
    }
    let Some(first) = parsed.first().cloned() else {
        return PromptCommandDetection::NoCommand;
    };
    if parsed.len() > 1 {
        return PromptCommandDetection::Blocked(PromptCommandBlock {
            code: "ambiguous_user_action_command",
            message: "Multiple Volicord user-action commands were found; send exactly one command."
                .to_owned(),
        });
    }
    PromptCommandDetection::Command(first)
}

fn parse_prompt_user_action_command_line(line: &str) -> Result<PromptUserActionCommand, String> {
    let tokens = tokenize(line)?;
    if tokens.first().map(String::as_str) != Some("resolve") {
        return Err(
            "Volicord user-action commands must start with `resolve` after `Volicord:`.".to_owned(),
        );
    }
    if tokens.len() < 7 {
        return Err(
            "Volicord resolve commands must include `A-N`, `--request <user_action_request_id>`, one stored form, and the displayed `#CODE`."
                .to_owned(),
        );
    }
    let chat_id = tokens[1].clone();
    validate_chat_id(&chat_id)?;
    let verification_code = normalize_verification_code(tokens.last().expect("length checked"))?;
    let form_tokens = &tokens[2..tokens.len() - 1];
    let options = parse_form_options(form_tokens)?;

    let user_action_request_id = options.value("request").ok_or_else(|| {
        "Volicord resolve commands require `--request <user_action_request_id>`.".to_owned()
    })?;
    let choice = options.value("choice");
    let note = options.value("note");
    let criterion = options.value("criterion");
    let claim = options.value("claim");
    let artifacts = options.values("artifact");
    let summary = options.value("summary");
    let contradicted = options.has("contradicted");
    let resolution = if let Some(selector) = choice {
        if criterion.is_some()
            || claim.is_some()
            || !artifacts.is_empty()
            || summary.is_some()
            || contradicted
        {
            return Err(
                "Choice resolve commands cannot include evidence-observation flags.".to_owned(),
            );
        }
        PromptUserActionResolution::Choice { selector, note }
    } else {
        if note.is_some() {
            return Err("`--note` requires the choice form.".to_owned());
        }
        if criterion.is_some() == claim.is_some() {
            return Err(
                "Evidence-observation resolve commands require exactly one of `--criterion` or `--claim`."
                    .to_owned(),
            );
        }
        if artifacts.is_empty() {
            return Err(
                "Evidence-observation resolve commands require at least one `--artifact`."
                    .to_owned(),
            );
        }
        let mut unique = std::collections::BTreeSet::new();
        if artifacts.iter().any(|artifact| !unique.insert(artifact)) {
            return Err("Each `--artifact` may be selected only once.".to_owned());
        }
        let summary = summary
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| {
                "Evidence-observation resolve commands require a non-empty `--summary`.".to_owned()
            })?;
        let target = if let Some(id) = criterion {
            PromptEvidenceTarget::AcceptanceCriterion(id)
        } else {
            PromptEvidenceTarget::SupplementalClaim(claim.expect("exclusive target checked"))
        };
        PromptUserActionResolution::EvidenceObservation {
            target,
            artifact_ids: artifacts,
            summary,
            relevance_status: if contradicted {
                EvidenceRelevanceStatus::Contradicted
            } else {
                EvidenceRelevanceStatus::Supported
            },
        }
    };

    Ok(PromptUserActionCommand {
        chat_id,
        user_action_request_id,
        resolution,
        verification_code,
    })
}

#[derive(Default)]
struct FormOptions {
    values: std::collections::BTreeMap<String, Vec<String>>,
}

impl FormOptions {
    fn value(&self, name: &str) -> Option<String> {
        self.values
            .get(name)
            .and_then(|values| values.first())
            .cloned()
    }

    fn values(&self, name: &str) -> Vec<String> {
        self.values.get(name).cloned().unwrap_or_default()
    }

    fn has(&self, name: &str) -> bool {
        self.values.contains_key(name)
    }
}

fn parse_form_options(tokens: &[String]) -> Result<FormOptions, String> {
    let mut options = FormOptions::default();
    let mut index = 0;
    while index < tokens.len() {
        let token = &tokens[index];
        let Some(name) = token.strip_prefix("--") else {
            return Err(format!("Unexpected resolve argument `{token}`."));
        };
        if name == "contradicted" {
            insert_option(&mut options, name, "true".to_owned(), false)?;
            index += 1;
            continue;
        }
        if !matches!(
            name,
            "request" | "choice" | "note" | "criterion" | "claim" | "artifact" | "summary"
        ) {
            return Err(format!("Unknown Volicord resolve option `--{name}`."));
        }
        index += 1;
        let value = tokens
            .get(index)
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| format!("Missing value for `--{name}`."))?;
        if value.starts_with("--") {
            return Err(format!("Missing value for `--{name}`."));
        }
        insert_option(&mut options, name, value.clone(), name == "artifact")?;
        index += 1;
    }
    Ok(options)
}

fn insert_option(
    options: &mut FormOptions,
    name: &str,
    value: String,
    repeated: bool,
) -> Result<(), String> {
    if repeated {
        options
            .values
            .entry(name.to_owned())
            .or_default()
            .push(value);
        return Ok(());
    }
    if options
        .values
        .insert(name.to_owned(), vec![value])
        .is_some()
    {
        return Err(format!("Option `--{name}` may be specified only once."));
    }
    Ok(())
}

fn tokenize(value: &str) -> Result<Vec<String>, String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut escaped = false;
    for character in value.chars() {
        if escaped {
            match character {
                'n' => current.push('\n'),
                't' => current.push('\t'),
                other => current.push(other),
            }
            escaped = false;
            continue;
        }
        match character {
            '\\' if quoted => escaped = true,
            '"' => quoted = !quoted,
            character if character.is_whitespace() && !quoted => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            other => current.push(other),
        }
    }
    if escaped || quoted {
        return Err("Volicord resolve command contains an unterminated quoted value.".to_owned());
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    Ok(tokens)
}

fn validate_chat_id(value: &str) -> Result<(), String> {
    parse_chat_id(value)
        .map(|_| ())
        .map_err(|error| error.message)
}

fn normalize_verification_code(value: &str) -> Result<String, String> {
    let Some(raw) = value.strip_prefix('#') else {
        return Err("Volicord verification code must start with `#`.".to_owned());
    };
    if raw.len() < 4 || raw.len() > 16 || !raw.chars().all(|ch| ch.is_ascii_alphanumeric()) {
        return Err("Volicord verification code must be the displayed `#CODE` token.".to_owned());
    }
    Ok(format!("#{}", raw.to_ascii_uppercase()))
}

pub(super) fn parse_chat_id(chat_id: &str) -> Result<usize, PromptCommandBlock> {
    let Some(raw_index) = chat_id.strip_prefix("A-") else {
        return Err(PromptCommandBlock {
            code: "invalid_user_action_id",
            message: format!("Volicord user-action id `{chat_id}` must use the chat form `A-N`."),
        });
    };
    if raw_index.is_empty() || !raw_index.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(PromptCommandBlock {
            code: "invalid_user_action_id",
            message: format!(
                "Volicord user-action id `{chat_id}` must use a positive numeric suffix."
            ),
        });
    }
    let index = raw_index.parse::<usize>().map_err(|_| PromptCommandBlock {
        code: "invalid_user_action_id",
        message: format!("Volicord user-action id `{chat_id}` is too large."),
    })?;
    if index == 0 {
        return Err(PromptCommandBlock {
            code: "invalid_user_action_id",
            message: "Volicord user-action ids start at A-1.".to_owned(),
        });
    }
    Ok(index)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_choice_and_observation_forms() {
        let choice = parse_prompt_user_action_command(
            "Volicord: resolve A-1 --request uar_choice --choice accept --note \"Approved locally\" #AB7K",
        );
        assert!(matches!(
            choice,
            PromptCommandDetection::Command(PromptUserActionCommand {
                resolution: PromptUserActionResolution::Choice { .. },
                ..
            })
        ));

        let observation = parse_prompt_user_action_command(
            "Volicord: resolve A-2 --request uar_observation --criterion criterion_1 --artifact artifact_1 --summary \"Checked output\" --contradicted #AB7K",
        );
        assert!(matches!(
            observation,
            PromptCommandDetection::Command(PromptUserActionCommand {
                resolution: PromptUserActionResolution::EvidenceObservation { .. },
                ..
            })
        ));
    }

    #[test]
    fn old_command_names_are_not_aliases() {
        for command in [
            "Volicord: answer A-1 1 #AB7K",
            "Volicord: note A-1 \"later\" #AB7K",
            "Volicord: observe A-1 --criterion criterion_1 #AB7K",
        ] {
            assert!(matches!(
                parse_prompt_user_action_command(command),
                PromptCommandDetection::Blocked(_)
            ));
        }
    }
}
