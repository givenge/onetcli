use crate::llm::{Message, MessageBlock, Role};

const MIN_MESSAGE_TOKENS: usize = 4;

fn estimate_text_tokens(text: &str) -> usize {
    let chars = text.chars().count();
    (chars / 4).max(1)
}

fn estimate_message_tokens(message: &Message) -> usize {
    let content_tokens = message
        .content
        .iter()
        .filter_map(|block| match block {
            MessageBlock::Text { text } => Some(estimate_text_tokens(text)),
            _ => None,
        })
        .sum::<usize>();

    (content_tokens + MIN_MESSAGE_TOKENS).max(MIN_MESSAGE_TOKENS)
}

pub fn trim_messages_to_context_window(messages: Vec<Message>, max_tokens: usize) -> Vec<Message> {
    if max_tokens == 0 || messages.is_empty() {
        return messages;
    }

    let mut system_messages = Vec::new();
    let mut conversation_messages = Vec::new();

    for message in messages {
        if matches!(message.role, Role::System) {
            system_messages.push(message);
        } else {
            conversation_messages.push(message);
        }
    }

    let system_tokens = system_messages
        .iter()
        .map(estimate_message_tokens)
        .sum::<usize>();
    let available_tokens = max_tokens.saturating_sub(system_tokens);

    if available_tokens == 0 {
        return system_messages;
    }

    let mut selected = Vec::new();
    let mut used_tokens = 0usize;

    for message in conversation_messages.into_iter().rev() {
        let message_tokens = estimate_message_tokens(&message);
        let would_exceed = used_tokens + message_tokens > available_tokens;

        if would_exceed && !selected.is_empty() {
            break;
        }

        used_tokens += message_tokens;
        selected.push(message);

        if used_tokens >= available_tokens {
            break;
        }
    }

    selected.reverse();
    system_messages.extend(selected);
    system_messages
}

#[cfg(test)]
mod tests {
    use super::trim_messages_to_context_window;
    use crate::llm::{Message, MessageBlock, Role};

    fn text_content(message: &Message) -> String {
        message
            .content
            .iter()
            .filter_map(|block| match block {
                MessageBlock::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("")
    }

    #[test]
    fn keeps_latest_non_system_messages_within_budget() {
        let messages = vec![
            Message::text(Role::User, "first message"),
            Message::text(Role::Assistant, "first reply"),
            Message::text(Role::User, "latest message"),
        ];

        let trimmed = trim_messages_to_context_window(messages, 10);

        assert_eq!(trimmed.len(), 2);
        assert_eq!(text_content(&trimmed[0]), "first reply");
        assert_eq!(text_content(&trimmed[1]), "latest message");
    }

    #[test]
    fn preserves_system_messages() {
        let messages = vec![
            Message::text(Role::System, "system prompt"),
            Message::text(Role::User, "short"),
        ];

        let trimmed = trim_messages_to_context_window(messages, 4);

        assert_eq!(trimmed.len(), 1);
        assert!(matches!(trimmed[0].role, Role::System));
    }
}
