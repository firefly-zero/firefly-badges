#![no_std]
#![no_main]
extern crate alloc;

mod state;

use crate::state::*;
use firefly_rust::*;
use firefly_ui::Input;

#[unsafe(no_mangle)]
extern "C" fn boot() {
    load_state();
}

#[unsafe(no_mangle)]
extern "C" fn update() {
    let state = get_state();
    state.input.update();
    match state.input.get() {
        Input::Up => {
            if state.cursor > 0 {
                state.cursor -= 1;
            }
        }
        Input::Left => state.cursor = 0,
        Input::Down => {
            if let Some(items) = &state.items {
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
    let font = state.font.as_font();
    let box_width = WIDTH - MARGIN * 2;
    let corner = Size::new(4, 4);
    let mut point = Point::new(MARGIN, MARGIN);
    for (i, item) in items.iter().enumerate() {
        let expanded = i == state.cursor;
        let earned = item.done >= item.goal;
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

        let size = Size::new(box_width, box_height);
        let point_shadow = Point::new(point.x + 1, point.y + 1);
        draw_rounded_rect(point_shadow, size, corner, Style::solid(color));
        let style = Style {
            fill_color: theme.bg,
            stroke_color: color,
            stroke_width: 1,
        };
        draw_rounded_rect(point, size, corner, style);

        let point_name = Point::new(point.x + 4, point.y + 7);
        draw_text(&item.name, &font, point_name, color);

        {
            let text_xp = alloc::format!("{}xp", item.xp);
            let text_w = font.line_width_ascii(&text_xp) as i32;
            let point_xp = Point::new(WIDTH - MARGIN - 4 - text_w, point_name.y);
            draw_text(&text_xp, &font, point_xp, theme.secondary);
        }

        if expanded && !item.descr.is_empty() {
            let point_descr = Point::new(point_name.x, point_name.y + 10);
            draw_text(&item.descr, &font, point_descr, theme.primary);
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
