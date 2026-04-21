use std::collections::HashMap;
use std::ops::Range;

use iced::Color;

use crate::app::NoteSearchMatch;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HighlightSpan {
    pub start: usize,
    pub end: usize,
    pub kind: Highlight,
}

/// 搜索高亮设置
#[derive(Debug, Clone, PartialEq)]
pub struct SearchHighlightSettings {
    pub matches_by_line: HashMap<usize, Vec<HighlightSpan>>,
}

impl SearchHighlightSettings {
    pub fn from_matches(matches: &[NoteSearchMatch], current_index: Option<usize>) -> Self {
        let mut matches_by_line: HashMap<usize, Vec<HighlightSpan>> = HashMap::new();

        for (index, matched) in matches.iter().copied().enumerate() {
            matches_by_line
                .entry(matched.line)
                .or_default()
                .push(HighlightSpan {
                    start: matched.byte_column,
                    end: matched.byte_column + matched.byte_len,
                    kind: if Some(index) == current_index {
                        Highlight::CurrentMatch
                    } else {
                        Highlight::Match
                    },
                });
        }

        Self { matches_by_line }
    }
}

/// 搜索高亮器：在 text_editor 中高亮搜索匹配文本
pub struct SearchHighlighter {
    matches_by_line: HashMap<usize, Vec<HighlightSpan>>,
    current_line: usize,
}

/// 高亮标记
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Highlight {
    Match,
    CurrentMatch,
}

impl iced::widget::text::Highlighter for SearchHighlighter {
    type Settings = SearchHighlightSettings;
    type Highlight = Highlight;
    type Iterator<'a> = std::vec::IntoIter<(Range<usize>, Highlight)>;

    fn new(settings: &Self::Settings) -> Self {
        Self {
            matches_by_line: settings.matches_by_line.clone(),
            current_line: 0,
        }
    }

    fn update(&mut self, new_settings: &Self::Settings) {
        self.matches_by_line = new_settings.matches_by_line.clone();
        self.current_line = 0;
    }

    fn change_line(&mut self, line: usize) {
        self.current_line = line;
    }

    fn highlight_line(&mut self, _line: &str) -> Self::Iterator<'_> {
        let line = self.current_line;
        self.current_line = self.current_line.saturating_add(1);

        self.matches_by_line
            .get(&line)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(|span| (span.start..span.end, span.kind))
            .collect::<Vec<_>>()
            .into_iter()
    }

    fn current_line(&self) -> usize {
        self.current_line
    }
}

/// 高亮格式：普通命中和当前命中使用不同颜色
pub fn to_format(
    highlight: &Highlight,
    theme: &iced::Theme,
) -> iced_core::text::highlighter::Format<iced::Font> {
    let palette = theme.extended_palette();
    let color = match highlight {
        Highlight::Match => palette.primary.base.color,
        Highlight::CurrentMatch => {
            let warning = palette.warning.base.color;
            Color {
                r: warning.r.min(1.0),
                g: (warning.g + 0.1).min(1.0),
                b: warning.b.min(1.0),
                a: 1.0,
            }
        }
    };

    iced_core::text::highlighter::Format {
        color: Some(color),
        font: Some(iced::Font {
            weight: iced::font::Weight::Bold,
            ..iced::Font::default()
        }),
    }
}
