use ssbh_data::prelude::*;
use ssbh_wgpu::ShaderDatabase;

// TODO: How to update these only when a file changes?
// TODO: Only validate known names like model.numatb or model.numdlb?
#[derive(Default)]
pub struct ModelValidationErrors {
    pub mesh_errors: Vec<MeshValidationError>,
    pub skel_errors: Vec<SkelValidationError>,
    pub matl_errors: Vec<MatlValidationError>,
    pub modl_errors: Vec<ModlValidationError>,
    pub adj_errors: Vec<AdjValidationError>,
    pub anim_errors: Vec<AnimValidationError>,
    pub hlpb_errors: Vec<HlpbValidationError>,
    pub nutexb_errors: Vec<NutexbValidationError>,
}

pub struct MeshValidationError;
pub struct SkelValidationError;
pub enum MatlValidationError {
    MissingRequiredVertexAttributes {
        entry_index: usize,
        mesh_name: String,
        missing_attributes: Vec<String>,
    },
}

impl MatlValidationError {
    pub fn entry_index(&self) -> usize {
        match self {
            Self::MissingRequiredVertexAttributes { entry_index, .. } => *entry_index,
        }
    }
}
pub struct ModlValidationError;
pub struct AdjValidationError;
pub struct AnimValidationError;
pub struct HlpbValidationError;
pub struct NutexbValidationError;

// TODO: How to incorporate this with the UI?
pub fn validate_matl(
    matl: &MatlData,
    modl: Option<&ModlData>,
    mesh: Option<&MeshData>,
    shader_database: &ShaderDatabase,
) -> Vec<MatlValidationError> {
    match (modl, mesh) {
        (Some(modl), Some(mesh)) => matl
            .entries
            .iter()
            .enumerate()
            .filter_map(|(entry_index, entry)| {
                // TODO: make this a method of the database?
                let program = shader_database.get(entry.shader_label.get(..24).unwrap_or(""))?;

                Some(
                    mesh.objects
                        .iter()
                        .filter(|o| {
                            modl.entries
                                .iter()
                                .filter(|e| e.material_label == entry.material_label)
                                .any(|e| {
                                    e.mesh_object_name == o.name
                                        && e.mesh_object_sub_index == o.sub_index
                                })
                        })
                        .filter_map(move |o| {
                            // TODO: check for missing attributes
                            let attribute_names: Vec<_> = o
                                .texture_coordinates
                                .iter()
                                .map(|a| a.name.to_string())
                                .chain(o.color_sets.iter().map(|a| a.name.to_string()))
                                .collect();

                            let missing_attributes =
                                program.missing_required_attributes(&attribute_names);
                            if !missing_attributes.is_empty() {
                                // TODO: Make this a function.
                                Some(MatlValidationError::MissingRequiredVertexAttributes {
                                    entry_index,
                                    mesh_name: o.name.clone(),
                                    missing_attributes,
                                })
                            } else {
                                None
                            }
                        }),
                )
            })
            .flatten()
            .collect(),
        _ => Vec::new(),
    }
}
