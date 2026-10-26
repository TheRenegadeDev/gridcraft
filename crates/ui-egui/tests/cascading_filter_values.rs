#[path = "common/mod.rs"]
mod common;

use egui::{Event, Key, Modifiers};
use egui_kittest::kittest::Queryable;
use gridcraft_engine::Session;
use serde_json::json;

fn key(h: &mut common::Harness, key: Key, modifiers: Modifiers) {
    for pressed in [true, false] {
        h.input_mut().events.push(Event::Key { key, physical_key: Some(key), pressed, repeat: false, modifiers });
    }
    h.run_steps(3);
}

#[test]
fn dropdown_can_change_and_clear_a_column_while_another_filter_stays_active() {
    for table in [false, true] {
        let mut session = Session::new();
        session.new_workbook();
        session
            .execute(
                "range.setValues",
                json!({"range": "A1", "values": [
                    ["Company", "Job"], ["East", "Alpha"], ["East", "Beta"], ["West", "Other"], ["East", null]
                ]}),
            )
            .unwrap();
        if table {
            session.execute("insert.table", json!({"range": "A1:B5", "header": true})).unwrap();
        }
        session.execute("data.filterBy", json!({"column": "A", "values": ["East"]})).unwrap();
        session.execute("data.filterBy", json!({"column": "B", "values": ["Alpha"]})).unwrap();
        session.execute("selection.set", json!({"cell": "B1"})).unwrap();
        let screenshot = std::env::var("GRIDCRAFT_CASCADING_SNAPSHOT").ok().filter(|_| !table);
        let mut h = if screenshot.is_some() {
            common::harness_with(
                common::builder().with_os(egui::os::OperatingSystem::Windows).with_size(egui::vec2(1200.0, 800.0)).wgpu(),
                common::app(session),
                4,
            )
        } else {
            common::os_harness(session, egui::os::OperatingSystem::Windows)
        };
        key(&mut h, Key::ArrowDown, Modifiers::ALT);
        assert!(h.query_by_label("Alpha").is_some());
        assert!(h.query_by_label("Beta").is_some(), "current-column filtering must not remove alternatives");
        assert!(h.query_by_label("(Blanks)").is_some());
        assert!(h.query_by_label("Other").is_none(), "the other company's job must not be listed");
        if let Some(path) = screenshot {
            h.render().unwrap().save(path).unwrap();
        }
        h.get_by_label("Alpha").click();
        h.run_steps(2);
        h.get_by_label("Beta").click();
        h.run_steps(2);
        h.get_by_label("Apply").click();
        h.run_steps(3);
        let hidden = |h: &common::Harness| {
            let sh = h.state().session.active().unwrap().wb.active().unwrap();
            (1..=4).map(|row| sh.is_row_hidden(row)).collect::<Vec<_>>()
        };
        assert_eq!(hidden(&h), [true, false, true, true]);
        key(&mut h, Key::ArrowDown, Modifiers::ALT);
        h.get_by_label("Clear Filter").click();
        h.run_steps(3);
        assert_eq!(hidden(&h), [false, false, true, false], "clearing Job keeps Company filtered");
    }
}
