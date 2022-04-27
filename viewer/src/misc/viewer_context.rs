use log_types::{Data, LogId, LogMsg, ObjectPath, ObjectPathComponent, TimeValue};

use crate::{log_db::LogDb, misc::log_db::ObjectTree};

/// Common things needed by many parts of the viewer.
#[derive(Default, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub(crate) struct ViewerContext {
    /// For displaying images effectively.
    #[serde(skip)]
    pub image_cache: crate::misc::ImageCache,

    /// The current time.
    pub time_control: crate::TimeControl,

    /// Currently selected thing, shown in the context menu.
    pub selection: Selection,

    /// Individual settings. Mutate this.
    pub individual_object_properties: ObjectsProperties,

    /// Properties, as inherited from parent.
    /// Read from this.
    ///
    /// Recalculated at the start of each frame form [`Self::object_properties`].
    #[serde(skip)]
    pub projected_object_properties: ObjectsProperties,
}

impl ViewerContext {
    /// Called at the start of each frame
    pub fn on_frame_start(&mut self, log_db: &LogDb) {
        crate::profile_function!();

        fn project_tree(
            context: &mut ViewerContext,
            path: &mut Vec<ObjectPathComponent>,
            prop: ObjectProps,
            tree: &ObjectTree,
        ) {
            // TODO: we need to speed up and simplify this a lot.
            let object_path = ObjectPath(path.clone());
            let prop = prop.with_child(&context.individual_object_properties.get(&object_path));
            context.projected_object_properties.set(object_path, prop);

            for (name, node) in &tree.children {
                path.push(ObjectPathComponent::String(name.clone()));
                project_tree(context, path, prop, &node.string_children);
                path.pop();

                for (id, tree) in &node.persist_id_children {
                    path.push(ObjectPathComponent::PersistId(name.clone(), id.clone()));
                    project_tree(context, path, prop, tree);
                    path.pop();
                }

                for (id, tree) in &node.temp_id_children {
                    path.push(ObjectPathComponent::TempId(name.clone(), id.clone()));
                    project_tree(context, path, prop, tree);
                    path.pop();
                }
            }
        }

        let mut path = vec![];
        project_tree(self, &mut path, ObjectProps::default(), &log_db.object_tree);
    }

    /// Button to select the current space.
    pub fn space_button(&mut self, ui: &mut egui::Ui, space: &ObjectPath) -> egui::Response {
        // TODO: common hover-effect of all buttons for the same space!
        let response = ui.selectable_label(self.selection.is_space(space), space.to_string());
        if response.clicked() {
            self.selection = Selection::Space(space.clone());
        }
        response
    }

    pub fn time_button(
        &mut self,
        ui: &mut egui::Ui,
        time_source: &str,
        value: TimeValue,
    ) -> egui::Response {
        let is_selected = self.time_control.is_time_selected(time_source, value);

        let response = ui.selectable_label(is_selected, value.to_string());
        if response.clicked() {
            self.time_control
                .set_source_and_time(time_source.to_string(), value);
            self.time_control.pause();
        }
        response
    }

    #[allow(clippy::unused_self)]
    pub fn object_color(&self, log_db: &LogDb, msg: &LogMsg) -> egui::Color32 {
        // Try to get the latest color at the time of the message:
        // TODO: pre-compute this to avoid lookups?
        let time_source = self.time_control.source();
        if let Some(time) = msg.time_point.0.get(time_source) {
            let color_path = msg.object_path.sibling("color");
            if let Some(color_msg) = log_db.latest(time_source, *time, &color_path) {
                if let Data::Color([r, g, b, a]) = &color_msg.data {
                    return egui::Color32::from_rgba_unmultiplied(*r, *g, *b, *a);
                } else {
                    tracing::warn!(
                        "Expected color data in {:?}; found {:?}",
                        color_path,
                        color_msg.data
                    );
                }
            }
        }

        use rand::rngs::SmallRng;
        use rand::{Rng, SeedableRng};

        // TODO: ignore `TempId` id:s!
        let mut small_rng = SmallRng::seed_from_u64(egui::util::hash(&msg.object_path));

        // TODO: OKLab
        let hsva = egui::color::Hsva {
            h: small_rng.gen(),
            s: small_rng.gen_range(0.35..=0.55_f32).sqrt(),
            v: small_rng.gen_range(0.55..=0.80_f32).cbrt(),
            a: 1.0,
        };

        hsva.into()
    }
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub(crate) enum Selection {
    None,
    LogId(LogId),
    Space(ObjectPath),
}

impl Default for Selection {
    fn default() -> Self {
        Self::None
    }
}

impl Selection {
    pub fn is_space(&self, needle: &ObjectPath) -> bool {
        if let Self::Space(hay) = self {
            hay == needle
        } else {
            false
        }
    }
}

#[derive(Default, serde::Deserialize, serde::Serialize)]
pub(crate) struct ObjectsProperties {
    props: ahash::AHashMap<ObjectPath, ObjectProps>,
}

impl ObjectsProperties {
    pub fn get(&self, object_path: &ObjectPath) -> ObjectProps {
        self.props.get(object_path).copied().unwrap_or_default()
    }

    pub fn set(&mut self, object_path: ObjectPath, prop: ObjectProps) {
        if prop == ObjectProps::default() {
            self.props.remove(&object_path); // save space
        } else {
            self.props.insert(object_path, prop);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub(crate) struct ObjectProps {
    pub visible: bool,
}

impl Default for ObjectProps {
    fn default() -> Self {
        Self { visible: true }
    }
}

impl ObjectProps {
    /// Multiply/and these together.
    fn with_child(&self, child: &ObjectProps) -> ObjectProps {
        ObjectProps {
            visible: self.visible && child.visible,
        }
    }
}
