// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Utilities to extract examples from
//! [The Rust Reference](https://doc.rust-lang.org/nightly/reference),
//! run them through RMC, and display their results.

use crate::dashboard;
use pulldown_cmark::{Parser, Tag};
use std::{
    collections::HashMap,
    env, fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

/// Parses the chapter/section hierarchy in the markdown file specified by
/// `summary_path` and returns a mapping from markdown files containing rust
/// code to corresponding directories where the extracted rust code should
/// reside.
fn parse_hierarchy(summary_path: &Path) -> HashMap<PathBuf, PathBuf> {
    let start = "# The Rust Reference\n\n[Introduction](introduction.md)";
    let summary = fs::read_to_string(summary_path).unwrap();
    assert!(summary.starts_with(start), "Error: The start of the summary file changed.");
    // Skip the title and introduction.
    let n = Parser::new(start).count();
    let parser = Parser::new(&summary).skip(n);
    // Set "ref" as the root of the hierarchical path.
    let mut hierarchy = PathBuf::from("ref");
    let mut map = HashMap::new();
    // Introduction is a especial case, so handle it separately.
    map.insert(PathBuf::from("introduction.md"), hierarchy.join("Introduction"));
    for event in parser {
        match event {
            pulldown_cmark::Event::End(Tag::Item) => {
                // Pop the current chapter/section from the hierarchy once
                // we are done processing it and its subsections.
                hierarchy.pop();
            }
            pulldown_cmark::Event::End(Tag::Link(_, path, _)) => {
                // At the start of the link tag, the hierarchy does not yet
                // contain the title of the current chapter/section. So, we wait
                // for the end of the link tag before adding the path and
                // hierarchy of the current chapter/section to the map.
                map.insert(path.split('/').collect(), hierarchy.clone());
            }
            pulldown_cmark::Event::Text(text) => {
                // Add the current chapter/section title to the hierarchy.
                hierarchy.push(text.to_string());
            }
            _ => (),
        }
    }
    map
}

/// Extracts examples from the given relative `paths` in the `book_dir` and
/// saves them in `gen_dir`.
fn extract_examples(paths: Vec<&PathBuf>, book_dir: &Path, gen_dir: &Path) {
    for path in paths {
        let mut cmd = Command::new("rustdoc");
        cmd.args([
            "+nightly",
            "--test",
            "-Z",
            "unstable-options",
            book_dir.join(path).to_str().unwrap(),
            "--test-builder",
            &["src", "tools", "dashboard", "print.sh"]
                .iter()
                .collect::<PathBuf>()
                .to_str()
                .unwrap(),
            "--persist-doctests",
            gen_dir.to_str().unwrap(),
            "--no-run",
        ]);
        cmd.stdout(Stdio::null());
        cmd.spawn().unwrap().wait().unwrap();
    }
}

/// Copies the extracted rust code in `from_dir` to `src/test` following the
/// hierarchy specified by `map`.
fn organize_examples(map: &HashMap<PathBuf, PathBuf>, book_dir: &Path, from_dir: &Path) {
    // The names of the extracted examples generated by `rustdoc` have the
    // format `<path>_<line-num>_<test-num>` where occurrences of '/', '-', and
    // '.' in <path> are replaced by '_'. This transformation is not injective,
    // so we cannot map those names back to the original markdown file path.
    // Instead, we apply the same transformation on the keys of `map` in the for
    // loop below and lookup <path> in those modified keys.
    let mut modified_map = HashMap::new();
    for (path, hierarchy) in map.iter() {
        modified_map.insert(
            book_dir.join(path).to_str().unwrap().replace(&['\\', '/', '-', '.'][..], "_"),
            hierarchy.clone(),
        );
    }
    for dir in from_dir.read_dir().unwrap() {
        let dir = dir.unwrap().path();
        // Some directories do not contain tests because the markdown file
        // instructs `rustdoc` to ignore those tests.
        if let Some(example) = dir.read_dir().unwrap().next() {
            let example = example.unwrap().path();
            copy(&example, &modified_map);
        }
    }
}

/// Copy the file specified by `from` to the corresponding location specified by
/// `map`.
fn copy(from: &Path, map: &HashMap<String, PathBuf>) {
    // The path specified by `from` has the form:
    // `src/tools/dashboard/target/ref/<key>_<line-num>_<test-num>/rust_out`
    // We copy the file in this path to a new path of the form:
    // `src/test/<val>/<line-num>.rs
    // where `map[<key>] == <val>`. We omit <test-num> because all tests have
    // the same number, 0.
    // Extract `<key>_<line-num>_<test-num>`.
    let key_line_test = from.parent().unwrap().file_name().unwrap().to_str().unwrap();
    // Extract <key> and <line-num> from `key_line_test` to get <val> and
    // construct destination path.
    let splits: Vec<_> = key_line_test.rsplitn(3, '_').collect();
    let key = splits[2];
    let line = splits[1];
    let val = &map[key];
    let name = &format!("{}.rs", line);
    let to = Path::new("src").join("test").join(val).join(name);
    fs::create_dir_all(to.parent().unwrap()).unwrap();
    fs::copy(&from, &to).unwrap();
}

/// Pre-processes the tests in the specified `paths` before running them with
/// `compiletest`.
fn preprocess_examples(_paths: Vec<&PathBuf>) {
    // For now, we will only pre-process the tests that cause infinite loops.
    // TODO: properly implement this step (see issue #324).
    let loop_tests: [PathBuf; 4] = [
        ["src", "test", "ref", "Appendices", "Glossary", "263.rs"].iter().collect(),
        ["src", "test", "ref", "Linkage", "190.rs"].iter().collect(),
        [
            "src",
            "test",
            "ref",
            "Statements and expressions",
            "Expressions",
            "Loop expressions",
            "133.rs",
        ]
        .iter()
        .collect(),
        [
            "src",
            "test",
            "ref",
            "Statements and expressions",
            "Expressions",
            "Method call expressions",
            "10.rs",
        ]
        .iter()
        .collect(),
    ];

    for test in loop_tests {
        let code = fs::read_to_string(&test).unwrap();
        let code = format!("// cbmc-flags: --unwind 1 --unwinding-assertions\n{}", code);
        fs::write(&test, code).unwrap();
    }
}

/// Runs `compiletest` on the `suite` and logs the results to `log_path`.
fn run_examples(suite: &str, log_path: &Path) {
    // Before executing this program, `cargo` populates the environment with
    // build configs. `x.py` respects those configs, causing a recompilation
    // of `rustc`. This is not a desired behavior, so we remove those configs.
    let filtered_env: HashMap<String, String> = env::vars()
        .filter(|&(ref k, _)| {
            !(k.contains("CARGO") || k.contains("LD_LIBRARY_PATH") || k.contains("RUST"))
        })
        .collect();
    let mut cmd = Command::new([".", "x.py"].iter().collect::<PathBuf>());
    cmd.args([
        "test",
        suite,
        "-i",
        "--stage",
        "1",
        "--test-args",
        "--logfile",
        "--test-args",
        log_path.to_str().unwrap(),
    ]);
    cmd.env_clear().envs(filtered_env);
    cmd.stdout(Stdio::null());

    cmd.spawn().unwrap().wait().unwrap();
}

/// Creates a new [`Tree`] from `path`, and a test `result`.
fn tree_from_path(mut path: Vec<String>, result: bool) -> dashboard::Tree {
    assert!(path.len() > 0, "Error: `path` must contain at least 1 element.");
    let mut tree = dashboard::Tree::new(
        dashboard::Node::new(
            path.pop().unwrap(),
            if result { 1 } else { 0 },
            if result { 0 } else { 1 },
        ),
        vec![],
    );
    for _ in 0..path.len() {
        tree = dashboard::Tree::new(
            dashboard::Node::new(path.pop().unwrap(), tree.data.num_pass, tree.data.num_fail),
            vec![tree],
        );
    }
    tree
}

/// Parses and generates a dashboard from the log output of `compiletest` in
/// `path`.
fn parse_log(path: &Path) -> dashboard::Tree {
    let file = fs::File::open(path).unwrap();
    let reader = BufReader::new(file);
    let mut tests = dashboard::Tree::new(dashboard::Node::new(String::from("ref"), 0, 0), vec![]);
    for line in reader.lines() {
        let (ns, l) = parse_log_line(&line.unwrap());
        tests = dashboard::Tree::merge(tests, tree_from_path(ns, l)).unwrap();
    }
    tests
}

/// Parses a line in the log output of `compiletest` and returns a pair containing
/// the path to a test and its result.
fn parse_log_line(line: &str) -> (Vec<String>, bool) {
    // Each line has the format `<result> [rmc] <path>`. Extract <result> and
    // <path>.
    let splits: Vec<_> = line.split(" [rmc] ").map(String::from).collect();
    let l = if splits[0].as_str() == "ok" { true } else { false };
    let mut ns: Vec<_> = splits[1].split(&['/', '.'][..]).map(String::from).collect();
    // Remove unnecessary `.rs` suffix.
    ns.pop();
    (ns, l)
}

/// Display the dashboard in the terminal.
fn display_dashboard(dashboard: dashboard::Tree) {
    println!(
        "# of tests: {}\t✔️ {}\t❌ {}",
        dashboard.data.num_pass + dashboard.data.num_fail,
        dashboard.data.num_pass,
        dashboard.data.num_fail
    );
    println!("{}", dashboard);
}

/// Extracts examples from The Rust Reference, run them through RMC, and
/// displays their results in a terminal dashboard.
pub fn display_reference_dashboard() {
    let summary_path: PathBuf = ["src", "doc", "reference", "src", "SUMMARY.md"].iter().collect();
    let ref_dir: PathBuf = ["src", "doc", "reference", "src"].iter().collect();
    let gen_dir: PathBuf = ["src", "tools", "dashboard", "target", "ref"].iter().collect();
    let log_path: PathBuf = ["src", "tools", "dashboard", "target", "ref.log"].iter().collect();
    // Parse the chapter/section hierarchy from the table of contents in The
    // Rust Reference.
    let map = parse_hierarchy(&summary_path);
    // Extract examples from The Rust Reference.
    extract_examples(map.keys().collect(), &ref_dir, &gen_dir);
    // Reorganize those examples following the The Rust Reference hierarchy.
    organize_examples(&map, &ref_dir, &gen_dir);
    // Pre-process the examples before running them through `compiletest`.
    preprocess_examples(map.values().collect());
    // Run `compiletest` on the reference examples.
    run_examples("ref", &log_path);
    // Parse `compiletest` log file.
    let dashboard = parse_log(&log_path);
    // Display the reference dashboard.
    display_dashboard(dashboard);
}
