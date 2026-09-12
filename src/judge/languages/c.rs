use crate::judge::languages::Language;

#[derive(Debug)]
pub struct C;

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
