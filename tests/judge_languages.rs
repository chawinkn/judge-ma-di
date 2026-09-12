use judge_ma_di::judge::languages::{get_language, Language};

#[test]
fn cpp_adapter_metadata_and_commands() {
    let cpp = get_language("cpp").unwrap();
    assert_eq!(cpp.name(), "cpp");
    assert_eq!(cpp.ext(), "cpp");
    assert_eq!(cpp.compiled_artifact(), Some("source"));
    assert_eq!(
        cpp.compile_command().unwrap(),
        vec![
            "/usr/bin/g++",
            "--std=c++17",
            "-O2",
            "source.cpp",
            "-o",
            "source"
        ]
    );
    assert_eq!(cpp.run_command(), vec!["./source"]);
    assert_eq!(cpp.custom_checker(), None);
}

#[test]
fn c_adapter_metadata_and_commands() {
    let c = get_language("c").unwrap();
    assert_eq!(c.name(), "c");
    assert_eq!(c.ext(), "c");
    assert_eq!(c.compiled_artifact(), Some("source"));
    assert_eq!(
        c.compile_command().unwrap(),
        vec![
            "/usr/bin/gcc",
            "--std=c11",
            "-O2",
            "source.c",
            "-o",
            "source"
        ]
    );
    assert_eq!(c.run_command(), vec!["./source"]);
    assert_eq!(c.custom_checker(), None);
}

#[test]
fn python_adapter_metadata_and_commands() {
    let py = get_language("python").unwrap();
    assert_eq!(py.name(), "python");
    assert_eq!(py.ext(), "py");
    assert_eq!(py.compiled_artifact(), None);
    assert_eq!(
        py.compile_command().unwrap(),
        vec!["/usr/bin/python3", "-m", "compileall", "source.py", "-b"]
    );
    assert_eq!(py.run_command(), vec!["/usr/bin/python3", "source.py"]);
    assert_eq!(py.custom_checker(), None);
}

#[test]
fn custom_compiled_language_uses_default_methods() {
    #[derive(Debug)]
    struct RustLang;

    impl Language for RustLang {
        fn name(&self) -> &'static str {
            "rust"
        }
        fn ext(&self) -> &'static str {
            "rs"
        }
        fn compiler(&self) -> Option<&'static str> {
            Some("/usr/bin/rustc")
        }
        fn compiler_flags(&self) -> &'static [&'static str] {
            &["-O"]
        }
    }

    let rust = RustLang;
    assert_eq!(rust.name(), "rust");
    assert_eq!(rust.ext(), "rs");
    assert_eq!(rust.compiled_artifact(), Some("source"));
    assert_eq!(
        rust.compile_command().unwrap(),
        vec!["/usr/bin/rustc", "-O", "source.rs", "-o", "source"]
    );
    assert_eq!(rust.run_command(), vec!["./source"]);
    assert_eq!(rust.custom_checker(), None);
}

#[test]
fn custom_domain_adapter_only_overwrites_run_and_checker() {
    #[derive(Debug)]
    struct SqlAdapter;

    // Only 3 methods needed: name, ext, and run_command!
    impl Language for SqlAdapter {
        fn name(&self) -> &'static str {
            "sql"
        }
        fn ext(&self) -> &'static str {
            "sql"
        }
        fn run_command(&self) -> Vec<String> {
            vec![
                "/usr/bin/sqlite3".to_string(),
                "-init".to_string(),
                "source.sql".to_string(),
            ]
        }
        fn custom_checker(&self) -> Option<&'static str> {
            Some("rcmp4")
        }
    }

    let sql = SqlAdapter;
    assert_eq!(sql.name(), "sql");
    assert_eq!(sql.ext(), "sql");
    assert!(sql.compile_command().is_none());
    assert_eq!(sql.compiled_artifact(), None);
    assert_eq!(sql.custom_checker(), Some("rcmp4"));
}
