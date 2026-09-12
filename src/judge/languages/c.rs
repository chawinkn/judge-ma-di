use crate::judge::languages::Language;

#[derive(Debug, Default)]
pub struct C;

impl C {
    pub const fn new() -> Self {
        Self
    }
}

impl Language for C {
    fn name(&self) -> &'static str {
        "c"
    }

    fn ext(&self) -> &'static str {
        "c"
    }

    fn compiler(&self) -> Option<&'static str> {
        Some("/usr/bin/gcc")
    }

    fn compiler_flags(&self) -> &'static [&'static str] {
        &["--std=c11", "-O2"]
    }
}
