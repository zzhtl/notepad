use iced::widget::{Space, button, container, row, text};
use iced::{Border, Color, Element, Fill, Theme};

use crate::message::Message;

fn banner_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(
            Color {
                a: 0.18,
                ..palette.danger.base.color
            }
            .into(),
        ),
        text_color: Some(palette.danger.base.color),
        border: Border {
            radius: 6.0.into(),
            width: 1.0,
            color: Color {
                a: 0.45,
                ..palette.danger.base.color
            },
        },
        ..container::Style::default()
    }
}

pub fn view(err: &str) -> Element<'_, Message> {
    container(
        row![
            text("\u{26A0}").size(13),
            text(format!("错误：{err}")).size(12),
            Space::new().width(Fill),
            button(text("关闭").size(11))
                .on_press(Message::DismissError)
                .padding([2, 8])
                .style(button::text),
        ]
        .spacing(8)
        .align_y(iced::Center)
        .padding([6, 12]),
    )
    .style(banner_style)
    .into()
}
