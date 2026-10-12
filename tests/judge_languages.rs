use judge_ma_di::error::AppError;
use judge_ma_di::judge::languages::{get_language, LANGUAGES};

#[test]
fn cpp_entry() {
    let cpp = get_language("cpp").unwrap();
    assert_eq!(cpp.name, "cpp");
    assert_eq!(cpp.source_file, "source.cpp");
    let compile = cpp.compile.as_ref().unwrap();
    assert_eq!(
        compile.cmd,
        [
            "/usr/bin/g++",
            "--std=c++17",
            "-O2",
            "source.cpp",
            "-o",
            "source"
        ]
    );
    assert_eq!(compile.artifact, "source");
    assert_eq!(cpp.run, ["./source"]);
}

#[test]
fn c_entry() {
    let c = get_language("c").unwrap();
    assert_eq!(c.name, "c");
    assert_eq!(c.source_file, "source.c");
    let compile = c.compile.as_ref().unwrap();
    assert_eq!(
        compile.cmd,
        [
            "/usr/bin/gcc",
            "--std=c11",
            "-O2",
            "source.c",
            "-o",
            "source"
        ]
    );
    assert_eq!(compile.artifact, "source");
    assert_eq!(c.run, ["./source"]);
}

#[test]
fn python_runs_the_compiled_bytecode() {
    let py = get_language("python").unwrap();
    assert_eq!(py.name, "python");
    assert_eq!(py.source_file, "source.py");
    let compile = py.compile.as_ref().unwrap();
    assert_eq!(
        compile.cmd,
        [
            "/usr/bin/python3",
            "-m",
            "compileall",
            "-b",
            "-q",
            "source.py"
        ]
    );
    assert_eq!(compile.artifact, "source.pyc");
    assert_eq!(py.run, ["/usr/bin/python3", "source.pyc"]);
}

#[test]
fn get_language_finds_every_entry_and_rejects_unknown() {
    for language in LANGUAGES {
        assert_eq!(get_language(language.name).unwrap().name, language.name);
    }
    assert!(matches!(
        get_language("brainfuck"),
        Err(AppError::BadRequest(_))
    ));
}

#[test]
fn compile_command_builds_source_and_run_uses_artifact() {
    for language in LANGUAGES {
        let Some(compile) = &language.compile else {
            continue;
        };
        assert!(
            compile.cmd.contains(&language.source_file),
            "{}",
            language.name
        );
        assert!(
            language
                .run
                .iter()
                .any(|arg| arg.ends_with(compile.artifact)),
            "{}",
            language.name
        );
    }
}
