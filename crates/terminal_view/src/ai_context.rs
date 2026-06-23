use terminal::terminal::SshTerminalConfig;

const TERMINAL_AI_SYSTEM_INSTRUCTION: &str = r#"你是终端侧边栏中的 Linux 命令助手，默认面向 Linux shell 环境回答。
请严格遵循以下规则：
1. 当用户请求安装、配置、排查、运维或执行命令时，优先返回可以直接在 Linux 终端执行的命令。
2. 所有命令都必须放在 Markdown 代码块中，代码块语言使用 bash。
3. 每个代码块只能包含一条命令，不要在同一个代码块中放多条命令，不要使用 &&、; 或换行把多个命令塞进同一个代码块，除非用户明确要求组合命令。
4. 如果任务需要多步骤，请拆成多个独立代码块，每个代码块只对应一步的一条命令。
5. 解释、注意事项、风险提示、步骤标题必须写在代码块外面，保持简洁。
6. 如果命令依赖 sudo、包管理器或发行版差异，请先简短说明再给命令。
7. 如果用户明确要求非 Linux 平台、非命令答案或更详细的解释，再按用户要求调整。"#;

const SSH_CONTEXT_HEADER: &str =
    "\n\n当前 SSH 会话上下文如下；如果用户没有特别说明，请优先基于这些信息回答：\n";
const SSH_CONTEXT_SUFFIX: &str = "\n在给出命令前，请先判断它是否适合当前 SSH 主机和工作目录。";

fn normalize(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn push_context_line(lines: &mut Vec<String>, label: &str, value: Option<&str>) {
    if let Some(value) = normalize(value) {
        lines.push(format!("- {label}：{value}"));
    }
}

pub(crate) fn build_terminal_ai_system_instruction(
    connection_name: Option<&str>,
    ssh_config: Option<&SshTerminalConfig>,
    working_dir: Option<&str>,
) -> String {
    let mut instruction = TERMINAL_AI_SYSTEM_INSTRUCTION.trim().to_string();
    let mut context_lines = Vec::new();

    push_context_line(&mut context_lines, "连接名称", connection_name);

    if let Some(config) = ssh_config {
        push_context_line(
            &mut context_lines,
            "SSH 主机",
            Some(&config.ssh_config.host),
        );
        push_context_line(
            &mut context_lines,
            "SSH 用户",
            Some(&config.ssh_config.username),
        );
        context_lines.push(format!("- SSH 端口：{}", config.ssh_config.port));
    }

    push_context_line(&mut context_lines, "当前远程目录", working_dir);

    if !context_lines.is_empty() {
        instruction.push_str(SSH_CONTEXT_HEADER);
        instruction.push_str(&context_lines.join("\n"));
        instruction.push_str(SSH_CONTEXT_SUFFIX);
    }

    instruction
}

#[cfg(test)]
mod tests {
    use super::build_terminal_ai_system_instruction;
    use ssh::{SshAuth, SshConnectConfig};
    use terminal::terminal::{PtyConfig, SshTerminalConfig};

    fn sample_ssh_config() -> SshTerminalConfig {
        SshTerminalConfig {
            ssh_config: SshConnectConfig {
                host: "prod.example.com".to_string(),
                port: 2222,
                username: "deploy".to_string(),
                auth: SshAuth::Password("secret".to_string()),
                timeout: None,
                keepalive_interval: None,
                keepalive_max: None,
                jump_server: None,
                proxy: None,
                keyboard_interactive_responder: None,
            },
            pty_config: PtyConfig::default(),
            disable_shell_integration: false,
        }
    }

    #[test]
    fn build_terminal_ai_system_instruction_keeps_base_prompt_without_ssh_context() {
        let instruction = build_terminal_ai_system_instruction(None, None, None);

        assert!(instruction.contains("Linux 命令助手"));
        assert!(!instruction.contains("当前 SSH 会话上下文"));
    }

    #[test]
    fn build_terminal_ai_system_instruction_appends_ssh_context() {
        let config = sample_ssh_config();

        let instruction =
            build_terminal_ai_system_instruction(Some("生产环境"), Some(&config), Some("/srv/app"));

        assert!(instruction.contains("当前 SSH 会话上下文"));
        assert!(instruction.contains("- 连接名称：生产环境"));
        assert!(instruction.contains("- SSH 主机：prod.example.com"));
        assert!(instruction.contains("- SSH 用户：deploy"));
        assert!(instruction.contains("- SSH 端口：2222"));
        assert!(instruction.contains("- 当前远程目录：/srv/app"));
        assert!(!instruction.contains("secret"));
    }
}
