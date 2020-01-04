use std::cell::RefCell;
use std::ffi::OsString;
use std::fmt;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use winapi::shared::guiddef::GUID;
use winapi::shared::winerror;
use winapi::um::combaseapi::CoCreateGuid;
use winapi::um::errhandlingapi::GetLastError;
use winapi::um::shellapi::{self, Shell_NotifyIconW, NOTIFYICONDATAW};

const INFO_BUFFER_LEN: usize = 256;
const TITLE_BUFFER_LEN: usize = 64;

macro_rules! encode_string_wide {
    ($string:expr, $length:expr) => {{
        let codepoints = OsString::from($string)
            .as_os_str()
            .encode_wide()
            .take(64)
            .collect::<Vec<u16>>();
        let mut array = [0u16; $length];

        unsafe {
            std::ptr::copy_nonoverlapping(
                codepoints.as_ptr(),
                array.as_mut_ptr(),
                codepoints.len().min($length),
            );
        }

        array
    }};
}

fn generate_guid() -> Result<GUID, i32> {
    unsafe {
        let mut gen_guid: GUID = Default::default();
        let result = CoCreateGuid(&mut gen_guid);
        if result == winerror::S_OK {
            Ok(gen_guid)
        } else {
            Err(result)
        }
    }
}

/// Represents a Win32 notification that can be shown or hidden. Internally this is a thin wrapper around an underlying win32::NOTIFYICONDATAW.
/// Use a [NotificationBuilder](NotificationBuilder) to create a new Notification.
/// ```rust
/// let notification = NotificationBuilder::new()
///     .title_text("Notification Title")
///     .info_text("This is the notification body")
///     .build();
///
/// notification.show();
/// ```
// Win32 APIs require that we pass a mutable pointer, but the user doesn't
// need to care about this, so we use a RefCell to avoid exposing details.
pub struct Notification(RefCell<NOTIFYICONDATAW>);

impl fmt::Debug for Notification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let descriptor = self.0.borrow();

        f.debug_struct("Notification")
            .field("uFlags", &descriptor.uFlags)
            .field(
                "szInfo",
                &OsString::from_wide(&descriptor.szInfo[..])
                    .into_string()
                    // Notifications are created from NotificationBuilders, which require Strings
                    // when specifying title/info text. Therefore, this will always succeed, since it's
                    // functionally a round-trip conversion from String -> [u16] -> OsString -> String.
                    .unwrap()
                    .trim()
                    // Trim the terminating null characters since they're not relevant.
                    .trim_end_matches('\u{0}'),
            )
            .field(
                "szInfoTitle",
                &OsString::from_wide(&descriptor.szInfoTitle[..])
                    .into_string()
                    .unwrap()
                    .trim()
                    .trim_end_matches('\u{0}'),
            )
            .field("guidItem", &descriptor.guidItem)
            .finish()
    }
}

impl Notification {
    /// Adds the notification to Windows, sending it to the user and making it visible.
    /// This will fail if called multiple times.
    pub fn show(&self) -> Result<(), i32> {
        let mut descriptor = self.0.borrow_mut();
        let success = unsafe { Shell_NotifyIconW(shellapi::NIM_ADD, &mut *descriptor) != 0 };
        if !success {
            let last_err = unsafe { GetLastError() };
            return Err(last_err as i32);
        }

        Ok(())
    }

    /// Consumes and deletes the notification.
    pub fn delete(self) -> Result<(), i32> {
        let mut descriptor = self.0.borrow_mut();
        let success = unsafe { Shell_NotifyIconW(shellapi::NIM_DELETE, &mut *descriptor) != 0 };
        if !success {
            let last_err = unsafe { GetLastError() };
            return Err(last_err as i32);
        }

        Ok(())
    }
}

/// A builder for [Notifications](Notification).
/// ```rust
/// let notification = NotificationBuilder::new()
///     .title_text("Notification Title")
///     .info_text("This is the notification body")
///     .build();
/// ```
pub struct NotificationBuilder(NOTIFYICONDATAW);

#[derive(Debug)]
pub enum NotificationBuilderError {
    /// An error was returned by a Windows API call. The error code is included.
    WindowsError(i32),
}

impl NotificationBuilder {
    /// Creates a notification builder with a new GUID.
    pub fn new() -> Result<NotificationBuilder, NotificationBuilderError> {
        let notification_descriptor = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            uFlags: shellapi::NIF_INFO | shellapi::NIF_GUID,
            guidItem: generate_guid().map_err(|e| NotificationBuilderError::WindowsError(e))?,
            ..Default::default()
        };

        Ok(NotificationBuilder(notification_descriptor))
    }

    /// Sets the text that will be the notification's body.
    /// This data is copied internally into the notification.
    /// If the length exceeds 256 characters, it will be silently truncated.
    pub fn info_text(mut self, text: &str) -> Self {
        let buffer = encode_string_wide!(text, INFO_BUFFER_LEN);
        self.0.szInfo = buffer;

        self
    }

    /// Sets the text that will be the notification's title. If not specified, the notification will not display a title.
    /// This data is copied internally into the notification.
    /// If the length exceeds 64 characters, it will be silently truncated.
    pub fn title_text(mut self, text: &str) -> Self {
        let buffer = encode_string_wide!(text, TITLE_BUFFER_LEN);
        self.0.szInfoTitle = buffer;

        self
    }

    /// Consumes the builder and creates a Notification.
    pub fn build(self) -> Notification {
        Notification(RefCell::new(self.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_string_wide() {
        let encoded = encode_string_wide!("Test", 12);
        assert_eq!(
            encoded.len(),
            12,
            "should have buffer length always even when source input is shorter (is {}, {})",
            encoded.len(),
            12
        );
        assert_eq!(encoded, [84, 101, 115, 116, 0, 0, 0, 0, 0, 0, 0, 0]);

        let encoded = encode_string_wide!("testabcabcabc", 12);
        assert_eq!(
            encoded.len(),
            12,
            "should truncate input data if it is longer than the requested buffer length (is {}, {})",
            encoded.len(),
            12
        );
        assert_eq!(
            encoded,
            [116, 101, 115, 116, 97, 98, 99, 97, 98, 99, 97, 98]
        );
    }

    #[test]
    fn test_notification_builder() {
        // Technically CoCreateGuid can fail and cause this test to fail.
        // I don't expect this to happen.
        let mut builder = NotificationBuilder::new().expect("new call should succeed");
        builder = builder.info_text("test");
        let mut expected_info_text = [0u16; INFO_BUFFER_LEN];
        expected_info_text[0] = 116;
        expected_info_text[1] = 101;
        expected_info_text[2] = 115;
        expected_info_text[3] = 116;
        assert_eq!(
            builder.0.szInfo.iter().collect::<Vec<&u16>>(),
            expected_info_text.iter().collect::<Vec<&u16>>()
        );

        builder = builder.title_text("Test title");
        let mut expected_title_text = [0u16; TITLE_BUFFER_LEN];
        expected_title_text[0] = 0x54;
        expected_title_text[1] = 0x65;
        expected_title_text[2] = 0x73;
        expected_title_text[3] = 0x74;
        expected_title_text[4] = 0x20;
        expected_title_text[5] = 0x74;
        expected_title_text[6] = 0x69;
        expected_title_text[7] = 0x74;
        expected_title_text[8] = 0x6C;
        expected_title_text[9] = 0x65;
        assert_eq!(
            builder.0.szInfoTitle.iter().collect::<Vec<&u16>>(),
            expected_title_text.iter().collect::<Vec<&u16>>()
        );

        let notification = builder.build();
        assert_eq!(
            notification.0.borrow().szInfo.iter().collect::<Vec<&u16>>(),
            expected_info_text.iter().collect::<Vec<&u16>>()
        );
        assert_eq!(
            notification
                .0
                .borrow()
                .szInfoTitle
                .iter()
                .collect::<Vec<&u16>>(),
            expected_title_text.iter().collect::<Vec<&u16>>()
        );
    }
}
