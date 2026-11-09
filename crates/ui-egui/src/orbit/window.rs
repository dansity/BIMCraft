//! A body drag keeps the grabbed point under the cursor in a vertical plane parallel to the host.
use super::*;
use bimcraft_core::engine::Session;
use bimcraft_core::model::Element;

pub(super) struct WindowDrag {
    pub(super) base: Arc<Model>,
    pub(super) id: Id,
    grab: P3,
    axis: P2,
    offset: f64,
    sill: f64,
    // Cache by placement: idle frames share the same snapshot with the scene worker.
    place: Option<(f64, f64)>,
    preview: Result<Arc<Model>, String>,
}

impl WindowDrag {
    pub(super) fn start(base: &Arc<Model>, hit: Surface, cam: &Camera, pixel: (f64, f64)) -> Option<Self> {
        let Element::Window(w) = base.elements.get(hit.id)? else { return None };
        let wall = base.wall(w.host)?;
        if wall.arc != 0.0 {
            return None;
        }
        let axis = (wall.end - wall.start).unit();
        let drag = Self {
            base: Arc::clone(base),
            id: hit.id,
            grab: hit.at,
            axis,
            offset: w.offset,
            sill: w.sill,
            place: Some((w.offset, w.sill)),
            preview: Ok(Arc::clone(base)),
        };
        drag.point(cam, pixel)?;
        Some(drag)
    }

    fn point(&self, cam: &Camera, pixel: (f64, f64)) -> Option<P3> {
        cam.on_face_plane(pixel, self.grab, [-self.axis.1, self.axis.0, 0.0])
    }

    fn params(&self, (offset, sill): (f64, f64)) -> serde_json::Value {
        json!({"id": self.id, "offset": offset, "sill": sill})
    }

    pub(super) fn update(&mut self, cam: &Camera, pixel: (f64, f64), shift: bool) {
        let place = self.point(cam, pixel).map(|p| {
            let along = P2(p[0] - self.grab[0], p[1] - self.grab[1]).dot(self.axis);
            let rise = if shift { 0.0 } else { (p[2] - self.grab[2]).round() };
            (self.offset + along.round(), self.sill + rise)
        });
        if place == self.place {
            return;
        }
        self.place = place;
        self.preview = match place {
            None => Err(t("The wall is edge-on to this view.").into()),
            Some(p) if p == (self.offset, self.sill) => Ok(Arc::clone(&self.base)),
            Some(p) => {
                let mut trial = Session::new();
                trial.model = Arc::clone(&self.base);
                trial.execute("modify", &self.params(p)).map(|_| trial.model)
            }
        };
    }

    /// Keep the opening visible under the pointer even while a new shaded picture is on the worker.
    pub(super) fn outline(&self, painter: &egui::Painter, screen: impl Fn(P3) -> Pos2) {
        let Some((offset, sill)) = self.place else { return };
        let Some(e @ Element::Window(w)) = self.base.elements.get(self.id) else { return };
        let (Some(size), Some((center, axis, z))) = (self.base.opening_type(e), self.base.opening_frame(w)) else { return };
        let center = center + axis * (offset - w.offset);
        let left = center - axis * (size.width / 2.0);
        let right = center + axis * (size.width / 2.0);
        let bottom = z + sill - w.sill;
        let top = bottom + size.height;
        let ring = [(left, bottom), (right, bottom), (right, top), (left, top)].map(|(xy, z)| screen([xy.0, xy.1, z]));
        if ring.iter().all(|p| p.is_finite()) {
            let ink = if self.preview.is_err() { pal().error } else { pal().accent };
            painter.add(egui::Shape::closed_line(ring.to_vec(), Stroke::new(2.0, ink)));
        }
    }

    pub(super) fn model(&self) -> Result<&Arc<Model>, &String> {
        self.preview.as_ref()
    }

    pub(super) fn commit(&self) -> Option<serde_json::Value> {
        self.preview.as_ref().ok()?;
        self.place.filter(|p| *p != (self.offset, self.sill)).map(|p| self.params(p))
    }
}
