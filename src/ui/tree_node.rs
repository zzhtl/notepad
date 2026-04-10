use iced::widget::{Space, button, column, container, mouse_area, row, text, text_input};
use iced::{Border, Element, Font, Length, Theme, font, mouse};

use crate::app::{PendingCreate, update::pending_input_id};
use crate::message::{ContextMenuTarget, Message};
use crate::model::folder::TreeNode;

fn tree_node_style(
    theme: &Theme,
    selected: bool,
    drop_target: bool,
    dragging: bool,
) -> container::Style {
    let palette = theme.extended_palette();

    let background = if drop_target {
        palette.primary.base.color
    } else if selected {
        palette.primary.weak.color
    } else if dragging {
        palette.background.strong.color
    } else {
        palette.background.base.color.scale_alpha(0.0)
    };

    let text_color = if drop_target {
        palette.primary.base.text
    } else if selected {
        palette.primary.weak.text
    } else {
        palette.background.base.text
    };

    container::Style {
        background: Some(background.into()),
        text_color: Some(text_color),
        border: Border {
            radius: 4.0.into(),
            ..Default::default()
        },
        ..container::Style::default()
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
    dragging_note_id: Option<&'a str>,
    drag_hover_folder_id: Option<&'a str>,
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
            let is_drop_target = drag_hover_folder_id == Some(folder.id.as_str());

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
                let note_count = children
                    .iter()
                    .filter(|c| matches!(c, TreeNode::Note { .. }))
                    .count();

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

                container(label_row)
                    .padding([5, 10])
                    .width(Length::Fill)
                    .style(move |theme| tree_node_style(theme, is_selected, is_drop_target, false))
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
            .interaction(mouse::Interaction::Pointer)
            .on_press(Message::ToggleFolder(folder.id.clone()))
            .on_right_press(Message::ShowContextMenu(ctx))
            .on_enter(Message::NoteDragEnteredFolder(folder.id.clone()))
            .on_exit(Message::NoteDragLeftFolder(folder.id.clone()))
            .on_release(Message::DropDraggedNoteOnFolder(folder.id.clone()));

            let mut items: Vec<Element<'a, Message>> = vec![label.into()];

            if *expanded {
                for child in children {
                    items.push(render_tree_node(
                        child,
                        selected_id,
                        depth + 1,
                        rename_state,
                        pending_create,
                        dragging_note_id,
                        drag_hover_folder_id,
                    ));
                }

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
            let is_dragging = dragging_note_id == Some(meta.id.as_str());

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
                container(row![text("\u{2022}").size(8), text(&meta.title).size(13)].spacing(6))
                    .padding([5, 10])
                    .width(Length::Fill)
                    .style(move |theme| tree_node_style(theme, is_selected, false, is_dragging))
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
            .interaction(if is_renaming {
                mouse::Interaction::Pointer
            } else {
                mouse::Interaction::Grab
            })
            .on_press(Message::StartNoteDrag(meta.id.clone()))
            .on_release(Message::FinishNoteDrag)
            .on_right_press(Message::ShowContextMenu(ctx))
            .into()
        }
    }
}
