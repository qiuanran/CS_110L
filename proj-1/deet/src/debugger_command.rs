pub enum DebuggerCommand {
    Quit,
    Run(Vec<String>),
    Continue,
    Backtrace,
    Breakpoint(String),
}

impl DebuggerCommand {
    pub fn from_tokens(tokens: &Vec<&str>) -> Option<DebuggerCommand> {
        match tokens[0] {
            "q" | "quit" => Some(DebuggerCommand::Quit),
            "r" | "run" => {
                let args = tokens[1..].to_vec();
                Some(DebuggerCommand::Run(
                    args.iter().map(|s| s.to_string()).collect(),
                ))
            },
            "c" | "continue" | "cont"=> {
                Some(DebuggerCommand::Continue)
            },
            "bt" | "back" | "backtrace" => {
                Some(DebuggerCommand::Backtrace)
            },
            "b" | "breakpoint" | "break"  => {
                let arg = tokens[1].to_string();
                Some(DebuggerCommand::Breakpoint(arg))
            }
            // Default case:
            _ => None,
        }
    }
}