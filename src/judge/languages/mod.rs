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
        cmd.push(format!("source.{}", self.ext()));
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

pub static CPP: Cpp = Cpp::new();
pub static C_LANG: C = C::new();
pub static PYTHON: Python = Python::new();

pub fn get_language(name: &str) -> Result<&'static dyn Language, AppError> {
    match name {
        "cpp" => Ok(&CPP),
        "c" => Ok(&C_LANG),
        "python" => Ok(&PYTHON),
        _ => Err(AppError::BadRequest("Unsupported Language".to_string())),
    }
}
