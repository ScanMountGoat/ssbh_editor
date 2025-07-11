use ssbh_wgpu::{animation::camera::animate_camera, CameraTransforms, SsbhRenderer};

use crate::{CameraState, CameraValues, RenderState};

use super::SsbhApp;

impl SsbhApp {
    pub fn refresh_render_state(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        render_state: &mut RenderState,
        width: f32,
        height: f32,
        scale_factor: f64,
    ) {
        // TODO: Load models on a separate thread to avoid freezing the UI.
        // TODO: Just take the entire app as a parameter?
        render_state.update(
            device,
            queue,
            &self.models,
            &mut self.render_actions,
            &self.ui_state.stage_lighting,
            &self.camera_state,
            self.preferences.autohide_expressions,
            self.preferences.autohide_ink_meshes,
            self.animation_state.current_frame,
            self.preferences.viewport_color,
        );

        // TODO: Does this need to be updated every frame?
        update_camera(
            queue,
            &mut render_state.renderer,
            &mut self.camera_state,
            width,
            height,
            scale_factor,
        );

        if self.should_validate_models {
            // Folders can be validated independently from one another.
            for model in &mut self.models {
                model.validate(&render_state.shared_data)
            }
            self.should_validate_models = false;
        }

        if self.swing_state.should_update_swing {
            for ((render_model, prc_index), model) in render_state
                .render_models
                .iter_mut()
                .zip(self.swing_state.selected_swing_folders.iter())
                .zip(self.models.iter())
            {
                if let Some(swing_prc) = prc_index
                    .and_then(|prc_index| self.models.get(prc_index))
                    .and_then(|m| m.swing_prc.as_ref())
                {
                    render_model.recreate_swing_collisions(
                        device,
                        swing_prc,
                        model.model.find_skel(),
                    );
                }
            }
            self.swing_state.should_update_swing = false;
        }

        if self.animation_state.is_playing || self.animation_state.should_update_animations {
            render_state.animate_lighting(queue, self.animation_state.current_frame);
            self.animate_viewport_camera(render_state, queue, width, height, scale_factor);
            self.animate_models(queue, render_state);
            self.animation_state.should_update_animations = false;
        }
    }

    fn animate_viewport_camera(
        &mut self,
        render_state: &mut RenderState,
        queue: &wgpu::Queue,
        width: f32,
        height: f32,
        scale_factor: f64,
    ) {
        if let Some(anim) = &render_state.camera_anim {
            if let Some(values) = animate_camera(
                anim,
                self.animation_state.current_frame,
                self.camera_state.values.fov_y_radians,
                self.camera_state.values.near_clip,
                self.camera_state.values.far_clip,
            ) {
                let transforms = values.to_transforms(width as u32, height as u32, scale_factor);
                render_state.renderer.update_camera(queue, transforms);

                // Apply the animation values to the viewport camera.
                // This reduces "snapping" when moving the camera while paused.
                // These changes won't take effect unless the user actually moves the camera.
                // Decomposition is necessary to account for different transform orders.
                let (_, r, t) = transforms.model_view_matrix.to_scale_rotation_translation();
                self.camera_state.values.translation = t;
                self.camera_state.values.rotation_radians = r.to_euler(glam::EulerRot::XYZ).into();
                self.camera_state.values.fov_y_radians = values.fov_y_radians;
                self.camera_state.mvp_matrix = transforms.mvp_matrix;
            }
        }
    }

    pub fn animate_models(&mut self, queue: &wgpu::Queue, render_state: &mut RenderState) {
        for ((render_model, model), model_animations) in render_state
            .render_models
            .iter_mut()
            .zip(self.models.iter())
            .zip(self.animation_state.animations.iter())
        {
            // Only render enabled animations.
            let animations = model_animations
                .iter()
                .filter(|anim_slot| anim_slot.is_enabled)
                .filter_map(|anim_slot| {
                    anim_slot
                        .animation
                        .and_then(|anim_index| anim_index.get_animation(&self.models))
                        .and_then(|(_, a)| a.as_ref())
                });

            render_model.apply_anims(
                queue,
                animations,
                model
                    .model
                    .skels
                    .iter()
                    .find(|(f, _)| f == "model.nusktb")
                    .and_then(|(_, m)| m.as_ref()),
                model
                    .model
                    .matls
                    .iter()
                    .find(|(f, _)| f == "model.numatb")
                    .and_then(|(_, m)| m.as_ref()),
                if self.enable_helper_bones {
                    model
                        .model
                        .hlpbs
                        .iter()
                        .find(|(f, _)| f == "model.nuhlpb")
                        .and_then(|(_, m)| m.as_ref())
                } else {
                    None
                },
                &render_state.shared_data,
                self.animation_state.current_frame,
            );
        }
    }
}

fn update_camera(
    queue: &wgpu::Queue,
    renderer: &mut SsbhRenderer,
    camera_state: &mut CameraState,
    width: f32,
    height: f32,
    scale_factor: f64,
) {
    let (camera_pos, model_view_matrix, projection_matrix, mvp_matrix) =
        calculate_mvp(width, height, &camera_state.values);
    let transforms = CameraTransforms {
        model_view_matrix,
        mvp_matrix,
        projection_matrix,
        mvp_inv_matrix: mvp_matrix.inverse(),
        camera_pos,
        screen_dimensions: glam::Vec4::new(width, height, scale_factor as f32, 0.0),
    };
    renderer.update_camera(queue, transforms);

    // Needed for bone name rendering.
    camera_state.mvp_matrix = mvp_matrix;
}

// TODO: Separate module for camera + input handling?
pub fn calculate_mvp(
    width: f32,
    height: f32,
    camera_values: &CameraValues,
) -> (glam::Vec4, glam::Mat4, glam::Mat4, glam::Mat4) {
    let aspect = width / height;

    let rotation = glam::Mat4::from_euler(
        glam::EulerRot::XYZ,
        camera_values.rotation_radians.x,
        camera_values.rotation_radians.y,
        camera_values.rotation_radians.z,
    );
    let model_view_matrix = glam::Mat4::from_translation(camera_values.translation) * rotation;
    let projection_matrix = glam::Mat4::perspective_rh(
        camera_values.fov_y_radians,
        aspect,
        camera_values.near_clip,
        camera_values.far_clip,
    );

    let camera_pos = model_view_matrix.inverse().col(3);

    (
        camera_pos,
        model_view_matrix,
        projection_matrix,
        projection_matrix * model_view_matrix,
    )
}
