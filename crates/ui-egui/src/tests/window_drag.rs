use super::*;
use bimcraft_core::model::Opening;

const WALL: &str = include_str!("window_drag.script");

fn opening(m: &bimcraft_core::model::Model, id: Id) -> Opening {
    let Some(Element::Window(w)) = m.elements.get(id) else { panic!("window {id} missing") };
    w.clone()
}

fn capture(h: &mut Harness<BimApp>, from: Pos2, to: Pos2) {
    h.hover_at(from);
    h.run();
    h.event(button(from, true));
    h.run();
    h.hover_at(to);
    h.run();
    assert!(h.state().orbit.window_drag_active(), "window body took the primary drag");
}

#[test]
fn hosted_window_drag_moves_the_cut_and_commits_once() {
    let mut h = app(WALL);
    press(&mut h, "Default 3D View");
    let id = ids(&h, "window")[0];
    let wall = ids(&h, "wall")[0];
    let from = h.state().screen3([2000.0, 0.0, 1500.0]);
    let to = h.state().screen3([3600.0, 0.0, 1850.0]);
    click_at(&mut h, from);
    assert_eq!(h.state().selection, [id]);
    let base = Arc::clone(&h.state().session.model);
    let original = opening(&base, id);
    let guid = base.elements.guid(id);
    let history = h.state().session.history();
    let camera = h.state().orbit.camera.unwrap();
    capture(&mut h, from, to);
    let (trial, scene) = h.state().orbit.scene.as_ref().unwrap();
    let moved = opening(trial, id);
    assert!((moved.offset - 3600.0).abs() <= 1.0 && (moved.sill - 1250.0).abs() <= 1.0, "{moved:?}");
    assert_eq!(moved, Opening { offset: moved.offset, sill: moved.sill, ..original.clone() });
    let image = scene.render(&camera, &[]);
    for (q, expected) in [([2000.0, 0.0, 1500.0], wall), ([3600.0, 0.0, 1850.0], id)] {
        let (x, y, _) = camera.project(q);
        assert_eq!(image.pick(x, y), Some(expected));
    }
    assert!(Arc::ptr_eq(&base, &h.state().session.model));
    assert_eq!(h.state().session.history(), history);
    h.event(button(to, false));
    h.run();
    assert_eq!(opening(&h.state().session.model, id), moved);
    assert_eq!(h.state().orbit.camera, Some(camera));
    assert_eq!(h.state().session.history().len(), history.len() + 1);
    h.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    h.run();
    assert_eq!(opening(&h.state().session.model, id), original);
    press(&mut h, "Redo");
    assert_eq!(opening(&h.state().session.model, id), moved);
    let path = std::env::temp_dir().join(format!("window-drag-{}.bimcraft", std::process::id()));
    assert!(h.state_mut().save_to(&path));
    assert!(h.state_mut().open_path(&path));
    assert_eq!(h.state().session.model.elements.guid(id), guid);
    assert_eq!(opening(&h.state().session.model, id), moved);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn hosted_window_drag_shift_escape_and_invalid_release() {
    let mut h = app(WALL);
    press(&mut h, "Default 3D View");
    let id = ids(&h, "window")[0];
    let at = |h: &Harness<BimApp>, x, z| h.state().screen3([x, 0.0, z]);
    let from = at(&h, 2000.0, 1500.0);
    let to = at(&h, 3300.0, 1900.0);
    click_at(&mut h, from);
    let base = Arc::clone(&h.state().session.model);
    let n = h.state().session.history().len();
    capture(&mut h, from, to);
    h.event(Event::ModifiersChanged(Modifiers::SHIFT));
    h.run();
    let w = opening(&h.state().orbit.scene.as_ref().unwrap().0, id);
    assert_eq!(w.sill, 900.0);
    assert!((w.offset - 3300.0).abs() <= 1.0);
    h.key_press_modifiers(Modifiers::SHIFT, Key::Escape);
    h.run();
    h.event(button(to, false));
    h.run();
    assert!(!h.state().orbit.window_drag_active());
    assert!(Arc::ptr_eq(&base, &h.state().session.model));
    assert_eq!(h.state().selection, [id]);
    h.event(Event::ModifiersChanged(Modifiers::SHIFT));
    capture(&mut h, from, to);
    h.event(Event::PointerButton { pos: to, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::SHIFT });
    h.run();
    assert_eq!(opening(&h.state().session.model, id), w);
    h.event(Event::ModifiersChanged(Modifiers::NONE));
    h.run();
    let now = Arc::clone(&h.state().session.model);
    let from = at(&h, w.offset, 1500.0);
    for to in [at(&h, 6200.0, 1500.0), at(&h, 90.0, 1500.0), at(&h, 4000.0, 400.0)] {
        capture(&mut h, from, to);
        assert!(h.state().status.error, "{}", h.state().status.text);
        h.event(button(to, false));
        h.run();
        assert!(Arc::ptr_eq(&now, &h.state().session.model));
        assert_eq!(h.state().session.history().len(), n + 1);
    }
}
