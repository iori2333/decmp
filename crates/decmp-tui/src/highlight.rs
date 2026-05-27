use std::sync::LazyLock;

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use syntect::easy::HighlightLines;
use syntect::highlighting::{FontStyle, ThemeSet};
use syntect::parsing::SyntaxSet;

static SYNTAX_SET: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_nonewlines);
static THEME_SET: LazyLock<ThemeSet> = LazyLock::new(ThemeSet::load_defaults);

fn syntect_style_to_ratatui(st: syntect::highlighting::Style) -> Style {
  let mut style = Style::default().fg(Color::Rgb(
    st.foreground.r,
    st.foreground.g,
    st.foreground.b,
  ));
  if st.font_style.contains(FontStyle::BOLD) {
    style = style.add_modifier(Modifier::BOLD);
  }
  if st.font_style.contains(FontStyle::ITALIC) {
    style = style.add_modifier(Modifier::ITALIC);
  }
  if st.font_style.contains(FontStyle::UNDERLINE) {
    style = style.add_modifier(Modifier::UNDERLINED);
  }
  style
}

fn find_syntax(filename: &str) -> &'static syntect::parsing::SyntaxReference {
  let ext = std::path::Path::new(filename)
    .extension()
    .and_then(|e| e.to_str())
    .unwrap_or("");
  SYNTAX_SET
    .find_syntax_by_extension(ext)
    .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text())
}

pub fn highlight_text(text: &str, filename: &str) -> Vec<Vec<Span<'static>>> {
  let syntax = find_syntax(filename);
  let theme = &THEME_SET.themes["base16-ocean.dark"];
  let mut highlighter = HighlightLines::new(syntax, theme);

  let mut result = Vec::new();
  for line in text.lines() {
    let spans = match highlighter.highlight_line(line, &SYNTAX_SET) {
      Ok(ranges) => ranges
        .iter()
        .map(|(style, s)| Span::styled((*s).to_string(), syntect_style_to_ratatui(*style)))
        .collect(),
      Err(_) => vec![Span::raw(line.to_string())],
    };
    result.push(spans);
  }
  result
}
