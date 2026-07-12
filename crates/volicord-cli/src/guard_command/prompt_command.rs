#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum PromptCommandDetection {
    NoCommand,
    Command(PromptJudgmentCommand),
    Blocked(PromptCommandBlock),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum PromptJudgmentCommand {
    Answer {
        chat_id: String,
        answer_selector: String,
        verification_code: String,
    },
    Note {
        chat_id: String,
        note: String,
        verification_code: String,
    },
}

impl PromptJudgmentCommand {
    pub(super) fn chat_id(&self) -> &str {
        match self {
            Self::Answer { chat_id, .. } | Self::Note { chat_id, .. } => chat_id,
        }
    }

    pub(super) fn verification_code(&self) -> &str {
        match self {
            Self::Answer {
                verification_code, ..
            }
            | Self::Note {
                verification_code, ..
            } => verification_code,
        }
    }

    pub(super) fn command_kind(&self) -> &'static str {
        match self {
            Self::Answer { .. } => "answer",
            Self::Note { .. } => "note",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PromptCommandBlock {
    pub(super) code: &'static str,
    pub(super) message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RecordedPromptJudgment {
    pub(super) command_kind: &'static str,
    pub(super) chat_id: String,
    pub(super) verification_code: String,
    pub(super) selected_option_id: String,
    pub(super) machine_action: String,
    pub(super) resolution_outcome: String,
    pub(super) note_text_omitted: bool,
    pub(super) replayed: bool,
    pub(super) model_context: String,
}

pub(super) fn parse_prompt_judgment_command(prompt: &str) -> PromptCommandDetection {
    let command_lines = prompt
        .lines()
        .filter_map(|line| line.trim_start().strip_prefix("Volicord:").map(str::trim))
        .collect::<Vec<_>>();
    if command_lines.is_empty() {
        return PromptCommandDetection::NoCommand;
    }

    let mut parsed = Vec::new();
    for line in command_lines {
        match parse_prompt_judgment_command_line(line) {
            Ok(command) => parsed.push(command),
            Err(message) => {
                return PromptCommandDetection::Blocked(PromptCommandBlock {
                    code: "malformed_judgment_command",
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
            code: "ambiguous_judgment_command",
            message: "Multiple Volicord judgment commands were found; send exactly one command."
                .to_owned(),
        });
    }
    PromptCommandDetection::Command(first)
}

fn parse_prompt_judgment_command_line(line: &str) -> Result<PromptJudgmentCommand, String> {
    let Some((action, rest)) = split_once_whitespace(line) else {
        return Err(
            "Volicord judgment commands must be `answer J-N OPTION #CODE` or `note J-N \"text\" #CODE`."
                .to_owned(),
        );
    };
    match action {
        "answer" => {
            let parts = rest.split_whitespace().collect::<Vec<_>>();
            if parts.len() == 2 {
                return Err(
                    "Volicord answer commands must include the displayed verification code."
                        .to_owned(),
                );
            }
            if parts.len() != 3 {
                return Err(
                    "Volicord answer commands must be exactly `Volicord: answer J-N OPTION #CODE`."
                        .to_owned(),
                );
            }
            validate_chat_id(parts[0])?;
            if parts[1].trim().is_empty() || parts[1].starts_with('"') {
                return Err("Volicord answer option must be a number or option id.".to_owned());
            }
            let verification_code = normalize_verification_code(parts[2])?;
            Ok(PromptJudgmentCommand::Answer {
                chat_id: parts[0].to_owned(),
                answer_selector: parts[1].to_owned(),
                verification_code,
            })
        }
        "note" => {
            let Some((chat_id, note_text)) = split_once_whitespace(rest) else {
                return Err(
                    "Volicord note commands must be exactly `Volicord: note J-N \"text\" #CODE`."
                        .to_owned(),
                );
            };
            validate_chat_id(chat_id)?;
            let (note, verification_code) = parse_quoted_note_and_code(note_text)?;
            Ok(PromptJudgmentCommand::Note {
                chat_id: chat_id.to_owned(),
                note,
                verification_code,
            })
        }
        _ => Err(
            "Volicord judgment commands must start with `answer` or `note` after `Volicord:`."
                .to_owned(),
        ),
    }
}

fn split_once_whitespace(value: &str) -> Option<(&str, &str)> {
    let trimmed = value.trim();
    let split_at = trimmed.find(char::is_whitespace)?;
    let (first, rest) = trimmed.split_at(split_at);
    Some((first, rest.trim_start()))
}

fn validate_chat_id(value: &str) -> Result<(), String> {
    parse_chat_id(value)
        .map(|_| ())
        .map_err(|message| message.message)
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

fn parse_quoted_note_and_code(value: &str) -> Result<(String, String), String> {
    let trimmed = value.trim();
    if !trimmed.starts_with('"') {
        return Err("Volicord note text must be a double-quoted string.".to_owned());
    }
    let mut output = String::new();
    let mut chars = trimmed[1..].chars();
    let mut escaped = false;
    while let Some(ch) = chars.next() {
        if escaped {
            match ch {
                '"' | '\\' => output.push(ch),
                'n' => output.push('\n'),
                't' => output.push('\t'),
                other => {
                    output.push('\\');
                    output.push(other);
                }
            }
            escaped = false;
            continue;
        }
        match ch {
            '\\' => escaped = true,
            '"' => {
                let rest = chars.as_str().trim();
                if rest.is_empty() {
                    return Err(
                        "Volicord note commands must include the displayed verification code."
                            .to_owned(),
                    );
                }
                if rest.split_whitespace().count() == 1 {
                    let verification_code = normalize_verification_code(rest)?;
                    return Ok((output, verification_code));
                }
                return Err(
                    "Volicord note commands accept only the verification code after the closing quote."
                        .to_owned(),
                );
            }
            other => output.push(other),
        }
    }
    Err("Volicord note text is missing a closing double quote.".to_owned())
}

pub(super) fn parse_chat_id(chat_id: &str) -> Result<usize, PromptCommandBlock> {
    let Some(raw_index) = chat_id.strip_prefix("J-") else {
        return Err(PromptCommandBlock {
            code: "invalid_judgment_id",
            message: format!("Volicord judgment id `{chat_id}` must use the chat form `J-N`."),
        });
    };
    if raw_index.is_empty() || !raw_index.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(PromptCommandBlock {
            code: "invalid_judgment_id",
            message: format!(
                "Volicord judgment id `{chat_id}` must use a positive numeric suffix."
            ),
        });
    }
    let index = raw_index.parse::<usize>().map_err(|_| PromptCommandBlock {
        code: "invalid_judgment_id",
        message: format!("Volicord judgment id `{chat_id}` is too large."),
    })?;
    if index == 0 {
        return Err(PromptCommandBlock {
            code: "invalid_judgment_id",
            message: "Volicord judgment ids start at J-1.".to_owned(),
        });
    }
    Ok(index)
}
