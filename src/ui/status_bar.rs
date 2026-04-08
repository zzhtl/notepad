use iced::widget::{Space, container, row, text};
use iced::{Element, Fill, Theme};

use crate::app::ActiveNote;
use crate::message::Message;

/// 状态栏容器样式
fn status_bar_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(palette.background.weak.color.into()),
        ..container::Style::default()
    }
}

fn dim(theme: &Theme) -> text::Style {
    let palette = theme.extended_palette();
    text::Style {
        color: Some(palette.background.weak.text),
    }
}

/// 中文/英文混合分词后估算阅读时长（按 300 字/分钟）
fn estimate_reading_minutes(content: &str) -> u32 {
    let chars = content.chars().filter(|c| !c.is_whitespace()).count();
    ((chars as f32 / 300.0).ceil() as u32).max(1)
}

/// 渲染底部状态栏（仅编辑模式）
pub fn view(active: &ActiveNote) -> Element<'_, Message> {
    let cursor = active.content.cursor();
    let line_count = active.content.line_count();
    let char_count = active.note.content.chars().count();
    let word_count = active
        .note
        .content
        .split_whitespace()
        .filter(|w| !w.is_empty())
        .count();
    let reading = estimate_reading_minutes(&active.note.content);
    let updated = active
        .note
        .updated_at
        .split('T')
        .next()
        .unwrap_or(&active.note.updated_at);

    container(
        row![
            text(format!(
                "行 {}, 列 {}",
                cursor.position.line + 1,
                cursor.position.column + 1
            ))
            .size(11)
            .style(dim),
            Space::new().width(Fill),
            text(format!("{char_count} 字符")).size(11).style(dim),
            text("·").size(11).style(dim),
            text(format!("{word_count} 词")).size(11).style(dim),
            text("·").size(11).style(dim),
            text(format!("{line_count} 行")).size(11).style(dim),
            text("·").size(11).style(dim),
            text(format!("约 {reading} 分钟")).size(11).style(dim),
            text("·").size(11).style(dim),
            text(format!("更新于 {updated}")).size(11).style(dim),
        ]
        .spacing(8)
        .padding([5, 14])
        .align_y(iced::Center),
    )
    .style(status_bar_style)
    .into()
}
