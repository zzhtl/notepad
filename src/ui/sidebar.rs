use iced::widget::{Space, button, column, container, row, rule, scrollable, text, text_input};
use iced::{Border, Element, Fill, Length, Theme};

use crate::app::{PendingCreate, update::pending_input_id};
use crate::message::Message;
use crate::model::folder::TreeNode;
use crate::model::note::SearchResult;
use crate::ui::tree_node::render_tree_node;

/// 侧边栏容器样式
fn sidebar_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(palette.background.weak.color.into()),
        ..container::Style::default()
    }
}

/// 头部 banner 样式
fn header_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(palette.background.weakest.color.into()),
        ..container::Style::default()
    }
}

fn brand_style(theme: &Theme) -> text::Style {
    let palette = theme.extended_palette();
    text::Style {
        color: Some(palette.primary.base.color),
    }
}

/// 搜索结果项样式
fn search_result_snippet(theme: &Theme) -> text::Style {
    let palette = theme.extended_palette();
    text::Style {
        color: Some(palette.background.base.text),
    }
}

fn search_result_meta(theme: &Theme) -> text::Style {
    let palette = theme.extended_palette();
    text::Style {
        color: Some(palette.primary.base.color),
    }
}

fn create_btn_style(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();
    let bg = match status {
        button::Status::Hovered => palette.primary.weak.color,
        _ => palette.background.strong.color,
    };
    button::Style {
        background: Some(bg.into()),
        text_color: palette.background.base.text,
        border: Border {
            radius: 6.0.into(),
            ..Border::default()
        },
        ..button::Style::default()
    }
}

/// 渲染左侧边栏
pub fn view<'a>(
    tree: &'a [TreeNode],
    selected_id: Option<&'a str>,
    rename_state: Option<(&'a str, &'a str)>,
    search_query: &'a str,
    search_active: bool,
    search_results: &'a [SearchResult],
    pending_create: Option<&'a PendingCreate>,
    dragging_note_id: Option<&'a str>,
    drag_hover_folder_id: Option<&'a str>,
) -> Element<'a, Message> {
    // 顶部品牌
    let header = container(
        row![
            text("\u{1F4D3}").size(18),
            text("Notepad").size(16).style(brand_style),
        ]
        .spacing(8)
        .align_y(iced::Center)
        .padding([10, 12]),
    )
    .style(header_style)
    .width(Fill);

    // 搜索框
    let search = container(
        text_input("\u{1F50D} 搜索笔记...", search_query)
            .id(iced::widget::Id::new("search-input"))
            .on_input(Message::SearchQueryChanged)
            .size(13)
            .padding([7, 10]),
    )
    .padding([8, 8]);

    let create_buttons = row![
        button(text("+ 文件夹").size(12))
            .on_press(Message::StartCreateFolder)
            .padding([6, 12])
            .style(create_btn_style),
        button(text("+ 笔记").size(12))
            .on_press(Message::StartCreateNote)
            .padding([6, 12])
            .style(create_btn_style),
    ]
    .spacing(6)
    .padding([2, 8]);

    // 搜索结果或树形视图
    let body: Element<'a, Message> = if search_active {
        if search_results.is_empty() {
            container(text("无搜索结果").size(13)).padding(12).into()
        } else {
            // 按 note_id 分组展示
            let mut items: Vec<Element<'a, Message>> = Vec::new();
            let mut current_note_id = "";

            for r in search_results {
                // 新笔记分组：展示标题
                if r.note_id != current_note_id {
                    current_note_id = &r.note_id;
                    if !items.is_empty() {
                        // 分组间距
                        items.push(Space::new().height(Length::Fixed(4.0)).into());
                    }
                    items.push(
                        container(text(&r.title).size(13).style(search_result_meta))
                            .padding([4, 10])
                            .width(Length::Fill)
                            .into(),
                    );
                }

                // 匹配行（可点击）
                let location = r
                    .match_line
                    .map(|line| format!("第 {} 行", line + 1))
                    .unwrap_or_else(|| "标题命中".to_string());

                items.push(
                    button(
                        column![
                            row![
                                Space::new().width(Length::Fixed(10.0)),
                                text(location).size(10).style(search_result_meta),
                            ],
                            container(row![
                                Space::new().width(Length::Fixed(10.0)),
                                text(&r.snippet).size(11).style(search_result_snippet),
                            ],)
                            .width(Length::Fixed(220.0))
                            .clip(true),
                        ]
                        .spacing(2),
                    )
                    .on_press(Message::OpenSearchResult(r.clone()))
                    .padding([4, 10])
                    .width(Length::Fill)
                    .style(button::text)
                    .into(),
                );
            }
            column(items).spacing(1).into()
        }
    } else {
        let tree_items: Vec<Element<'a, Message>> = tree
            .iter()
            .map(|node| {
                render_tree_node(
                    node,
                    selected_id,
                    0,
                    rename_state,
                    pending_create,
                    dragging_note_id,
                    drag_hover_folder_id,
                )
            })
            .collect();

        let mut tree_col: Vec<Element<'a, Message>> = Vec::new();

        if tree_items.is_empty() && pending_create.is_none() {
            tree_col.push(
                container(
                    text("暂无文件夹，点击上方按钮创建")
                        .size(12)
                        .style(search_result_snippet),
                )
                .padding(12)
                .into(),
            );
        } else {
            tree_col.extend(tree_items);
        }

        // 根级别的 pending create（无 parent_id）
        if let Some(pending) = pending_create
            && pending.parent_id.is_none()
        {
            let placeholder = if pending.is_folder {
                "文件夹名称（回车确认）"
            } else {
                "笔记名称（回车确认）"
            };
            tree_col.push(
                container(
                    row![
                        Space::new().width(Length::Fixed(8.0)),
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
                .padding([4, 6])
                .into(),
            );
        }

        column(tree_col).spacing(1).into()
    };

    container(
        column![
            header,
            rule::horizontal(1),
            search,
            create_buttons,
            rule::horizontal(1),
            scrollable(body).height(Fill),
        ]
        .spacing(0),
    )
    .width(Length::Fixed(250.0))
    .height(Fill)
    .style(sidebar_style)
    .into()
}
