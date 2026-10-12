use crate::error::AppError;

#[derive(Debug)]
pub struct Compile {
    pub cmd: &'static [&'static str],
    pub artifact: &'static str,
}

#[derive(Debug)]
pub struct Language {
    pub name: &'static str,
    pub source_file: &'static str,
    pub compile: Option<Compile>,
    pub run: &'static [&'static str],
}

pub static LANGUAGES: &[Language] = &[
    Language {
        name: "cpp",
        source_file: "source.cpp",
        compile: Some(Compile {
            cmd: &[
                "/usr/bin/g++",
                "--std=c++17",
                "-O2",
                "source.cpp",
                "-o",
                "source",
            ],
            artifact: "source",
        }),
        run: &["./source"],
    },
    Language {
        name: "c",
        source_file: "source.c",
        compile: Some(Compile {
            cmd: &[
                "/usr/bin/gcc",
                "--std=c11",
                "-O2",
                "source.c",
                "-o",
                "source",
            ],
            artifact: "source",
        }),
        run: &["./source"],
    },
    Language {
        name: "python",
        source_file: "source.py",
        compile: Some(Compile {
            cmd: &[
                "/usr/bin/python3",
                "-m",
                "compileall",
                "-b",
                "-q",
                "source.py",
            ],
            artifact: "source.pyc",
        }),
        run: &["/usr/bin/python3", "source.pyc"],
    },
];

pub fn get_language(name: &str) -> Result<&'static Language, AppError> {
    LANGUAGES
        .iter()
        .find(|l| l.name == name)
        .ok_or_else(|| AppError::BadRequest("Unsupported Language".to_string()))
}
