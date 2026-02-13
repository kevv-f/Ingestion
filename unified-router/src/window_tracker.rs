//! Window tracking across all displays.
//!
//! This module provides functionality to enumerate and track all visible windows
//! across all connected displays on macOS.

use crate::types::{DisplayId, DisplayInfo, WindowBounds, WindowId, WindowInfo};
use std::collections::HashMap;
use tracing::{debug, trace};

#[cfg(target_os = "macos")]
mod macos {
    use super::*;
    use core_foundation::array::CFArray;
    use core_foundation::base::{CFType, TCFType};
    use core_foundation::dictionary::CFDictionary;
    use core_foundation::number::CFNumber;
    use core_foundation::string::CFString;
    use core_graphics::display::{
        CGDirectDisplayID, CGDisplayBounds, CGGetActiveDisplayList, CGMainDisplayID,
    };
    use core_graphics::window::{
        kCGNullWindowID, kCGWindowListExcludeDesktopElements, kCGWindowListOptionOnScreenOnly,
        CGWindowListCopyWindowInfo,
    };

    /// Get all active displays
    pub fn get_displays() -> Vec<DisplayInfo> {
        let mut display_count: u32 = 0;

        // Get count first
        unsafe {
            CGGetActiveDisplayList(0, std::ptr::null_mut(), &mut display_count);
        }

        if display_count == 0 {
            return vec![];
        }

        let mut displays = vec![0u32; display_count as usize];

        unsafe {
            CGGetActiveDisplayList(display_count, displays.as_mut_ptr(), &mut display_count);
        }

        let main_display = unsafe { CGMainDisplayID() };

        displays
            .into_iter()
            .map(|id| {
                let bounds = unsafe { CGDisplayBounds(id) };
                DisplayInfo {
                    id,
                    bounds: WindowBounds::new(
                        bounds.origin.x as i32,
                        bounds.origin.y as i32,
                        bounds.size.width as u32,
                        bounds.size.height as u32,
                    ),
                    is_main: id == main_display,
                    is_builtin: is_builtin_display(id),
                }
            })
            .collect()
    }

    fn is_builtin_display(display_id: CGDirectDisplayID) -> bool {
        // CGDisplayIsBuiltin is available but requires linking
        // For now, assume the main display on laptops is built-in
        unsafe { CGMainDisplayID() == display_id }
    }

    /// Get all visible windows
    pub fn get_windows() -> Vec<WindowInfo> {
        let options = kCGWindowListOptionOnScreenOnly | kCGWindowListExcludeDesktopElements;

        let window_list: CFArray<CFDictionary<CFString, CFType>> = unsafe {
            let list_ref = CGWindowListCopyWindowInfo(options, kCGNullWindowID);
            if list_ref.is_null() {
                return vec![];
            }
            CFArray::wrap_under_create_rule(list_ref)
        };

        let displays = get_displays();
        let mut windows = Vec::new();

        for i in 0..window_list.len() {
            if let Some(dict) = window_list.get(i) {
                if let Some(window) = parse_window_dict(&dict, &displays) {
                    windows.push(window);
                }
            }
        }

        windows
    }

    fn parse_window_dict(
        dict: &CFDictionary<CFString, CFType>,
        displays: &[DisplayInfo],
    ) -> Option<WindowInfo> {
        // Get window ID
        let window_id = get_dict_number(dict, "kCGWindowNumber")? as u64;

        // Get owner PID
        let pid = get_dict_number(dict, "kCGWindowOwnerPID")? as u32;

        // Get window layer (skip non-normal windows)
        let layer = get_dict_number(dict, "kCGWindowLayer").unwrap_or(0);
        if layer != 0 {
            return None; // Skip menu bars, docks, etc.
        }

        // Get window bounds
        let bounds = get_window_bounds(dict)?;

        // Skip tiny windows
        if bounds.width < 100 || bounds.height < 100 {
            return None;
        }

        // Get window title
        let title = get_dict_string(dict, "kCGWindowName").unwrap_or_default();

        // Get owner name (app name)
        let app_name = get_dict_string(dict, "kCGWindowOwnerName").unwrap_or_default();

        // Get bundle ID from PID
        let bundle_id = get_bundle_id_for_pid(pid).unwrap_or_else(|| app_name.clone());

        // Check if window is actually on screen (visible on current Space)
        // kCGWindowIsOnscreen is 1 if the window is on the current Space
        let is_on_screen = get_dict_bool(dict, "kCGWindowIsOnscreen").unwrap_or(false);

        // Determine which display this window is on
        let (center_x, center_y) = bounds.center();
        let display_id = displays
            .iter()
            .find(|d| d.bounds.contains(center_x, center_y))
            .map(|d| d.id)
            .unwrap_or(0);

        Some(WindowInfo {
            id: window_id,
            display_id,
            title,
            bundle_id,
            app_name,
            bounds,
            pid,
            is_on_screen,
        })
    }

    fn get_dict_number(dict: &CFDictionary<CFString, CFType>, key: &str) -> Option<i64> {
        let cf_key = CFString::new(key);
        dict.find(&cf_key).and_then(|value| {
            // Try to downcast to CFNumber
            if value.type_of() == CFNumber::type_id() {
                let num: CFNumber = unsafe {
                    CFNumber::wrap_under_get_rule(value.as_CFTypeRef() as *const _)
                };
                num.to_i64()
            } else {
                None
            }
        })
    }

    fn get_dict_bool(dict: &CFDictionary<CFString, CFType>, key: &str) -> Option<bool> {
        let cf_key = CFString::new(key);
        dict.find(&cf_key).and_then(|value| {
            // Try CFNumber first (kCGWindowIsOnscreen is stored as CFNumber)
            if value.type_of() == CFNumber::type_id() {
                let num: CFNumber = unsafe {
                    CFNumber::wrap_under_get_rule(value.as_CFTypeRef() as *const _)
                };
                return num.to_i32().map(|n| n != 0);
            }
            // Try CFBoolean
            use core_foundation::boolean::CFBoolean;
            if value.type_of() == CFBoolean::type_id() {
                let b: CFBoolean = unsafe {
                    CFBoolean::wrap_under_get_rule(value.as_CFTypeRef() as *const _)
                };
                return Some(b.into());
            }
            None
        })
    }

    fn get_dict_string(dict: &CFDictionary<CFString, CFType>, key: &str) -> Option<String> {
        let cf_key = CFString::new(key);
        dict.find(&cf_key).and_then(|value| {
            if value.type_of() == CFString::type_id() {
                let s: CFString = unsafe {
                    CFString::wrap_under_get_rule(value.as_CFTypeRef() as *const _)
                };
                Some(s.to_string())
            } else {
                None
            }
        })
    }

    fn get_window_bounds(dict: &CFDictionary<CFString, CFType>) -> Option<WindowBounds> {
        let cf_key = CFString::new("kCGWindowBounds");
        let bounds_dict = dict.find(&cf_key)?;

        // Bounds is a CFDictionary with X, Y, Width, Height
        if bounds_dict.type_of() != CFDictionary::<CFString, CFType>::type_id() {
            return None;
        }

        let bounds: CFDictionary<CFString, CFType> = unsafe {
            CFDictionary::wrap_under_get_rule(bounds_dict.as_CFTypeRef() as *const _)
        };

        let x = get_dict_number_f64(&bounds, "X")? as i32;
        let y = get_dict_number_f64(&bounds, "Y")? as i32;
        let width = get_dict_number_f64(&bounds, "Width")? as u32;
        let height = get_dict_number_f64(&bounds, "Height")? as u32;

        Some(WindowBounds::new(x, y, width, height))
    }

    fn get_dict_number_f64(dict: &CFDictionary<CFString, CFType>, key: &str) -> Option<f64> {
        let cf_key = CFString::new(key);
        dict.find(&cf_key).and_then(|value| {
            if value.type_of() == CFNumber::type_id() {
                let num: CFNumber = unsafe {
                    CFNumber::wrap_under_get_rule(value.as_CFTypeRef() as *const _)
                };
                num.to_f64()
            } else {
                None
            }
        })
    }

    fn get_bundle_id_for_pid(pid: u32) -> Option<String> {
        use std::process::Command;

        // Use osascript to get bundle ID - most reliable method
        let output = Command::new("osascript")
            .args([
                "-e",
                &format!(
                    "tell application \"System Events\" to get bundle identifier of (first process whose unix id is {})",
                    pid
                ),
            ])
            .output()
            .ok()?;

        if output.status.success() {
            let bundle_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !bundle_id.is_empty() && bundle_id != "missing value" {
                return Some(bundle_id);
            }
        }

        None
    }

    /// Get the window ID of the frontmost (focused) window
    pub fn get_frontmost_window_id() -> Option<u64> {
        // Use CGWindowListCopyWindowInfo to find the frontmost window
        // The window list is ordered front-to-back, so the first normal window is frontmost
        let options = kCGWindowListOptionOnScreenOnly | kCGWindowListExcludeDesktopElements;
        
        let window_list: CFArray<CFDictionary<CFString, CFType>> = unsafe {
            let list_ref = CGWindowListCopyWindowInfo(options, kCGNullWindowID);
            if list_ref.is_null() {
                return None;
            }
            CFArray::wrap_under_create_rule(list_ref)
        };

        // The window list is ordered front-to-back, so the first normal window is frontmost
        for i in 0..window_list.len() {
            if let Some(dict) = window_list.get(i) {
                let layer = get_dict_number(&dict, "kCGWindowLayer").unwrap_or(-1);
                if layer == 0 {
                    // This is a normal window (not menu bar, dock, etc.)
                    // Also check it's on screen
                    let is_on_screen = get_dict_bool(&dict, "kCGWindowIsOnscreen").unwrap_or(false);
                    if is_on_screen {
                        if let Some(window_id) = get_dict_number(&dict, "kCGWindowNumber") {
                            // Skip tiny windows (likely tooltips or popups)
                            if let Some(bounds) = get_window_bounds(&dict) {
                                if bounds.width >= 100 && bounds.height >= 100 {
                                    return Some(window_id as u64);
                                }
                            }
                        }
                    }
                }
            }
        }

        None
    }
}

#[cfg(not(target_os = "macos"))]
mod macos {
    use super::*;

    pub fn get_displays() -> Vec<DisplayInfo> {
        vec![]
    }

    pub fn get_windows() -> Vec<WindowInfo> {
        vec![]
    }
}

/// Window tracker that maintains state across all displays
pub struct WindowTracker {
    /// All known displays
    displays: Vec<DisplayInfo>,
    /// All tracked windows by ID
    windows: HashMap<WindowId, WindowInfo>,
    /// Active window per display
    active_per_display: HashMap<DisplayId, WindowId>,
    /// Previous window titles (for change detection)
    previous_titles: HashMap<WindowId, String>,
}

impl WindowTracker {
    pub fn new() -> Self {
        Self {
            displays: Vec::new(),
            windows: HashMap::new(),
            active_per_display: HashMap::new(),
            previous_titles: HashMap::new(),
        }
    }

    /// Refresh the list of displays
    pub fn refresh_displays(&mut self) -> &[DisplayInfo] {
        self.displays = macos::get_displays();
        debug!("Found {} displays", self.displays.len());
        &self.displays
    }

    /// Get all displays
    pub fn displays(&self) -> &[DisplayInfo] {
        &self.displays
    }

    /// Refresh the list of windows and detect changes
    pub fn refresh_windows(&mut self) -> WindowChanges {
        let new_windows = macos::get_windows();
        let mut changes = WindowChanges::default();

        // Build set of current window IDs
        let new_ids: std::collections::HashSet<_> = new_windows.iter().map(|w| w.id).collect();
        let old_ids: std::collections::HashSet<_> = self.windows.keys().copied().collect();

        // Detect new windows
        for id in new_ids.difference(&old_ids) {
            if let Some(window) = new_windows.iter().find(|w| w.id == *id) {
                changes.created.push(window.clone());
                trace!("New window: {} ({})", window.title, window.app_name);
            }
        }

        // Detect destroyed windows
        for id in old_ids.difference(&new_ids) {
            if let Some(window) = self.windows.get(id) {
                changes.destroyed.push(*id);
                trace!("Window destroyed: {}", window.title);
            }
        }

        // Detect title changes
        for window in &new_windows {
            if let Some(old_title) = self.previous_titles.get(&window.id) {
                if old_title != &window.title {
                    changes.title_changed.push((window.id, window.title.clone()));
                    trace!(
                        "Title changed: '{}' -> '{}'",
                        old_title,
                        window.title
                    );
                }
            }
        }

        // Update state
        self.windows.clear();
        self.previous_titles.clear();
        for window in new_windows {
            self.previous_titles.insert(window.id, window.title.clone());
            self.windows.insert(window.id, window);
        }

        // Update active window per display
        self.update_active_per_display(&mut changes);

        changes
    }

    fn update_active_per_display(&mut self, changes: &mut WindowChanges) {
        // Get the actual frontmost (focused) window using macOS APIs
        // There is only ONE active window system-wide - the one with keyboard focus
        if let Some(frontmost_window_id) = macos::get_frontmost_window_id() {
            if let Some(window) = self.windows.get(&frontmost_window_id) {
                let display_id = window.display_id;
                let prev = self.active_per_display.get(&display_id);
                if prev != Some(&frontmost_window_id) {
                    changes.focus_changed.push((display_id, frontmost_window_id));
                    // Clear all other displays - only one window can be active
                    self.active_per_display.clear();
                    self.active_per_display.insert(display_id, frontmost_window_id);
                }
            }
        }
    }

    /// Get the single active window (the one with keyboard focus)
    pub fn get_active_window(&self) -> Option<&WindowInfo> {
        // There's only one active window, get it from any display
        self.active_per_display.values().next()
            .and_then(|id| self.windows.get(id))
    }

    /// Get all current windows
    pub fn windows(&self) -> impl Iterator<Item = &WindowInfo> {
        self.windows.values()
    }

    /// Get a specific window by ID
    pub fn get_window(&self, id: WindowId) -> Option<&WindowInfo> {
        self.windows.get(&id)
    }

    /// Get the active window for a display
    pub fn active_window_for_display(&self, display_id: DisplayId) -> Option<&WindowInfo> {
        self.active_per_display
            .get(&display_id)
            .and_then(|id| self.windows.get(id))
    }

    /// Get display containing a point
    pub fn display_at_point(&self, x: i32, y: i32) -> Option<&DisplayInfo> {
        self.displays.iter().find(|d| d.bounds.contains(x, y))
    }

    /// Get display for a window
    pub fn display_for_window(&self, window_id: WindowId) -> Option<&DisplayInfo> {
        self.windows
            .get(&window_id)
            .and_then(|w| self.displays.iter().find(|d| d.id == w.display_id))
    }
}

impl Default for WindowTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Changes detected during window refresh
#[derive(Debug, Default)]
pub struct WindowChanges {
    /// Newly created windows
    pub created: Vec<WindowInfo>,
    /// Destroyed window IDs
    pub destroyed: Vec<WindowId>,
    /// Windows with changed titles (id, new_title)
    pub title_changed: Vec<(WindowId, String)>,
    /// Focus changed on display (display_id, new_active_window_id)
    pub focus_changed: Vec<(DisplayId, WindowId)>,
}

impl WindowChanges {
    pub fn is_empty(&self) -> bool {
        self.created.is_empty()
            && self.destroyed.is_empty()
            && self.title_changed.is_empty()
            && self.focus_changed.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_tracker_new() {
        let tracker = WindowTracker::new();
        assert!(tracker.displays.is_empty());
        assert!(tracker.windows.is_empty());
    }

    #[test]
    fn test_window_changes_is_empty() {
        let changes = WindowChanges::default();
        assert!(changes.is_empty());
    }
}
