//! Terminal styling helpers.
//!
//! Every helper routes through owo-colors' `if_supports_color`, so output is
//! plain text when stdout is redirected, when `NO_COLOR` is set, or when the
//! user passes `--color never`. A global override is installed from `main`.
//!
//! Styles are built as owned `Style` values and applied via `style()`; chaining
//! `OwoColorize` methods directly returns values borrowing temporaries, which
//! cannot be stashed inside `if_supports_color`. Helpers return `String`s.
use owo_colors::{OwoColorize, Stream, Style};

const OUT: Stream = Stream::Stdout;

fn paint(s: &str, style: Style) -> String {
    s.if_supports_color(OUT, |v| v.style(style)).to_string()
}

pub fn bold(s: &str) -> String {
    paint(s, Style::new().bold())
}

pub fn dim(s: &str) -> String {
    paint(s, Style::new().dimmed())
}

pub fn green(s: &str) -> String {
    paint(s, Style::new().green())
}

pub fn red(s: &str) -> String {
    paint(s, Style::new().red())
}

pub fn yellow(s: &str) -> String {
    paint(s, Style::new().yellow())
}

pub fn cyan(s: &str) -> String {
    paint(s, Style::new().cyan())
}

#[allow(dead_code)]
pub fn magenta(s: &str) -> String {
    paint(s, Style::new().magenta())
}

pub fn green_bold(s: &str) -> String {
    paint(s, Style::new().green().bold())
}

pub fn red_bold(s: &str) -> String {
    paint(s, Style::new().red().bold())
}

pub fn yellow_bold(s: &str) -> String {
    paint(s, Style::new().yellow().bold())
}

/// Bold red, derived from stderr's color support (for error messages).
pub fn error(s: &str) -> String {
    s.if_supports_color(Stream::Stderr, |v| v.style(Style::new().red().bold()))
        .to_string()
}
