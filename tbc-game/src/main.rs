mod content;
mod game;
mod levels;

use game::PlaySession;
use levels::{discover_levels, LevelInfo};
use macroquad::prelude::*;

#[derive(PartialEq, Eq)]
enum Screen {
    Menu,
    Playing,
}

struct App {
    screen: Screen,
    levels: Vec<LevelInfo>,
    menu_index: usize,
    session: Option<PlaySession>,
}

#[macroquad::main("TBC Play")]
async fn main() {
    let levels = discover_levels();
    let mut app = App {
        screen: Screen::Menu,
        levels,
        menu_index: 0,
        session: None,
    };

    loop {
        match app.screen {
            Screen::Menu => {
                if is_key_pressed(KeyCode::Escape) {
                    break;
                }
                if is_key_pressed(KeyCode::Up) && app.menu_index > 0 {
                    app.menu_index -= 1;
                }
                if is_key_pressed(KeyCode::Down) && app.menu_index + 1 < app.levels.len() {
                    app.menu_index += 1;
                }
                if is_key_pressed(KeyCode::Enter) {
                    let level = app.levels[app.menu_index].clone();
                    match game::start_level(level) {
                        Ok(sess) => {
                            app.session = Some(sess);
                            app.screen = Screen::Playing;
                        }
                        Err(e) => {
                            eprintln!("Failed to start level: {}", e);
                        }
                    }
                }
                draw_menu(&app);
            }
            Screen::Playing => {
                if is_key_pressed(KeyCode::Escape) {
                    app.session = None;
                    app.screen = Screen::Menu;
                } else if let Some(session) = &mut app.session {
                    session.ensure_textures().await;

                    let mut dir = vec2(0.0, 0.0);
                    if is_key_down(KeyCode::W) || is_key_down(KeyCode::Up) {
                        dir.y += 1.0;
                    }
                    if is_key_down(KeyCode::S) || is_key_down(KeyCode::Down) {
                        dir.y -= 1.0;
                    }
                    if is_key_down(KeyCode::A) || is_key_down(KeyCode::Left) {
                        dir.x -= 1.0;
                    }
                    if is_key_down(KeyCode::D) || is_key_down(KeyCode::Right) {
                        dir.x += 1.0;
                    }
                    if is_key_pressed(KeyCode::E) {
                        session.interact();
                    }
                    if is_key_pressed(KeyCode::Space) {
                        session.attack();
                    }

                    let dt = get_frame_time();
                    session.tick(dt, dir);
                    let snap = session.snapshot();
                    session.draw(&snap);
                }
            }
        }

        next_frame().await;
    }
}

fn draw_menu(app: &App) {
    clear_background(Color::from_rgba(12, 16, 24, 255));
    let title = "TBC Play";
    let tw = measure_text(title, None, 48, 1.0).width;
    draw_text(title, screen_width() * 0.5 - tw * 0.5, 72.0, 48.0, WHITE);
    draw_text(
        "Select a level · ↑↓ · Enter to play · Esc to quit",
        24.0,
        120.0,
        20.0,
        LIGHTGRAY,
    );

    if app.levels.is_empty() {
        draw_text(
            "No levels found. Add packs under assets/packs/ or run from repo root.",
            24.0,
            180.0,
            22.0,
            YELLOW,
        );
        return;
    }

    let list_top = 150.0;
    let row_h = 56.0;
    let visible = ((screen_height() - list_top - 40.0) / row_h).floor() as usize;

    let scroll = if visible == 0 {
        0
    } else if app.menu_index >= visible {
        app.menu_index - visible + 1
    } else {
        0
    };

    let start = scroll;
    for (i, level) in app.levels.iter().enumerate().skip(start).take(visible) {
        let y = list_top + (i - start) as f32 * row_h;
        let selected = i == app.menu_index;
        if selected {
            draw_rectangle(16.0, y - 8.0, screen_width() - 32.0, row_h - 4.0, Color::from_rgba(50, 80, 140, 180));
        }
        draw_text(&level.title, 32.0, y + 16.0, 28.0, if selected { WHITE } else { LIGHTGRAY });
        let sub = if level.description.is_empty() {
            format!("{} · {}", level.id, level.ruleset_id)
        } else {
            format!("{} · {}", level.description, level.ruleset_id)
        };
        draw_text(&sub, 32.0, y + 38.0, 18.0, DARKGRAY);
    }
}
