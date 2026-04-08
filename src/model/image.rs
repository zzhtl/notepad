/// 图片附件
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ImageAttachment {
    pub id: String,
    pub note_id: String,
    pub filename: String,
    pub data: Vec<u8>,
    pub created_at: String,
}
