use crate::message::Message;
use iced::Task;

/// 打开文件对话框选择图片
pub fn pick_image() -> Task<Message> {
    Task::perform(
        async {
            let handle = rfd::AsyncFileDialog::new()
                .add_filter("图片", &["png", "jpg", "jpeg", "gif", "webp", "bmp"])
                .set_title("选择图片")
                .pick_file()
                .await;

            match handle {
                Some(file) => {
                    let filename = file.file_name();
                    let data = file.read().await;
                    Some((filename, data))
                }
                None => None,
            }
        },
        Message::ImagePicked,
    )
}
