use crate::*;
use alloc::{
    string::{String, ToString},
    vec::{self, Vec},
};
use core::mem::MaybeUninit;
use firefly_rust::*;
use firefly_types::{Encode, Stats};

static mut STATE: MaybeUninit<State> = MaybeUninit::uninit();

pub struct Item {
    pub position: u16,

    /// How much XP earning the badge brings to the player.
    pub xp: u8,

    /// The number of steps required for the badge to be shown.
    ///
    /// If zero, the badge is always shown. If equal to the number of steps
    /// required to earn the badge, the badge will be shown only when earned.
    pub hidden: u16,

    /// Human-readable badge name.
    pub name: String,

    /// Human-readable badge description. Typically, a hint on how to earn it.
    pub descr: String,

    /// If true, the earning of the badge hasn't been shown to the player yet.
    pub new: bool,

    /// How many points are already earned for the badge.
    pub done: u16,

    /// How many points needed to earn the badge.
    pub goal: u16,
}

pub struct State {
    pub font: FileBuf,
    pub target: Option<(String, String)>,
    pub settings: Settings,
    pub cursor: usize,
    pub input: firefly_ui::InputManager,
    pub items: Option<Vec<Item>>,
}

pub fn get_state() -> &'static mut State {
    #[allow(static_mut_refs)]
    unsafe {
        STATE.assume_init_mut()
    }
}

pub fn load_state() {
    let settings = get_settings(get_me());
    let encoding = settings.language.encoding();
    let font = load_file_buf(encoding).unwrap_or_else(|| load_file_buf("ascii").unwrap());
    let target = load_target();
    let items = match &target {
        Some((author_id, app_id)) => load_items(author_id, app_id),
        None => None,
    };

    if target.is_none() {
        log_error("failed to load target");
    } else if items.is_none() {
        log_error("app has no badges");
    }

    let state = State {
        font,
        target,
        settings,
        cursor: 0,
        input: firefly_ui::InputManager::new(),
        items,
    };
    #[allow(static_mut_refs)]
    unsafe {
        STATE.write(state)
    };
}

fn load_items(author_id: &str, app_id: &str) -> Option<Vec<Item>> {
    let badges_path = alloc::format!("roms/{author_id}/{app_id}/_badges");
    let raw = sudo::load_file_buf(&badges_path)?;
    let badges = firefly_types::Badges::decode(raw.as_bytes()).ok()?;

    let stats_path = alloc::format!("data/{author_id}/{app_id}/stats");
    let raw = sudo::load_file_buf(&stats_path)?;
    let stats = firefly_types::Stats::decode(raw.as_bytes()).ok()?;

    if badges.badges.len() != stats.badges.len() {
        return None;
    }
    let mut items = Vec::new();
    for (badge, progess) in badges.badges.iter().zip(stats.badges) {
        let item = Item {
            position: badge.position,
            xp: badge.xp,
            hidden: badge.hidden,
            name: badge.name.to_string(),
            descr: badge.descr.to_string(),

            new: progess.new,
            done: progess.done,
            goal: progess.goal,
        };
        items.push(item);
    }
    if items.is_empty() {
        return None;
    }
    Some(items)
}

/// Read the ID of the app to be removed.
fn load_target() -> Option<(String, String)> {
    let raw = load_file_buf("target")?;
    let raw = raw.as_bytes();
    let raw = raw.trim_ascii();
    let raw = alloc::str::from_utf8(raw).ok()?;
    let (author, app) = split_by(raw, '.')?;
    let target = (String::from(author), String::from(&app[1..]));
    Some(target)
}

/// Split the string once at the given character.
fn split_by(input: &str, sep: char) -> Option<(&str, &str)> {
    let mut split_at = None;
    let sep: u8 = sep.try_into().unwrap();
    for (i, ch) in input.bytes().enumerate() {
        if ch == sep {
            split_at = Some(i);
            break;
        }
    }
    let split_at = split_at?;
    Some(input.split_at(split_at))
}
