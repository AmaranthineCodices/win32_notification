# `win32_notification`

A simple wrapper around [`Shell_NotifyIcon`](https://docs.microsoft.com/en-us/windows/win32/api/shellapi/nf-shellapi-shell_notifyiconw). See [my blog post](https://lily.fyi/2020/01/03/rust-windows-notifications.html) for more information on how this works.

```rust
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
```