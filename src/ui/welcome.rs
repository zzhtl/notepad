use iced::widget::{Container, column, container, text};
use iced::{Border, Color, Fill, Theme};

use crate::message::Message;

/// 欢迎页样式
fn welcome_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(palette.background.base.color.into()),
        ..container::Style::default()
    }
}

fn card_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(palette.background.weak.color.into()),
        border: Border {
            radius: 12.0.into(),
            width: 1.0,
            color: Color {
                a: 0.15,
                ..palette.primary.base.color
            },
        },
        ..container::Style::default()
    }
}

fn hint_style(theme: &Theme) -> text::Style {
    let palette = theme.extended_palette();
    text::Style {
        color: Some(palette.background.weak.text),
    }
}

/// 欢迎页（无激活笔记时显示）
pub fn view<'a>(_dark: bool) -> Container<'a, Message> {
    let card = container(
        column![
            text("\u{1F4DD} Notepad").size(36),
            text("高性能 Markdown 记事本").size(14).style(hint_style),
            text("").size(8),
            text("快捷键").size(13),
            text("Ctrl + N    新建笔记").size(12).style(hint_style),
            text("Ctrl + F    搜索笔记").size(12).style(hint_style),
            text("Ctrl + S    保存当前笔记").size(12).style(hint_style),
            text("Ctrl + E    切换查看 / 编辑")
                .size(12)
                .style(hint_style),
            text("Ctrl + Z    撤销 (Ctrl+Shift+Z 重做)")
                .size(12)
                .style(hint_style),
            text("").size(8),
            text("操作提示").size(13),
            text("• 单击笔记 = 查看  |  双击 / 右键 = 编辑")
                .size(12)
                .style(hint_style),
            text("• 文件夹右键可新建子文件夹 / 笔记")
                .size(12)
                .style(hint_style),
            text("• 编辑器支持插入图片、Markdown 快捷按钮")
                .size(12)
                .style(hint_style),
        ]
        .spacing(6)
        .align_x(iced::Center),
    )
    .padding(32)
    .style(card_style);

    container(card)
        .center(Fill)
        .width(Fill)
        .height(Fill)
        .style(welcome_style)
}
