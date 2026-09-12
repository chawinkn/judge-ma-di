use crate::judge::languages::Language;

#[derive(Debug, Default)]
pub struct Cpp;

impl Cpp {
    pub const fn new() -> Self {
        Self
    }
}

impl Language for Cpp {
    fn name(&self) -> &'static str {
        "cpp"
    }

    fn ext(&self) -> &'static str {
        "cpp"
    }

    fn compiler(&self) -> Option<&'static str> {
        Some("/usr/bin/g++")
    }

    fn compiler_flags(&self) -> &'static [&'static str] {
        &["--std=c++17", "-O2"]
    }
}
