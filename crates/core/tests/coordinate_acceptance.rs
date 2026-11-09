//! Public-command acceptance for PR 284: native persistence and safety boundaries.

#[cfg(test)]
mod tests {
    use bimcraft_core::engine::Session;
    use bimcraft_core::model::{FORMAT, Model};
    use serde_json::{Value, json};

    fn run(s: &mut Session, command: &str, params: Value) -> Value {
        s.execute(command, &params).unwrap_or_else(|e| panic!("{command}: {e}"))
    }

    #[test]
    fn shared_coordinates_keep_height_rotation_scale_and_identity_after_native_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("synthetic.bimcraft");
        let mut s = Session::new();
        run(&mut s, "wall", json!({"points": [[0, 0], [8000, 0]]}));
        run(&mut s, "crs", json!({"code": 32632}));
        run(
            &mut s,
            "location",
            json!({"name": "Internal", "east": 500_000_000, "north": 5_000_000_000.0,
                "elevation": 42_000, "angle": -30, "scale": 0.9996}),
        );
        let before = run(&mut s, "coordinates", json!({}));
        run(&mut s, "save", json!({"path": path}));
        let mut reopened = Session::new();
        let opened = run(&mut reopened, "open", json!({"path": path}));
        assert!(opened.get("left_out").is_none_or(|v| v.as_array().is_some_and(Vec::is_empty)), "{opened}");
        assert_eq!(before, run(&mut reopened, "coordinates", json!({})));
        let point = [8000.0, 2000.0, 3000.0];
        let r = run(&mut reopened, "convert_coordinates", json!({"points": [point], "from": "internal", "to": "shared"}));
        let a = (-30.0_f64).to_radians();
        let expected = [
            500_000_000.0 + 0.9996 * (8000.0 * a.cos() - 2000.0 * a.sin()),
            5_000_000_000.0 + 0.9996 * (8000.0 * a.sin() + 2000.0 * a.cos()),
            45_000.0,
        ];
        for (i, want) in expected.into_iter().enumerate() {
            assert!((r["points"][0][i].as_f64().unwrap() - want).abs() < 1e-6, "{r}");
        }
        let back = run(&mut reopened, "convert_coordinates", json!({"points": [expected], "from": "shared", "to": "internal"}));
        for (i, want) in point.into_iter().enumerate() {
            assert!((back["points"][0][i].as_f64().unwrap() - want).abs() < 1e-6, "{back}");
        }
    }

    #[test]
    fn coordinate_edits_respect_read_only_and_invalid_scale_rolls_back() {
        let mut s = Session::new();
        let before = run(&mut s, "coordinates", json!({}));
        assert!(s.execute("location", &json!({"name": "Internal", "scale": 0})).is_err());
        assert_eq!(before, run(&mut s, "coordinates", json!({})));
        s.read_only = Some("acceptance read-only session");
        assert!(s.execute("location", &json!({"name": "Internal", "elevation": 42000})).is_err());
        assert_eq!(before, run(&mut s, "coordinates", json!({})));
    }

    #[test]
    fn newer_native_files_are_refused_and_unknown_fields_remain_reported() {
        let mut value: Value = serde_json::from_str(&Model::new("Synthetic").to_json().unwrap()).unwrap();
        value["future_coordinate_field"] = json!(42);
        let (_, left_out) = Model::read(&value.to_string()).unwrap();
        assert_eq!(left_out, ["/future_coordinate_field"]);
        value["format"] = json!(FORMAT + 1);
        assert!(Model::read(&value.to_string()).unwrap_err().contains("newer BIMCraft"));
    }
}
