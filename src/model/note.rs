/// 笔记完整数据
#[derive(Debug, Clone)]
pub struct Note {
    pub id: String,
    pub folder_id: String,
    pub title: String,
    pub content: String,
    #[allow(dead_code)]
    pub sort_order: i32,
    #[allow(dead_code)]
    pub created_at: String,
    #[allow(dead_code)]
    pub updated_at: String,
}

/// 搜索结果
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub note_id: String,
    #[allow(dead_code)]
    pub folder_id: String,
    pub title: String,
    pub snippet: String,
    pub match_line: Option<usize>,
    pub match_column: Option<usize>,
    pub match_len: usize,
}
