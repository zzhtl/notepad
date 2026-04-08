use std::collections::HashMap;

use iced::Element;
use iced::widget::{container, image, markdown, rich_text};

use crate::message::Message;

/// 自定义 Markdown Viewer，支持渲染本地图片
pub struct NotepadViewer<'a> {
    pub images: &'a HashMap<String, image::Handle>,
}

impl<'a> NotepadViewer<'a> {
    /// 从 URL 中提取图片 ID
    /// 兼容多种格式：notepad://image/{id}, notepad:image:{id}, attachment://{id}, img-{id}, 或纯 id
    fn resolve_image_id(&self, url: &str) -> Option<String> {
        // 尝试常见 scheme
        let candidates = [
            url.strip_prefix("notepad://image/"),
            url.strip_prefix("notepad:image:"),
            url.strip_prefix("attachment://"),
            url.strip_prefix("attachment:"),
            url.strip_prefix("img-"),
            url.strip_prefix("img:"),
        ];
        for c in candidates.iter().flatten() {
            let id = c.trim_end_matches('/').trim_start_matches('/');
            if self.images.contains_key(id) {
                return Some(id.to_string());
            }
        }
        // 直接以 url 当 ID 试一次
        if self.images.contains_key(url) {
            return Some(url.to_string());
        }
        // 取 URL 最后一段作为 ID
        if let Some(last) = url.rsplit('/').next()
            && self.images.contains_key(last)
        {
            return Some(last.to_string());
        }
        None
    }
}

impl<'a> markdown::Viewer<'a, Message> for NotepadViewer<'a> {
    fn on_link_click(url: markdown::Uri) -> Message {
        Message::MarkdownLinkClicked(url)
    }

    fn image(
        &self,
        settings: markdown::Settings,
        url: &'a markdown::Uri,
        _title: &'a str,
        alt: &markdown::Text,
    ) -> Element<'a, Message> {
        if let Some(id) = self.resolve_image_id(url.as_str())
            && let Some(handle) = self.images.get(&id)
        {
            return container(
                image(handle.clone())
                    .width(iced::Length::Fill)
                    .content_fit(iced::ContentFit::Contain),
            )
            .padding(settings.spacing.0)
            .into();
        }
        // 回退：显示 alt 文本
        container(rich_text(alt.spans(settings.style)).on_link_click(Self::on_link_click))
            .padding(settings.spacing.0)
            .into()
    }
}
