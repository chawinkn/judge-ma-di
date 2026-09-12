use crate::judge::languages::Language;

#[derive(Debug, Default)]
pub struct Python;

impl Python {
    pub const fn new() -> Self {
        Self
    }
}

impl Language for Python {
    fn name(&self) -> &'static str {
        "python"
    }

    fn ext(&self) -> &'static str {
        "py"
    }

    fn compile_command(&self) -> Option<Vec<String>> {
        Some(vec![
            "/usr/bin/python3".to_string(),
            "-m".to_string(),
            "compileall".to_string(),
            "source.py".to_string(),
            "-b".to_string(),
        ])
    }

    fn run_command(&self) -> Vec<String> {
        vec!["/usr/bin/python3".to_string(), "source.py".to_string()]
    }
}
