// Copyright (c) Facebook, Inc. and its affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use clap::{App, AppSettings, Arg};
use itertools::Itertools;
use mirai_annotations::*;
use rustc::session::config::ErrorOutputType;
use rustc::session::early_error;
use shellwords;

/// Creates the clap::App metadata for argument parsing.
fn make_options_parser<'a>() -> App<'a, 'a> {
    // We could put this into lazy_static! with a Mutex around, but we really do not expect
    // to construct this more then once per regular program run.
    App::new("MIRAI")
    .setting(AppSettings::NoBinaryName)
    .version("v1.0.5")
    .arg(Arg::with_name("single_func")
        .long("single_func")
        .takes_value(true)
        .help("Focus analysis on the named function.")
        .long_help("Name is the simple name of a top-level crate function or a MIRAI summary key."))
    .arg(Arg::with_name("test_only")
        .long("test_only")
        .takes_value(false)
        .help("Focus analysis on #[test] methods.")
        .long_help("Only #[test] methods and their usage are analyzed. This must be used together with the rustc --test option."))
    .arg(Arg::with_name("diag")
        .long("diag")
        .possible_values(&["relaxed", "strict", "paranoid"])
        .default_value("relaxed")
        .help("Level of diagnostics which will be produced.")
        .long_help("With `relaxed`, false positives will be avoided where possible. With `strict`, all errors will be reported."))
}

/// Represents options passed to MIRAI.
#[derive(Debug, Default)]
pub struct Options {
    pub single_func: Option<String>,
    pub test_only: bool,
    pub diag_level: DiagLevel,
}

/// Represents diag level.
#[derive(Debug, PartialEq, PartialOrd)]
pub enum DiagLevel {
    /// When a function can't be fully analyzed, for example because it calls a function
    /// without a body and no foreign function summary, it is simply assumed to be correct (angelic).
    /// This is transitive: calling an angelic function causes the caller to become angelic itself.
    /// This minimizes false positives but can lead to a lot code not being analyzed
    /// while devirtualization isn't perfect and not all intrinsics have been modeled.
    RELAXED,
    /// When a function calls another function without a body or summary, it assumes that the called
    /// function is angelic and that it has no preconditions and no post condition. Analysis of the
    /// caller continues optimistically. This can lead to false positives because of the missing
    /// post conditions and imprecision can be amplified if the missing function occurs deep down
    /// in a long call chain.
    STRICT,
    /// When a function calls another function without a body or summary, a diagnostic is generated
    /// to flag the call as a problem. Analysis of the caller then proceeds as in the STRICT case.
    PARANOID,
}

impl Default for DiagLevel {
    fn default() -> Self {
        DiagLevel::RELAXED
    }
}

impl Options {
    /// Parse options from an argument string. The argument string will be split using unix
    /// shell escaping rules. Any content beyond the leftmost `--` token will be returned
    /// (excluding this token).
    pub fn parse_from_str(&mut self, s: &str) -> Vec<String> {
        self.parse(&shellwords::split(s).unwrap_or_else(|e| {
            early_error(
                ErrorOutputType::default(),
                &format!("Cannot parse argument string: {:?}", e),
            )
        }))
    }

    /// Parses options from a list of strings. Any content beyond the leftmost `--` token
    /// will be returned (excluding this token).
    pub fn parse(&mut self, args: &[String]) -> Vec<String> {
        let mut mirai_args_end = args.len();
        let mut other_args_start = args.len();
        if let Some((p, _)) = args.iter().find_position(|s| s.as_str() == "--") {
            mirai_args_end = p;
            other_args_start = p + 1;
        }
        let mirai_args = &args[0..mirai_args_end];
        let other_args = &args[other_args_start..args.len()];
        let matches = make_options_parser().get_matches_from(mirai_args.iter());
        self.single_func = matches.value_of("single_func").map(|s| s.to_string());
        self.test_only = matches.is_present("test_only");
        self.diag_level = match matches.value_of("diag").unwrap() {
            "relaxed" => DiagLevel::RELAXED,
            "strict" => DiagLevel::STRICT,
            "paranoid" => DiagLevel::PARANOID,
            _ => assume_unreachable!(),
        };
        other_args.to_vec()
    }
}