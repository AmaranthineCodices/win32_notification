use std::thread;
use std::time::Duration;
use win32_notification::NotificationBuilder;

fn main() {
    let notification = NotificationBuilder::new()
        .title_text("Notification Title")
        .info_text("This is the notification body")
        .build()
        .expect("Could not create notification");

    notification.show().expect("Failed to show notification");
    thread::sleep(Duration::from_secs(5));
    notification
        .delete()
        .expect("Failed to delete notification");
}
