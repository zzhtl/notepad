use iced::widget::{button, container, row, text, text_input, tooltip};
use iced::{Border, Element, Fill, Theme};

use crate::message::{MdShortcut, Message};
use crate::ui::md_shortcut;

/// 工具栏容器样式
fn toolbar_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(palette.background.weak.color.into()),
        border: Border {
            radius: 0.0.into(),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

fn title_style(theme: &Theme) -> text::Style {
    let palette = theme.extended_palette();
    text::Style {
        color: Some(palette.primary.base.color),
    }
}

fn dirty_style(theme: &Theme) -> text::Style {
    let palette = theme.extended_palette();
    text::Style {
        color: Some(palette.warning.base.color),
    }
}

fn saved_style(theme: &Theme) -> text::Style {
    let palette = theme.extended_palette();
    text::Style {
        color: Some(palette.success.base.color),
    }
}

/// 包装一个 Markdown 快捷按钮
fn md_btn<'a>(kind: MdShortcut) -> Element<'a, Message> {
    tooltip(
        button(text(md_shortcut::label(kind)).size(12))
            .on_press(Message::InsertMdShortcut(kind))
            .padding([4, 8])
            .style(button::secondary),
        md_shortcut::tooltip(kind),
        tooltip::Position::Bottom,
    )
    .gap(4)
    .into()
}

/// 当前笔记搜索输入框 ID
pub fn note_search_input_id() -> iced::widget::Id {
    iced::widget::Id::new("note-search-input")
}

fn note_search_row<'a>(
    query: &'a str,
    current_match: usize,
    total_matches: usize,
) -> Element<'a, Message> {
    let previous = if total_matches > 0 {
        button(text("\u{2191}").size(12))
            .on_press(Message::NoteSearchPrevious)
            .padding([4, 8])
            .style(button::secondary)
    } else {
        button(text("\u{2191}").size(12))
            .padding([4, 8])
            .style(button::secondary)
    };

    let next = if total_matches > 0 {
        button(text("\u{2193}").size(12))
            .on_press(Message::NoteSearchNext)
            .padding([4, 8])
            .style(button::secondary)
    } else {
        button(text("\u{2193}").size(12))
            .padding([4, 8])
            .style(button::secondary)
    };

    let clear = if query.is_empty() {
        button(text("\u{2715}").size(11))
            .padding([4, 8])
            .style(button::secondary)
    } else {
        button(text("\u{2715}").size(11))
            .on_press(Message::ClearNoteSearch)
            .padding([4, 8])
            .style(button::secondary)
    };

    row![
        text("查找").size(11),
        text_input("搜索当前笔记... (Ctrl+F)", query)
            .id(note_search_input_id())
            .on_input(Message::NoteSearchQueryChanged)
            .on_submit(Message::NoteSearchNext)
            .size(12)
            .padding([5, 8])
            .width(Fill),
        text(format!("{current_match}/{total_matches}")).size(11),
        previous,
        next,
        clear,
    ]
    .spacing(6)
    .align_y(iced::Center)
    .into()
}

/// 渲染编辑器工具栏：标题独占一行，操作随窗口宽度换行。
pub fn view<'a>(
    title: &'a str,
    dirty: bool,
    editing: bool,
    font_size: u16,
    note_search_query: &'a str,
    current_match: usize,
    total_matches: usize,
) -> Element<'a, Message> {
    let status = if dirty {
        text("● 待保存").size(11).style(dirty_style)
    } else {
        text("✓ 已保存").size(11).style(saved_style)
    };
    let heading = row![
        container(text(title).size(18).style(title_style)).width(Fill).clip(true),
        status,
        text(if editing { "编辑模式" } else { "阅读模式" }).size(11),
    ].spacing(12).align_y(iced::Center);

    let mut actions = row![
        action(if editing { "阅读" } else { "编辑" }, Message::ToggleEditMode,
            "切换阅读 / 编辑 (Ctrl+E)", true),
        action("保存", Message::SaveNote, "保存当前笔记 (Ctrl+S)", false),
        action("导出当前", Message::ExportNote, "导出当前笔记为 .md (Ctrl+Shift+E)", false),
        action("A−", Message::ChangeFontSize(-1), "缩小字号 (Ctrl+-)", false),
        text(format!("{font_size} pt")).size(11),
        action("A+", Message::ChangeFontSize(1), "增大字号 (Ctrl+=)", false),
    ].spacing(6).align_y(iced::Center);
    if editing {
        actions = actions
            .push(action("撤销", Message::Undo, "撤销 (Ctrl+Z)", false))
            .push(action("重做", Message::Redo, "重做 (Ctrl+Shift+Z)", false))
            .push(action("插入图片", Message::InsertImage, "在光标处插入图片", false));
    }

    let mut content = iced::widget::column![
        heading,
        actions.wrap(),
        note_search_row(note_search_query, current_match, total_matches),
    ].spacing(10);
    if editing {
        let shortcuts = [
            MdShortcut::H1, MdShortcut::H2, MdShortcut::H3,
            MdShortcut::Bold, MdShortcut::Italic, MdShortcut::Strikethrough,
            MdShortcut::BulletList, MdShortcut::NumberedList, MdShortcut::Checkbox,
            MdShortcut::Quote, MdShortcut::InlineCode, MdShortcut::CodeBlock,
            MdShortcut::Link, MdShortcut::Table, MdShortcut::HorizontalRule,
        ];
        content = content.push(
            row(shortcuts.into_iter().map(md_btn)).spacing(4).wrap(),
        );
    }
    container(content.padding([12, 16])).style(toolbar_style).into()
}

fn action<'a>(label: &'static str, message: Message, hint: &'static str, primary: bool) -> Element<'a, Message> {
    tooltip(
        button(text(label).size(12))
            .on_press(message)
            .padding([6, 10])
            .style(if primary { button::primary } else { button::secondary }),
        hint,
        tooltip::Position::Bottom,
    ).gap(4).into()
}
