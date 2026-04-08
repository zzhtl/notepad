/// 生成新的 UUID v4 字符串
pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}
