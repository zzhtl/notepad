use iced::widget::{Space, button, container, row, text, tooltip};
use iced::{Border, Element, Fill, Length, Theme};

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

/// 渲染编辑器工具栏
pub fn view<'a>(
    title: &'a str,
    dirty: bool,
    editing: bool,
    dark_theme: bool,
    font_size: u16,
) -> Element<'a, Message> {
    let title_text = text(title).size(15).style(title_style);

    let theme_icon = if dark_theme { "\u{2600}" } else { "\u{263D}" };

    if editing {
        // Markdown 快捷工具栏（一行）
        let md_row = row![
            md_btn(MdShortcut::H1),
            md_btn(MdShortcut::H2),
            md_btn(MdShortcut::H3),
            container(Space::new().width(Length::Fixed(6.0))),
            md_btn(MdShortcut::Bold),
            md_btn(MdShortcut::Italic),
            md_btn(MdShortcut::Strikethrough),
            container(Space::new().width(Length::Fixed(6.0))),
            md_btn(MdShortcut::BulletList),
            md_btn(MdShortcut::NumberedList),
            md_btn(MdShortcut::Checkbox),
            md_btn(MdShortcut::Quote),
            container(Space::new().width(Length::Fixed(6.0))),
            md_btn(MdShortcut::InlineCode),
            md_btn(MdShortcut::CodeBlock),
            md_btn(MdShortcut::Link),
            md_btn(MdShortcut::Table),
            md_btn(MdShortcut::HorizontalRule),
        ]
        .spacing(3)
        .align_y(iced::Center);

        let status: Element<'_, Message> = if dirty {
            text("\u{25CF} 未保存").size(11).style(dirty_style).into()
        } else {
            text("\u{2714} 已保存").size(11).style(saved_style).into()
        };

        let action_row = row![
            title_text,
            Space::new().width(Fill),
            status,
            Space::new().width(Length::Fixed(8.0)),
            tooltip(
                button(text("\u{21B6}").size(13))
                    .on_press(Message::Undo)
                    .padding([4, 8])
                    .style(button::secondary),
                "撤销 (Ctrl+Z)",
                tooltip::Position::Bottom,
            )
            .gap(4),
            tooltip(
                button(text("\u{21B7}").size(13))
                    .on_press(Message::Redo)
                    .padding([4, 8])
                    .style(button::secondary),
                "重做 (Ctrl+Shift+Z)",
                tooltip::Position::Bottom,
            )
            .gap(4),
            tooltip(
                button(text("A-").size(11))
                    .on_press(Message::ChangeFontSize(-1))
                    .padding([4, 8])
                    .style(button::secondary),
                "缩小字号",
                tooltip::Position::Bottom,
            )
            .gap(4),
            text(format!("{font_size}")).size(11),
            tooltip(
                button(text("A+").size(11))
                    .on_press(Message::ChangeFontSize(1))
                    .padding([4, 8])
                    .style(button::secondary),
                "增大字号",
                tooltip::Position::Bottom,
            )
            .gap(4),
            tooltip(
                button(text("\u{1F5BC}").size(12))
                    .on_press(Message::InsertImage)
                    .padding([4, 8])
                    .style(button::secondary),
                "插入图片到光标位置",
                tooltip::Position::Bottom,
            )
            .gap(4),
            tooltip(
                button(text("导出").size(11))
                    .on_press(Message::ExportNote)
                    .padding([4, 10])
                    .style(button::secondary),
                "导出为 .md",
                tooltip::Position::Bottom,
            )
            .gap(4),
            tooltip(
                button(text("保存").size(11))
                    .on_press(Message::SaveNote)
                    .padding([4, 10])
                    .style(button::primary),
                "保存 (Ctrl+S)",
                tooltip::Position::Bottom,
            )
            .gap(4),
            tooltip(
                button(text("预览").size(11))
                    .on_press(Message::ToggleEditMode)
                    .padding([4, 10])
                    .style(button::secondary),
                "切换查看模式 (Ctrl+E)",
                tooltip::Position::Bottom,
            )
            .gap(4),
            tooltip(
                button(text(theme_icon).size(12))
                    .on_press(Message::ToggleTheme)
                    .padding([4, 8])
                    .style(button::secondary),
                "切换主题",
                tooltip::Position::Bottom,
            )
            .gap(4),
        ]
        .spacing(4)
        .align_y(iced::Center);

        container(
            iced::widget::column![action_row, md_row,]
                .spacing(6)
                .padding([8, 12]),
        )
        .style(toolbar_style)
        .into()
    } else {
        let dirty_label: Element<'_, Message> = if dirty {
            text("\u{25CF} 未保存").size(11).style(dirty_style).into()
        } else {
            Space::new().width(Length::Fixed(0.0)).into()
        };

        container(
            row![
                title_text,
                Space::new().width(Length::Fixed(10.0)),
                dirty_label,
                Space::new().width(Fill),
                tooltip(
                    button(text("编辑").size(11))
                        .on_press(Message::ToggleEditMode)
                        .padding([4, 12])
                        .style(button::primary),
                    "Ctrl+E 或双击笔记",
                    tooltip::Position::Bottom,
                )
                .gap(4),
                tooltip(
                    button(text("导出").size(11))
                        .on_press(Message::ExportNote)
                        .padding([4, 12])
                        .style(button::secondary),
                    "导出为 .md 文件",
                    tooltip::Position::Bottom,
                )
                .gap(4),
                tooltip(
                    button(text(theme_icon).size(12))
                        .on_press(Message::ToggleTheme)
                        .padding([4, 8])
                        .style(button::secondary),
                    "切换主题",
                    tooltip::Position::Bottom,
                )
                .gap(4),
            ]
            .spacing(6)
            .padding([8, 12])
            .align_y(iced::Center),
        )
        .style(toolbar_style)
        .into()
    }
}
