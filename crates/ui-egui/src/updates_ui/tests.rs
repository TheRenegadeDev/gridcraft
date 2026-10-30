#![allow(clippy::unwrap_used)]

use super::*;
use crate::Services;
use egui::Key;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use gridcraft_engine::Session;
use serde_json::json;

fn harness(render: bool) -> Harness<'static, SheetApp> {
    let mut session = Session::new();
    session.new_workbook();
    session.execute("cell.set", json!({"cell": "A1", "input": "42"})).unwrap();
    let mut app = SheetApp::new(session, Services::default());
    app.use_prepared_fonts();
    let builder = Harness::builder().with_size(egui::vec2(1200.0, 800.0)).with_context_setup(|ctx| {
        ctx.set_fonts(crate::theme::pinned_font_definitions());
        crate::theme::apply(ctx, false);
    });
    let builder = if render { builder.wgpu() } else { builder };
    let mut h = builder.build_ui_state(
        |ui, app: &mut SheetApp| {
            let ctx = ui.ctx().clone();
            app.logic(&ctx);
            app.ui(ui);
        },
        app,
    );
    h.run_steps(4);
    let ctx = h.ctx.clone();
    h.state_mut().request_quit(&ctx);
    h.run_steps(2);
    assert_eq!(h.state().dialog.as_ref().map(|d| d.name()), Some("saveChanges"));
    h
}

fn offer(h: &mut Harness<'static, SheetApp>) {
    h.state_mut().updates_rt.updater.offer(craft_update::release::Release {
        tag: "v99.0.0".into(),
        name: "GridCraft 99.0.0".into(),
        html_url: "https://example.invalid/gridcraft/release".into(),
        ..Default::default()
    });
}

#[test]
fn save_changes_yields_to_the_update_prompt_and_accepts_fresh_input_afterward() {
    for key in [Key::N, Key::Y, Key::Escape] {
        let image = std::env::var("GRIDCRAFT_UPDATE_SAVE_SNAPSHOT").ok().filter(|_| key == Key::N);
        let mut h = harness(image.is_some());
        let uid = h.state().session.doc().unwrap().uid;
        offer(&mut h);
        h.run_steps(3);
        assert!(h.query_by_label("LATER").is_some(), "the actual updater prompt is drawn");
        if let Some(path) = &image {
            h.render().unwrap().save(path).unwrap();
        }
        h.key_press(key);
        h.run_steps(3);
        assert!(h.state().session.documents().iter().any(|d| d.uid == uid && d.is_dirty()), "covered key {key:?} must retain unsaved work");
        assert_eq!(h.state().dialog.as_ref().map(|d| d.name()), Some("saveChanges"));
        assert_eq!(h.state().updates_rt.updater.prompt_open(), key != Key::Escape);
        if key != Key::Escape {
            h.key_press(Key::Escape);
            h.run_steps(3);
        }
        assert!(!h.state().updates_rt.updater.prompt_open());
        assert_eq!(h.state().dialog.as_ref().map(|d| d.name()), Some("saveChanges"), "Escape dismisses only the update prompt");
        if let Some(path) = image {
            h.render().unwrap().save(std::path::Path::new(&path).with_extension("save.png")).unwrap();
        }
        h.key_press(Key::N);
        h.run_steps(3);
        assert!(h.state().session.documents().iter().all(|d| d.uid != uid), "a fresh N still answers after the prompt closes");
    }
}

#[test]
fn update_prompt_cancels_armed_and_pending_answers_even_on_its_dismissal_frame() {
    for pending in [false, true] {
        for (immediate_dismissal, click_later) in [(true, false), (false, false), (false, true)] {
            let mut h = harness(false);
            let uid = h.state().session.doc().unwrap().uid;
            h.key_down(Key::N);
            h.run_steps(2);
            if pending {
                h.key_press(Key::Y);
                h.run_steps(2);
            }
            offer(&mut h);
            if !immediate_dismissal {
                h.run_steps(2);
            }
            if click_later {
                h.key_up(Key::N);
                h.get_by_label("LATER").click();
            } else {
                // These events must reach one pass, rather than separate harness key gestures.
                for (key, pressed) in [(Key::N, false), (Key::Escape, true), (Key::Escape, false)] {
                    h.input_mut().events.push(egui::Event::Key {
                        key,
                        physical_key: Some(key),
                        pressed,
                        repeat: false,
                        modifiers: egui::Modifiers::NONE,
                    });
                }
            }
            h.run_steps(3);
            assert!(!h.state().updates_rt.updater.prompt_open());
            assert!(
                h.state().session.documents().iter().any(|d| d.uid == uid && d.is_dirty()),
                "pending={pending}, immediate_dismissal={immediate_dismissal}, click_later={click_later}"
            );
            assert_eq!(h.state().dialog.as_ref().map(|d| d.name()), Some("saveChanges"));
            h.key_press(Key::N);
            h.run_steps(3);
            assert!(h.state().session.documents().iter().all(|d| d.uid != uid));
        }
    }
}
