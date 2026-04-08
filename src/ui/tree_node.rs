use iced::widget::{Space, button, column, container, mouse_area, row, text, text_input};
use iced::{Border, Element, Font, Length, Theme, font};

use crate::app::{PendingCreate, update::pending_input_id};
use crate::message::{ContextMenuTarget, Message};
use crate::model::folder::TreeNode;

/// 选中状态的按钮样式
fn tree_node_selected(theme: &Theme, _status: button::Status) -> button::Style {
    let palette = theme.extended_palette();
    button::Style {
        background: Some(palette.primary.weak.color.into()),
        text_color: palette.primary.weak.text,
        border: Border {
            radius: 4.0.into(),
            ..Default::default()
        },
        ..button::Style::default()
    }
}

/// 默认状态的按钮样式（含 hover 效果）
fn tree_node_default(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();
    match status {
        button::Status::Hovered => button::Style {
            background: Some(palette.background.weak.color.into()),
            text_color: palette.background.base.text,
            border: Border {
                radius: 4.0.into(),
                ..Default::default()
            },
            ..button::Style::default()
        },
        _ => button::Style {
            background: None,
            text_color: palette.background.base.text,
            border: Border {
                radius: 4.0.into(),
                ..Default::default()
            },
            ..button::Style::default()
        },
    }
}

/// 笔记数量小标签样式
fn count_style(theme: &Theme) -> text::Style {
    let palette = theme.extended_palette();
    text::Style {
        color: Some(palette.background.weak.text),
    }
}

/// 渲染单个树节点（递归）
pub fn render_tree_node<'a>(
    node: &'a TreeNode,
    selected_id: Option<&'a str>,
    depth: usize,
    rename_state: Option<(&'a str, &'a str)>,
    pending_create: Option<&'a PendingCreate>,
) -> Element<'a, Message> {
    let indent = 8.0 + (depth as f32) * 20.0;
    let is_selected = selected_id == Some(node.id());
    let is_renaming = rename_state.is_some_and(|(id, _)| id == node.id());

    let bold_font = Font {
        weight: font::Weight::Bold,
        ..Font::DEFAULT
    };

    match node {
        TreeNode::Folder {
            folder,
            expanded,
            children,
            ..
        } => {
            let icon = if *expanded { "\u{25BE}" } else { "\u{25B8}" };

            // 计算笔记数量
            let note_count = children
                .iter()
                .filter(|c| matches!(c, TreeNode::Note { .. }))
                .count();

            let label_content: Element<'a, Message> = if is_renaming {
                let (_, current_input) = rename_state.unwrap();
                row![
                    text_input("名称", current_input)
                        .on_input(Message::RenameInputChanged)
                        .on_submit(Message::ConfirmRename)
                        .size(13)
                        .width(Length::Fill),
                    button(text("x").size(11))
                        .on_press(Message::CancelRename)
                        .padding([2, 6])
                        .style(button::text),
                ]
                .spacing(2)
                .into()
            } else {
                let mut label_row = row![
                    text(icon).size(12),
                    text(&folder.name).size(13).font(bold_font),
                ]
                .spacing(6);
                if note_count > 0 {
                    label_row = label_row.push(Space::new().width(Length::Fill));
                    label_row =
                        label_row.push(text(format!("{note_count}")).size(10).style(count_style));
                }
                button(label_row)
                    .on_press(Message::ToggleFolder(folder.id.clone()))
                    .padding([5, 10])
                    .width(Length::Fill)
                    .style(if is_selected {
                        tree_node_selected
                    } else {
                        tree_node_default
                    })
                    .into()
            };

            let ctx = ContextMenuTarget {
                node_id: folder.id.clone(),
                is_folder: true,
                parent_folder_id: folder.parent_id.clone(),
            };

            let label = mouse_area(row![
                Space::new().width(Length::Fixed(indent)),
                label_content
            ])
            .on_right_press(Message::ShowContextMenu(ctx));

            let mut items: Vec<Element<'a, Message>> = vec![label.into()];

            if *expanded {
                for child in children {
                    items.push(render_tree_node(
                        child,
                        selected_id,
                        depth + 1,
                        rename_state,
                        pending_create,
                    ));
                }

                // 在文件夹内显示 pending create 输入框
                if let Some(pending) = pending_create
                    && pending.parent_id.as_deref() == Some(&folder.id)
                {
                    let child_indent = 8.0 + ((depth + 1) as f32) * 20.0;
                    let placeholder = if pending.is_folder {
                        "文件夹名称（回车确认）"
                    } else {
                        "笔记名称（回车确认）"
                    };
                    items.push(
                        container(
                            row![
                                Space::new().width(Length::Fixed(child_indent)),
                                text_input(placeholder, &pending.input)
                                    .id(pending_input_id())
                                    .on_input(Message::PendingCreateInputChanged)
                                    .on_submit(Message::ConfirmCreate)
                                    .size(13)
                                    .padding([4, 8])
                                    .width(Length::Fill),
                                button(text("\u{2715}").size(11))
                                    .on_press(Message::CancelCreate)
                                    .padding([2, 6])
                                    .style(button::text),
                            ]
                            .spacing(2)
                            .align_y(iced::Center),
                        )
                        .padding([2, 0])
                        .into(),
                    );
                }
            }

            column(items).spacing(1).into()
        }
        TreeNode::Note { meta } => {
            let label_content: Element<'a, Message> = if is_renaming {
                let (_, current_input) = rename_state.unwrap();
                row![
                    text_input("名称", current_input)
                        .on_input(Message::RenameInputChanged)
                        .on_submit(Message::ConfirmRename)
                        .size(13)
                        .width(Length::Fill),
                    button(text("x").size(11))
                        .on_press(Message::CancelRename)
                        .padding([2, 6])
                        .style(button::text),
                ]
                .spacing(2)
                .into()
            } else {
                button(row![text("\u{2022}").size(8), text(&meta.title).size(13)].spacing(6))
                    .on_press(Message::SelectNote(meta.id.clone()))
                    .padding([5, 10])
                    .width(Length::Fill)
                    .style(if is_selected {
                        tree_node_selected
                    } else {
                        tree_node_default
                    })
                    .into()
            };

            let ctx = ContextMenuTarget {
                node_id: meta.id.clone(),
                is_folder: false,
                parent_folder_id: Some(meta.folder_id.clone()),
            };

            mouse_area(row![
                Space::new().width(Length::Fixed(indent)),
                label_content
            ])
            .on_right_press(Message::ShowContextMenu(ctx))
            .into()
        }
    }
}
