use iced::Color;
use std::ops::Range;

/// 搜索高亮设置
#[derive(Debug, Clone, PartialEq)]
pub struct SearchHighlightSettings {
    pub query: Option<String>,
}

/// 搜索高亮器：在 text_editor 中高亮搜索匹配文本
pub struct SearchHighlighter {
    query_lower: Option<String>,
    current_line: usize,
}

/// 高亮标记
#[derive(Debug, Clone, Copy)]
pub enum Highlight {
    Match,
}

impl iced::widget::text::Highlighter for SearchHighlighter {
    type Settings = SearchHighlightSettings;
    type Highlight = Highlight;
    type Iterator<'a> = HighlightIter<'a>;

    fn new(settings: &Self::Settings) -> Self {
        Self {
            query_lower: settings
                .query
                .as_ref()
                .filter(|q| !q.is_empty())
                .map(|q| q.to_lowercase()),
            current_line: 0,
        }
    }

    fn update(&mut self, new_settings: &Self::Settings) {
        self.query_lower = new_settings
            .query
            .as_ref()
            .filter(|q| !q.is_empty())
            .map(|q| q.to_lowercase());
        self.current_line = 0;
    }

    fn change_line(&mut self, line: usize) {
        self.current_line = self.current_line.min(line);
    }

    fn highlight_line(&mut self, line: &str) -> Self::Iterator<'_> {
        self.current_line += 1;
        HighlightIter {
            line_lower: line.to_lowercase(),
            query_lower: self.query_lower.as_deref(),
            pos: 0,
        }
    }

    fn current_line(&self) -> usize {
        self.current_line
    }
}

pub struct HighlightIter<'a> {
    line_lower: String,
    query_lower: Option<&'a str>,
    pos: usize,
}

impl Iterator for HighlightIter<'_> {
    type Item = (Range<usize>, Highlight);

    fn next(&mut self) -> Option<Self::Item> {
        let query = self.query_lower?;

        if let Some(found) = self.line_lower[self.pos..].find(query) {
            let start = self.pos + found;
            let end = start + query.len();
            self.pos = end;
            Some((start..end, Highlight::Match))
        } else {
            self.pos = self.line_lower.len();
            None
        }
    }
}

/// 高亮格式：加粗 + 亮色
pub fn to_format(
    _highlight: &Highlight,
    theme: &iced::Theme,
) -> iced_core::text::highlighter::Format<iced::Font> {
    let palette = theme.extended_palette();
    let base = palette.primary.base.color;
    let color = Color {
        r: base.r.min(1.0),
        g: (base.g + 0.3).min(1.0),
        b: base.b.min(1.0),
        a: 1.0,
    };
    iced_core::text::highlighter::Format {
        color: Some(color),
        font: Some(iced::Font {
            weight: iced::font::Weight::Bold,
            ..iced::Font::default()
        }),
    }
}
