use crate::message::MdShortcut;

/// 返回 markdown 快捷键的插入文本
pub fn snippet(kind: MdShortcut) -> &'static str {
    match kind {
        MdShortcut::H1 => "# ",
        MdShortcut::H2 => "## ",
        MdShortcut::H3 => "### ",
        MdShortcut::Bold => "**粗体**",
        MdShortcut::Italic => "*斜体*",
        MdShortcut::Strikethrough => "~~删除线~~",
        MdShortcut::InlineCode => "`code`",
        MdShortcut::CodeBlock => "\n```\ncode\n```\n",
        MdShortcut::BulletList => "- ",
        MdShortcut::NumberedList => "1. ",
        MdShortcut::Checkbox => "- [ ] ",
        MdShortcut::Quote => "> ",
        MdShortcut::HorizontalRule => "\n---\n",
        MdShortcut::Link => "[链接文本](https://)",
        MdShortcut::Table => "\n| 列1 | 列2 |\n| --- | --- |\n| 内容 | 内容 |\n",
    }
}

/// 按钮显示文本
pub fn label(kind: MdShortcut) -> &'static str {
    match kind {
        MdShortcut::H1 => "H1",
        MdShortcut::H2 => "H2",
        MdShortcut::H3 => "H3",
        MdShortcut::Bold => "B",
        MdShortcut::Italic => "I",
        MdShortcut::Strikethrough => "S",
        MdShortcut::InlineCode => "</>",
        MdShortcut::CodeBlock => "{ }",
        MdShortcut::BulletList => "•",
        MdShortcut::NumberedList => "1.",
        MdShortcut::Checkbox => "\u{2611}",
        MdShortcut::Quote => "\u{201C}\u{201D}",
        MdShortcut::HorizontalRule => "—",
        MdShortcut::Link => "\u{1F517}",
        MdShortcut::Table => "\u{229E}",
    }
}

/// 按钮 tooltip
pub fn tooltip(kind: MdShortcut) -> &'static str {
    match kind {
        MdShortcut::H1 => "一级标题",
        MdShortcut::H2 => "二级标题",
        MdShortcut::H3 => "三级标题",
        MdShortcut::Bold => "粗体",
        MdShortcut::Italic => "斜体",
        MdShortcut::Strikethrough => "删除线",
        MdShortcut::InlineCode => "行内代码",
        MdShortcut::CodeBlock => "代码块",
        MdShortcut::BulletList => "无序列表",
        MdShortcut::NumberedList => "有序列表",
        MdShortcut::Checkbox => "任务清单",
        MdShortcut::Quote => "引用",
        MdShortcut::HorizontalRule => "分隔线",
        MdShortcut::Link => "插入链接",
        MdShortcut::Table => "插入表格",
    }
}
