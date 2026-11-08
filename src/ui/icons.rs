//! App-specific icons (16pt view box, `currentColor` strokes), in the style of rune-kit's.

use rune::Icon;

pub static SKULL: Icon = Icon {
    name: "wks-skull",
    view: 16.0,
    body: r#"<path d="M8 2.2C5.1 2.2 3 4.2 3 6.9C3 8.4 3.7 9.5 4.6 10.2V11.8C4.6 12.3 5 12.7 5.5 12.7H6V11.3H7V12.7H9V11.3H10V12.7H10.5C11 12.7 11.4 12.3 11.4 11.8V10.2C12.3 9.5 13 8.4 13 6.9C13 4.2 10.9 2.2 8 2.2Z" stroke="currentColor" stroke-width="1.1" stroke-linejoin="round" fill="none"/><circle cx="6.2" cy="7" r="1" fill="currentColor"/><circle cx="9.8" cy="7" r="1" fill="currentColor"/>"#,
};

pub static PROCESSES: Icon = Icon {
    name: "wks-processes",
    view: 16.0,
    body: r#"<rect x="2.5" y="3" width="11" height="3" rx="1" stroke="currentColor" stroke-width="1.1" fill="none"/><rect x="2.5" y="7" width="11" height="3" rx="1" stroke="currentColor" stroke-width="1.1" fill="none"/><rect x="2.5" y="11" width="7" height="2.5" rx="1" stroke="currentColor" stroke-width="1.1" fill="none"/>"#,
};

pub static PAUSE: Icon = Icon {
    name: "wks-pause",
    view: 16.0,
    body: r#"<rect x="5" y="3.5" width="2" height="9" rx="1" fill="currentColor"/><rect x="9" y="3.5" width="2" height="9" rx="1" fill="currentColor"/>"#,
};

pub static PLUS: Icon = Icon {
    name: "wks-plus",
    view: 16.0,
    body: r#"<path d="M8 3.5V12.5M3.5 8H12.5" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/>"#,
};

pub static TRASH: Icon = Icon {
    name: "wks-trash",
    view: 16.0,
    body: r#"<path d="M3.5 4.5H12.5M6.5 4.5V3.5H9.5V4.5M5 4.5L5.5 12.5H10.5L11 4.5" stroke="currentColor" stroke-width="1.1" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"#,
};

pub static RENAME: Icon = Icon {
    name: "wks-rename",
    view: 16.0,
    body: r#"<path d="M3 11L10.5 3.5L12.5 5.5L5 13H3V11Z" stroke="currentColor" stroke-width="1.1" stroke-linejoin="round" fill="none"/>"#,
};

pub static LINK: Icon = Icon {
    name: "wks-link",
    view: 16.0,
    body: r#"<path d="M6.5 9.5L9.5 6.5M7 4.5L8.5 3C9.6 1.9 11.1 1.9 12 3C12.9 3.9 12.9 5.4 11.8 6.5L10.5 7.8M9 11.5L7.5 13C6.4 14.1 4.9 14.1 4 13C3.1 12.1 3.1 10.6 4.2 9.5L5.5 8.2" stroke="currentColor" stroke-width="1.1" stroke-linecap="round" fill="none"/>"#,
};

pub static UNLINK: Icon = Icon {
    name: "wks-unlink",
    view: 16.0,
    body: r#"<path d="M7 4.5L8.5 3C9.6 1.9 11.1 1.9 12 3C12.9 3.9 12.9 5.4 11.8 6.5L10.5 7.8M9 11.5L7.5 13C6.4 14.1 4.9 14.1 4 13C3.1 12.1 3.1 10.6 4.2 9.5L5.5 8.2M2.5 2.5L13.5 13.5" stroke="currentColor" stroke-width="1.1" stroke-linecap="round" fill="none"/>"#,
};

pub static SETTINGS: Icon = Icon {
    name: "wks-settings",
    view: 16.0,
    body: r#"<circle cx="8" cy="8" r="2.2" stroke="currentColor" stroke-width="1.2" fill="none"/><path d="M8 1.8V3.4M8 12.6V14.2M14.2 8H12.6M3.4 8H1.8M12.4 3.6L11.3 4.7M4.7 11.3L3.6 12.4M12.4 12.4L11.3 11.3M4.7 4.7L3.6 3.6" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>"#,
};

macro_rules! icon16 {
    ($id:ident, $name:literal, $body:literal) => {
        pub static $id: Icon = Icon { name: $name, view: 16.0, body: $body };
    };
}

icon16!(CLEAN, "wks-clean", r#"<path d="M7 2.5L8 5.5L11 6.5L8 7.5L7 10.5L6 7.5L3 6.5L6 5.5Z" fill="currentColor"/><path d="M12 9.5L12.6 11.4L14.5 12L12.6 12.6L12 14.5L11.4 12.6L9.5 12L11.4 11.4Z" fill="currentColor"/>"#);
icon16!(CHIP, "wks-chip", r#"<rect x="4" y="4" width="8" height="8" rx="1.5" stroke="currentColor" stroke-width="1.2" fill="none"/><path d="M6 1.8V4M10 1.8V4M6 12V14.2M10 12V14.2M1.8 6H4M1.8 10H4M12 6H14.2M12 10H14.2" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>"#);
icon16!(DRIVE, "wks-drive", r#"<rect x="2" y="4" width="12" height="8" rx="2" stroke="currentColor" stroke-width="1.2" fill="none"/><circle cx="11.2" cy="8" r=".9" fill="currentColor"/><path d="M4.5 8H8" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>"#);
icon16!(CLOCK, "wks-clock", r#"<circle cx="8" cy="8" r="5.5" stroke="currentColor" stroke-width="1.2" fill="none"/><path d="M8 5V8.2L10 9.5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" fill="none"/>"#);
icon16!(WINDOW, "wks-window", r#"<rect x="2.5" y="3" width="11" height="10" rx="1.5" stroke="currentColor" stroke-width="1.2" fill="none"/><path d="M2.5 5.8H13.5" stroke="currentColor" stroke-width="1.2"/>"#);
icon16!(ALERT, "wks-alert", r#"<path d="M8 2.5L14 13H2Z" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round" fill="none"/><path d="M8 6.5V9.2" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/><circle cx="8" cy="11" r=".7" fill="currentColor"/>"#);
icon16!(IMAGE, "wks-image", r#"<rect x="2.5" y="3" width="11" height="10" rx="1.5" stroke="currentColor" stroke-width="1.2" fill="none"/><circle cx="6" cy="6.5" r="1.1" fill="currentColor"/><path d="M3 12L6.5 8.8L9 11L11 9.2L13.5 11.5" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round" fill="none"/>"#);
icon16!(GLOBE, "wks-globe", r#"<circle cx="8" cy="8" r="5.5" stroke="currentColor" stroke-width="1.2" fill="none"/><path d="M2.5 8H13.5M8 2.5C9.8 4.2 9.8 11.8 8 13.5M8 2.5C6.2 4.2 6.2 11.8 8 13.5" stroke="currentColor" stroke-width="1.2" fill="none"/>"#);
icon16!(CUBE, "wks-cube", r#"<path d="M8 2.2L13 5V11L8 13.8L3 11V5Z" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round" fill="none"/><path d="M3 5L8 7.8L13 5M8 7.8V13.8" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round" fill="none"/>"#);
icon16!(REFRESH, "wks-refresh", r#"<path d="M12.8 6.5A5 5 0 1 0 13 9.5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" fill="none"/><path d="M13.2 3.2V6.8H9.6" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"#);
icon16!(SEARCH, "wks-search", r#"<circle cx="7" cy="7" r="4" stroke="currentColor" stroke-width="1.2" fill="none"/><path d="M10 10L13.2 13.2" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>"#);
icon16!(CHECK, "wks-check", r#"<path d="M3.5 8.5L6.5 11.5L12.5 4.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"#);

icon16!(TARGET, "wks-target", r#"<circle cx="8" cy="8" r="5" stroke="currentColor" stroke-width="1.2" fill="none"/><circle cx="8" cy="8" r="1.4" fill="currentColor"/><path d="M8 1.5V4M8 12V14.5M1.5 8H4M12 8H14.5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>"#);

icon16!(CHEVRON, "wks-chevron", r#"<path d="M6 3.5L10.5 8L6 12.5" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"#);
