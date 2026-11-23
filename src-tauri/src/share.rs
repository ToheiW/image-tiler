use std::path::Path;

use objc2::AllocAnyThread;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_app_kit::{NSSharingServicePicker, NSWindow};
use objc2_foundation::{NSArray, NSPoint, NSRect, NSRectEdge, NSSize, NSString, NSURL};

/// Presents the macOS share sheet for `path`, anchored to the window centre.
///
/// # Safety
///
/// Must be called on the main thread with `ns_window` pointing at a live
/// `NSWindow` (Tauri's `Window::ns_window`).
pub unsafe fn present(ns_window: usize, path: &Path) -> Result<(), String> {
    let window = unsafe { &*(ns_window as *const NSWindow) };
    let view = window
        .contentView()
        .ok_or_else(|| "window has no content view".to_string())?;

    let file_url = NSURL::fileURLWithPath(&NSString::from_str(&path.to_string_lossy()));
    let item = unsafe { Retained::cast::<AnyObject>(file_url) };
    let items = NSArray::from_retained_slice(&[item]);

    let picker =
        unsafe { NSSharingServicePicker::initWithItems(NSSharingServicePicker::alloc(), &items) };

    let bounds = view.bounds();
    let anchor = NSRect::new(
        NSPoint::new(
            bounds.origin.x + bounds.size.width / 2.0,
            bounds.origin.y + bounds.size.height / 2.0,
        ),
        NSSize::new(1.0, 1.0),
    );

    picker.showRelativeToRect_ofView_preferredEdge(anchor, &view, NSRectEdge::MinY);
    Ok(())
}
