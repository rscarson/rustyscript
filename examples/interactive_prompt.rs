//!
//! This example demonstrates an interactive prompt for evaluating JavaScript expressions
//!
use rustyscript::{Runtime, RuntimeOptions};

fn main() {
    interactive_prompt()
}

fn interactive_prompt() {
    // Preload command stack from arguments
    let mut stack: Vec<String> = std::env::args().skip(1).collect();
    if stack.is_empty() {
        println!("Ready! Type expressions below!");
    } else {
        stack.insert(0, "exit".to_string());
    }

    let mut runtime = Runtime::new(RuntimeOptions {
        ..Default::default()
    })
    .expect("Failed to create runtime");

    loop {
        // Make sure we have a command ready
        if stack.is_empty() {
            stack.push(next_command());
        }
        let cmd = stack.pop().unwrap();

        if cmd.is_empty() {
            continue;
        } else if ["exit", "quit"].contains(&cmd.as_str()) {
            break;
        } else {
            // Process the next command
            let input = cmd.trim();
            match runtime.eval::<ResponseType>(input) {
                Ok(value) => println!("{value}\n"),
                Err(e) => eprintln!("{}\n", e.as_highlighted(Default::default())),
            }
        }
    }
}

fn next_command() -> String {
    let mut input = String::new();
    print!("> ");
    let _ = std::io::Write::flush(&mut std::io::stdout());

    loop {
        std::io::stdin()
            .read_line(&mut input)
            .expect("error: unable to read user input");
        if !input.trim().ends_with('\\') || input.trim().ends_with("\\\\") {
            break;
        }
    }

    input.trim().to_string()
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(untagged)]
enum ResponseType {
    String(String),
    Float(f64),
    Int(i64),
    Bool(bool),
    Array(Vec<ResponseType>),
    Map(std::collections::HashMap<String, ResponseType>),
    Null,
}

impl std::fmt::Display for ResponseType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResponseType::String(s) => write!(f, "{s}")?,
            ResponseType::Float(n) => write!(f, "{n}")?,
            ResponseType::Int(n) => write!(f, "{n}")?,
            ResponseType::Bool(b) => write!(f, "{b}")?,
            ResponseType::Null => write!(f, "")?,

            ResponseType::Array(a) => {
                write!(f, "[")?;
                let parts = a
                    .iter()
                    .map(|x| format!("{x:?}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "{parts}]")?;
            }

            ResponseType::Map(m) => {
                write!(f, "{{")?;
                let parts = m
                    .iter()
                    .map(|(k, v)| format!("{k}: {v:?}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "{parts}")?;
                write!(f, "}}")?;
            }
        }
        Ok(())
    }
}

impl std::fmt::Debug for ResponseType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResponseType::String(s) => {
                // Escape string
                let s = s
                    .replace('\n', "\\n")
                    .replace('\r', "\\r")
                    .replace('\t', "\\t")
                    .replace('\"', "\\\"");
                write!(f, "\"{s}\"")?;
            }

            ResponseType::Null => write!(f, "")?,

            _ => write!(f, "{self}")?,
        }
        Ok(())
    }
}
