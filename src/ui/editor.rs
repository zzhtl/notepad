use iced::widget::{container, markdown, row, rule, scrollable, text_editor};
use iced::{Element, Fill, Theme};

use crate::app::ActiveNote;
use crate::message::Message;
use crate::ui::md_viewer::NotepadViewer;
use crate::ui::search_highlight::{SearchHighlightSettings, SearchHighlighter, to_format};

const PREVIEW_MAX_WIDTH: f32 = 920.0;
pub const EDITOR_LINE_HEIGHT_FACTOR: f32 = 1.3;
const NOTE_TEXT_WRAPPING: iced::widget::text::Wrapping =
    iced::widget::text::Wrapping::WordOrGlyph;

/// 编辑器 ID（供 focus 操作使用）
pub fn editor_id() -> iced::widget::Id {
    iced::widget::Id::new("note-editor")
}

/// 编辑区 scrollable ID（供程序化滚动使用）
pub fn editor_scrollable_id() -> iced::widget::Id {
    iced::widget::Id::new("note-editor-scrollable")
}

fn note_scrollbar_direction() -> scrollable::Direction {
    scrollable::Direction::Vertical(
        scrollable::Scrollbar::default()
            .width(12)
            .scroller_width(12)
            .spacing(8),
    )
}

/// 渲染编辑模式：左右分屏
fn view_split<'a>(active: &'a ActiveNote, theme: &Theme, font_size: u16) -> Element<'a, Message> {
    let settings = SearchHighlightSettings::from_matches(
        &active.note_search_matches,
        active.note_search_index,
    );

    let editor = text_editor(&active.content)
        .id(editor_id())
        .on_action(Message::EditorAction)
        .size(font_size as f32)
        .line_height(EDITOR_LINE_HEIGHT_FACTOR)
        .wrapping(NOTE_TEXT_WRAPPING)
        .padding(12)
        .height(iced::Length::Shrink)
        .highlight_with::<SearchHighlighter>(settings, to_format);
    let editor = scrollable(editor)
        .id(editor_scrollable_id())
        .on_scroll(Message::EditorScrolled)
        .direction(note_scrollbar_direction())
        .height(Fill);

    let md_settings = markdown::Settings::from(theme);
    let viewer = NotepadViewer {
        images: &active.images,
    };
    let preview_content = container(markdown::view_with(
        &active.markdown_items,
        md_settings,
        &viewer,
    ))
    .max_width(PREVIEW_MAX_WIDTH)
    .center_x(Fill)
    .width(Fill);
    let preview = container(
        scrollable(preview_content)
            .id(iced::widget::Id::new("preview-scrollable"))
            .on_scroll(Message::PreviewScrolled)
            .direction(note_scrollbar_direction())
            .height(Fill),
    )
    .padding(16)
    .width(Fill)
    .height(Fill);

    row![
        container(editor).width(Fill).height(Fill),
        rule::vertical(1),
        container(preview).width(Fill).height(Fill),
    ]
    .height(Fill)
    .into()
}

/// 渲染查看模式：全屏只读编辑器 + 搜索高亮（支持鼠标选中 + Ctrl+C 复制）
fn view_readonly<'a>(
    active: &'a ActiveNote,
    _theme: &Theme,
    font_size: u16,
) -> Element<'a, Message> {
    let settings = SearchHighlightSettings::from_matches(
        &active.note_search_matches,
        active.note_search_index,
    );

    let editor = text_editor(&active.content)
        .id(editor_id())
        .on_action(Message::EditorAction)
        .size(font_size as f32)
        .line_height(EDITOR_LINE_HEIGHT_FACTOR)
        .wrapping(NOTE_TEXT_WRAPPING)
        .padding([20, 32])
        .height(iced::Length::Shrink)
        .highlight_with::<SearchHighlighter>(settings, to_format);
    let editor = scrollable(editor)
        .id(editor_scrollable_id())
        .on_scroll(Message::EditorScrolled)
        .direction(note_scrollbar_direction())
        .height(Fill);

    container(editor).width(Fill).height(Fill).into()
}

/// 根据编辑状态分发视图
pub fn view<'a>(active: &'a ActiveNote, theme: &Theme, font_size: u16) -> Element<'a, Message> {
    if active.editing {
        view_split(active, theme, font_size)
    } else {
        view_readonly(active, theme, font_size)
    }
}
