pub mod c;
pub mod cpp;
pub mod python;

pub use c::C;
pub use cpp::Cpp;
pub use python::Python;

use crate::error::AppError;
use std::fmt::Debug;

pub trait Language: Send + Sync + Debug {
    fn name(&self) -> &'static str;
    fn ext(&self) -> &'static str;

    fn source_filename(&self) -> String {
        format!("source.{}", self.ext())
    }

    fn compiler(&self) -> Option<&'static str> {
        None
    }

    fn compiler_flags(&self) -> &'static [&'static str] {
        &[]
    }

    fn compile_command(&self) -> Option<Vec<String>> {
        let compiler = self.compiler()?;
        let artifact = self.compiled_artifact()?;
        let mut cmd = vec![compiler.to_string()];
        cmd.extend(self.compiler_flags().iter().map(|s| s.to_string()));
        cmd.push(self.source_filename());
        cmd.push("-o".to_string());
        cmd.push(artifact.to_string());
        Some(cmd)
    }

    fn compiled_artifact(&self) -> Option<&'static str> {
        if self.compiler().is_some() {
            Some("source")
        } else {
            None
        }
    }

    fn run_command(&self) -> Vec<String> {
        vec!["./source".to_string()]
    }

    fn custom_checker(&self) -> Option<&'static str> {
        None
    }
}

pub static CPP: Cpp = Cpp;
pub static C_LANG: C = C;
pub static PYTHON: Python = Python;

pub static ALL_LANGUAGES: &[&'static dyn Language] = &[&CPP, &C_LANG, &PYTHON];

pub fn supported_languages() -> &'static [&'static dyn Language] {
    ALL_LANGUAGES
}

pub fn get_language(name: &str) -> Result<&'static dyn Language, AppError> {
    match name {
        "cpp" => Ok(&CPP),
        "c" => Ok(&C_LANG),
        "python" => Ok(&PYTHON),
        _ => Err(AppError::BadRequest("Unsupported Language".to_string())),
    }
}
