use core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop};
use core_graphics::event::{
    CGEvent, CGEventFlags, CGEventTap, CGEventTapLocation, CGEventTapOptions,
    CGEventTapPlacement, CGEventTapProxy, CGEventType, CGKeyCode, EventField,
};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use std::cell::RefCell;
use std::time::{Duration, Instant};

const RCTRL_KEYCODE: i64 = 62;
const TAP_TIMEOUT: Duration = Duration::from_millis(200);
const VERBOSE: bool = false;

// Keycodes
const KEY_H: i64 = 4;
const KEY_J: i64 = 38;
const KEY_K: i64 = 40;
const KEY_L: i64 = 37;
const KEY_ESCAPE: CGKeyCode = 0x35;
const ARROW_LEFT: i64 = 0x7B;
const ARROW_DOWN: i64 = 0x7D;
const ARROW_UP: i64 = 0x7E;
const ARROW_RIGHT: i64 = 0x7C;

struct RemapState {
    rctrl_pressed_at: Option<Instant>,
    rctrl_used_as_modifier: bool,
}

fn post_escape() {
    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .expect("Failed to create event source");
    if let Ok(down) = CGEvent::new_keyboard_event(source.clone(), KEY_ESCAPE, true) {
        down.set_flags(CGEventFlags::CGEventFlagNonCoalesced);
        down.post(CGEventTapLocation::HID);
    }
    let source2 = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .expect("Failed to create event source");
    if let Ok(up) = CGEvent::new_keyboard_event(source2, KEY_ESCAPE, false) {
        up.set_flags(CGEventFlags::CGEventFlagNonCoalesced);
        up.post(CGEventTapLocation::HID);
    }
}

fn remap_hjkl(keycode: i64) -> Option<i64> {
    match keycode {
        KEY_H => Some(ARROW_LEFT),
        KEY_J => Some(ARROW_DOWN),
        KEY_K => Some(ARROW_UP),
        KEY_L => Some(ARROW_RIGHT),
        _ => None,
    }
}

fn main() {
    let state = RefCell::new(RemapState {
        rctrl_pressed_at: None,
        rctrl_used_as_modifier: false,
    });

    let events = vec![
        CGEventType::KeyDown,
        CGEventType::KeyUp,
        CGEventType::FlagsChanged,
    ];

    let tap = CGEventTap::new(
        CGEventTapLocation::HID,
        CGEventTapPlacement::HeadInsertEventTap,
        CGEventTapOptions::Default,
        events,
        move |_proxy: CGEventTapProxy, event_type: CGEventType, event: &CGEvent| -> Option<CGEvent> {
            let keycode = event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE);
            let flags = event.get_flags();

            match event_type {
                CGEventType::KeyDown | CGEventType::KeyUp => {
                    // Only keyDown marks rctrl as used (matching Lua behavior)
                    if matches!(event_type, CGEventType::KeyDown) {
                        let mut s = state.borrow_mut();
                        if s.rctrl_pressed_at.is_some() {
                            s.rctrl_used_as_modifier = true;
                        }
                    }

                    // Alt+HJKL → Arrow keys
                    let alt_held = flags.contains(CGEventFlags::CGEventFlagAlternate);
                    if alt_held {
                        if let Some(arrow) = remap_hjkl(keycode) {
                            let label = match keycode {
                                KEY_H => "Left",
                                KEY_J => "Down",
                                KEY_K => "Up",
                                KEY_L => "Right",
                                _ => unreachable!(),
                            };
                            let key_name = match keycode {
                                KEY_H => "h",
                                KEY_J => "j",
                                KEY_K => "k",
                                KEY_L => "l",
                                _ => unreachable!(),
                            };
                            if matches!(event_type, CGEventType::KeyDown) {
                                eprintln!("Alt+{} -> {}", key_name, label);
                            }

                            let modified = event.clone();
                            modified.set_integer_value_field(
                                EventField::KEYBOARD_EVENT_KEYCODE,
                                arrow,
                            );
                            // Strip Alt flag, preserve Shift/Ctrl/Cmd
                            let new_flags = flags & !CGEventFlags::CGEventFlagAlternate;
                            modified.set_flags(new_flags);
                            return Some(modified);
                        }
                    }

                    Some(event.clone())
                }

                CGEventType::FlagsChanged => {
                    if keycode != RCTRL_KEYCODE {
                        return Some(event.clone());
                    }

                    let ctrl_active = flags.contains(CGEventFlags::CGEventFlagControl);

                    if ctrl_active {
                        // Right Control pressed
                        let mut s = state.borrow_mut();
                        s.rctrl_pressed_at = Some(Instant::now());
                        s.rctrl_used_as_modifier = false;
                        if VERBOSE {
                            eprintln!("rctrl pressed");
                        }
                        Some(event.clone())
                    } else {
                        // Right Control released
                        let mut s = state.borrow_mut();
                        let held = s
                            .rctrl_pressed_at
                            .map(|t| t.elapsed())
                            .unwrap_or(Duration::MAX);
                        let used = s.rctrl_used_as_modifier;

                        if VERBOSE {
                            eprintln!(
                                "rctrl released (held={}ms, used_as_mod={})",
                                held.as_millis(),
                                used
                            );
                        }

                        s.rctrl_pressed_at = None;

                        if held < TAP_TIMEOUT && !used {
                            eprintln!("rctrl tap -> Escape");
                            drop(s); // release borrow before posting
                            post_escape();
                        }

                        // Always pass through the rctrl release so apps see
                        // the Control key-up (matches Lua behavior)
                        Some(event.clone())
                    }
                }

                CGEventType::TapDisabledByTimeout => {
                    eprintln!("WARNING: Event tap disabled by timeout — re-enabling");
                    Some(event.clone())
                }

                _ => Some(event.clone()),
            }
        },
    )
    .expect("Failed to create event tap — grant Accessibility permission to this terminal");

    let loop_source = tap
        .mach_port
        .create_runloop_source(0)
        .expect("Failed to create run loop source");

    unsafe {
        let run_loop = CFRunLoop::get_current();
        run_loop.add_source(&loop_source, kCFRunLoopCommonModes);
    }

    tap.enable();

    eprintln!("keytaps active: RCtrl tap→Escape, Alt+HJKL→Arrows. Press Ctrl+C to stop.");
    CFRunLoop::run_current();
}
