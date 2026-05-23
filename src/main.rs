#![no_std]
#![no_main]
extern crate alloc;

mod state;

use crate::state::*;
use firefly_rust::*;
use firefly_sudo::sudo;
use firefly_types::Encode;
use firefly_ui::Input;

#[unsafe(no_mangle)]
extern "C" fn boot() {
    load_state();
}

#[unsafe(no_mangle)]
extern "C" fn before_exit() {
    let state = get_state();
    _ = update_settings(state);
    _ = update_stats(state);
}

/// If there are new badges earned, add their XP to the total player XP in settings.
fn update_settings(state: &State) -> Option<()> {
    let items = state.items.as_ref()?;
    let raw = sudo::load_file_buf("sys/settings")?;
    let raw = raw.into_bytes();
    let mut settings = firefly_types::Settings::decode(&raw).ok()?;
    let mut dirty = false;
    for item in items {
        if item.new {
            dirty = true;
            settings.xp += u32::from(item.xp.min(200));
        }
    }
    if dirty {
        let raw = settings.encode_vec().ok()?;
        sudo::dump_file("sys/settings", &raw);
    }
    Some(())
}

/// Mark newly earned badges as viewed and add their XP to the total XP earned in the app.
fn update_stats(state: &State) -> Option<()> {
    let items = state.items.as_ref()?;
    let (author_id, app_id) = state.target.as_ref()?;
    let stats_path = alloc::format!("data/{author_id}/{app_id}/stats");
    let raw = sudo::load_file_buf(&stats_path)?;
    let raw = raw.into_bytes();
    let mut stats = firefly_types::Stats::decode(&raw).ok()?;
    let mut dirty = false;
    for (badge, item) in stats.badges.iter_mut().zip(items) {
        if badge.new {
            dirty = true;
        }
        badge.new = false;
        let new_xp = stats.xp + u16::from(item.xp.min(200));
        stats.xp = new_xp.min(1000);
    }
    if dirty {
        let raw = stats.encode_vec().ok()?;
        sudo::dump_file(&stats_path, &raw);
    }
    Some(())
}

#[unsafe(no_mangle)]
extern "C" fn update() {
    let state = get_state();
    state.input.update();
    match state.input.get() {
        Input::Up => {
            state.cursor = state.cursor.saturating_sub(1);
        }
        Input::Down => {
            if let Some(items) = &state.items {
                // Move cursor to the next visible item.
                let old_cursor = state.cursor;
                state.cursor += 1;
                loop {
                    let Some(item) = items.get(state.cursor) else {
                        state.cursor = old_cursor;
                        break;
                    };
                    if item.visible {
                        break;
                    }
                    state.cursor += 1;
                }
            }
        }
        Input::Left => state.cursor = 0,
        Input::Right => {
            if let Some(items) = &state.items {
                // Move cursor to the last visible item.
                for (i, item) in items.iter().enumerate() {
                    if item.visible {
                        state.cursor = i;
                    }
                }
            }
        }
        Input::Back => quit(),
        _ => {}
    }
}

#[unsafe(no_mangle)]
extern "C" fn render() {
    let state = get_state();
    draw_bg_grid(state.settings.theme);
    draw_items(state);
}

fn draw_items(state: &State) {
    const MARGIN: i32 = 12;

    let Some(items) = state.items.as_ref() else {
        return;
    };
    let theme = state.settings.theme;
    let font = &state.font;
    let box_width = WIDTH - MARGIN * 2;
    let corner = Size::new(4, 4);
    let mut point = Point::new(MARGIN, MARGIN);
    for (i, item) in items.iter().enumerate() {
        let expanded = i == state.cursor;
        let earned = item.done >= item.goal;
        let show_bar = expanded && item.done != 0 && item.goal != 0;
        if !item.visible {
            continue;
        }

        let color = if item.new {
            theme.accent
        } else if earned {
            theme.primary
        } else {
            theme.secondary
        };

        let mut box_height = 12;
        if expanded && !item.descr.is_empty() {
            box_height += 10;
        }
        if show_bar {
            box_height += 10;
        }

        // Box.
        let size = Size::new(box_width, box_height);
        let point_shadow = Point::new(point.x + 1, point.y + 1);
        draw_rounded_rect(point_shadow, size, corner, Style::solid(color));
        let style = Style {
            fill_color: theme.bg,
            stroke_color: color,
            stroke_width: 1,
        };
        draw_rounded_rect(point, size, corner, style);

        // Name.
        let mut point_text = Point::new(point.x + 4, point.y + 7);
        draw_text(&item.name, font, point_text, theme.accent);

        // XP.
        {
            let text_xp = alloc::format!("{}xp", item.xp);
            let text_w = font.line_width_ascii(&text_xp) as i32;
            let point_xp = Point::new(WIDTH - MARGIN - 4 - text_w, point_text.y);
            draw_text(&text_xp, font, point_xp, theme.secondary);
        }

        // Description.
        if expanded && !item.descr.is_empty() {
            point_text.y += 10;
            draw_text(&item.descr, font, point_text, theme.primary);
        }

        // Progress bar.
        if show_bar {
            let point_bar = Point::new(point_text.x, point_text.y + 4);
            let bar_width = WIDTH - MARGIN * 2 - 8;
            {
                let progress = f32::from(item.done) / f32::from(item.goal);
                let mut progress = (progress * bar_width as f32) as i32;
                if earned {
                    progress = bar_width;
                } else if progress >= bar_width {
                    progress = bar_width - 2;
                }
                if progress > 0 {
                    let size_bar = Size::new(progress, 8);
                    draw_rect(point_bar, size_bar, Style::solid(theme.accent));
                }
            }
            let size_bar = Size::new(bar_width, 8);
            draw_rect(point_bar, size_bar, Style::outlined(theme.primary, 1));
        }

        point.y += box_height + 4;
    }
}

fn draw_bg_grid(theme: Theme) {
    const CELL_SIZE: i32 = 8;

    clear_screen(theme.bg);
    let style = LineStyle::new(theme.secondary, 1);
    for x in (CELL_SIZE..WIDTH).step_by(CELL_SIZE as _) {
        draw_line(Point::new(x, 0), Point::new(x, HEIGHT), style);
    }
    for y in (CELL_SIZE..HEIGHT).step_by(CELL_SIZE as _) {
        draw_line(Point::new(0, y), Point::new(WIDTH, y), style);
    }
}
