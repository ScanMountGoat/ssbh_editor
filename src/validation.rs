use std::fmt::Display;

use ssbh_data::prelude::*;
use ssbh_wgpu::{ModelFolder, ShaderDatabase};

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

impl ModelValidationErrors {
    pub fn from_model(model: &ModelFolder, shader_database: &ShaderDatabase) -> Self {
        let mut validation = Self::default();

        // Each validation check may add errors to multiple related files.
        if let Some(matl) = model.find_matl() {
            validate_required_attributes(
                &mut validation,
                matl,
                model.find_modl(),
                model.find_mesh(),
                shader_database,
            );
        }
        validation
    }
}

pub struct MeshValidationError;
impl Display for MeshValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
}

pub struct SkelValidationError;
impl Display for SkelValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum MatlValidationError {
    MissingRequiredVertexAttributes {
        entry_index: usize,
        material_label: String,
        mesh_name: String,
        missing_attributes: Vec<String>,
    },
}

impl Display for MatlValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MatlValidationError::MissingRequiredVertexAttributes {
                material_label,
                mesh_name,
                missing_attributes,
                ..
            } => write!(
                f,
                "Mesh {} is missing {} attributes required by assigned material {}",
                mesh_name,
                missing_attributes.len(),
                material_label
            ),
        }
    }
}

impl MatlValidationError {
    pub fn entry_index(&self) -> usize {
        // Use the index to associate errors to entries.
        // The material label in user created files isn't always unique.
        match self {
            Self::MissingRequiredVertexAttributes { entry_index, .. } => *entry_index,
        }
    }
}

pub struct ModlValidationError;
impl Display for ModlValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
}

pub struct AdjValidationError;
impl Display for AdjValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
}

pub struct AnimValidationError;
impl Display for AnimValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
}

pub struct HlpbValidationError;
impl Display for HlpbValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
}

pub struct NutexbValidationError;
impl Display for NutexbValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
}

fn validate_required_attributes(
    validation: &mut ModelValidationErrors,
    matl: &MatlData,
    modl: Option<&ModlData>,
    mesh: Option<&MeshData>,
    shader_database: &ShaderDatabase,
) {
    // Both the modl and mesh should be present to determine material assignments.
    match (modl, mesh) {
        (Some(modl), Some(mesh)) => {
            for (entry_index, entry) in matl.entries.iter().enumerate() {
                // TODO: make this a method of the database?
                if let Some(program) =
                    shader_database.get(entry.shader_label.get(..24).unwrap_or(""))
                {
                    for o in mesh.objects.iter().filter(|o| {
                        modl.entries
                            .iter()
                            .filter(|e| e.material_label == entry.material_label)
                            .any(|e| {
                                e.mesh_object_name == o.name
                                    && e.mesh_object_sub_index == o.sub_index
                            })
                    }) {
                        // Find attributes required by the shader not present in the mesh.
                        let attribute_names: Vec<_> = o
                            .texture_coordinates
                            .iter()
                            .map(|a| a.name.to_string())
                            .chain(o.color_sets.iter().map(|a| a.name.to_string()))
                            .collect();

                        let missing_attributes =
                            program.missing_required_attributes(&attribute_names);
                        if !missing_attributes.is_empty() {
                            let error = MatlValidationError::MissingRequiredVertexAttributes {
                                entry_index,
                                material_label: entry.material_label.clone(),
                                mesh_name: o.name.clone(),
                                missing_attributes,
                            };

                            validation.matl_errors.push(error);
                        }
                    }
                }
            }
        }
        _ => (),
    }
}

#[cfg(test)]
mod tests {
    use ssbh_data::{
        matl_data::MatlEntryData, mesh_data::MeshObjectData, modl_data::ModlEntryData,
    };
    use ssbh_wgpu::create_database;

    use super::*;

    #[test]
    fn required_attributes_all_missing() {
        let shader_database = create_database();
        let matl = MatlData {
            major_version: 1,
            minor_version: 6,
            entries: vec![MatlEntryData {
                material_label: "a".to_string(),
                shader_label: "SFX_PBS_010002000800824f_opaque".to_string(),
                blend_states: Vec::new(),
                floats: Vec::new(),
                booleans: Vec::new(),
                vectors: Vec::new(),
                rasterizer_states: Vec::new(),
                samplers: Vec::new(),
                textures: Vec::new(),
            }],
        };
        let mesh = MeshData {
            major_version: 1,
            minor_version: 10,
            objects: vec![MeshObjectData {
                name: "object1".to_string(),
                sub_index: 0,
                ..Default::default()
            }],
        };
        let modl = ModlData {
            major_version: 1,
            minor_version: 0,
            model_name: String::new(),
            skeleton_file_name: String::new(),
            material_file_names: Vec::new(),
            animation_file_name: None,
            mesh_file_name: String::new(),
            entries: vec![ModlEntryData {
                mesh_object_name: "object1".to_owned(),
                mesh_object_sub_index: 0,
                material_label: "a".to_string(),
            }],
        };

        let mut validation = ModelValidationErrors::default();
        validate_required_attributes(
            &mut validation,
            &matl,
            Some(&modl),
            Some(&mesh),
            &shader_database,
        );

        // TODO: Add a mesh error as well?
        assert_eq!(
            vec![MatlValidationError::MissingRequiredVertexAttributes {
                entry_index: 0,
                material_label: "a".to_string(),
                mesh_name: "object1".to_string(),
                missing_attributes: vec!["map1".to_string(), "uvSet".to_string()]
            }],
            validation.matl_errors
        );
    }
}
