use crate::material::{
    default_texture, is_blend, is_bool, is_float, is_rasterizer, is_sampler, is_texture, is_vector,
};
use anyhow::anyhow;
use ssbh_data::{matl_data::*, Vector4};
use std::str::FromStr;
use xmltree::{Element, XMLNode};

fn default_texture_param(param: ParamId) -> TextureParam {
    TextureParam::new(param, default_texture(param).to_string())
}

pub fn default_presets() -> Vec<MatlEntryData> {
    vec![
        MatlEntryData {
            material_label: "PRM Opaque".into(),
            shader_label: "SFX_PBS_0100000008008269_opaque".into(),
            blend_states: vec![ParamData::new(ParamId::BlendState0, Default::default())],
            floats: vec![ParamData::new(ParamId::CustomFloat8, 0.4)],
            booleans: vec![
                ParamData::new(ParamId::CustomBoolean1, true),
                ParamData::new(ParamId::CustomBoolean3, true),
                ParamData::new(ParamId::CustomBoolean4, true),
            ],
            vectors: vec![
                ParamData::new(ParamId::CustomVector0, Vector4::new(1.0, 0.0, 0.0, 0.0)),
                ParamData::new(ParamId::CustomVector13, Vector4::new(1.0, 1.0, 1.0, 1.0)),
                ParamData::new(ParamId::CustomVector14, Vector4::new(1.0, 1.0, 1.0, 1.0)),
                ParamData::new(ParamId::CustomVector8, Vector4::new(1.0, 1.0, 1.0, 1.0)),
            ],
            rasterizer_states: vec![ParamData::new(
                ParamId::RasterizerState0,
                Default::default(),
            )],
            samplers: vec![
                ParamData::new(ParamId::Sampler0, Default::default()),
                ParamData::new(ParamId::Sampler4, Default::default()),
                ParamData::new(ParamId::Sampler6, Default::default()),
                ParamData::new(ParamId::Sampler7, Default::default()),
            ],
            textures: vec![
                default_texture_param(ParamId::Texture0),
                default_texture_param(ParamId::Texture4),
                default_texture_param(ParamId::Texture6),
                default_texture_param(ParamId::Texture7),
            ],
        },
        MatlEntryData {
            material_label: "PRM Skin Opaque".into(),
            shader_label: "SFX_PBS_010000000800826b_opaque".into(),
            blend_states: vec![ParamData::new(ParamId::BlendState0, Default::default())],
            floats: vec![ParamData::new(ParamId::CustomFloat8, 0.7)],
            booleans: vec![
                ParamData::new(ParamId::CustomBoolean1, true),
                ParamData::new(ParamId::CustomBoolean3, true),
                ParamData::new(ParamId::CustomBoolean4, true),
            ],
            vectors: vec![
                ParamData::new(ParamId::CustomVector0, Vector4::new(0.0, 0.0, 0.0, 0.0)),
                ParamData::new(
                    ParamId::CustomVector11,
                    Vector4::new(0.25, 0.03333333, 0.0, 1.0),
                ),
                ParamData::new(ParamId::CustomVector13, Vector4::new(1.0, 1.0, 1.0, 1.0)),
                ParamData::new(ParamId::CustomVector14, Vector4::new(1.0, 1.0, 1.0, 1.0)),
                ParamData::new(ParamId::CustomVector30, Vector4::new(0.5, 1.5, 1.0, 1.0)),
                ParamData::new(ParamId::CustomVector8, Vector4::new(1.0, 1.0, 1.0, 1.0)),
            ],
            rasterizer_states: vec![ParamData::new(
                ParamId::RasterizerState0,
                Default::default(),
            )],
            samplers: vec![
                ParamData::new(ParamId::Sampler0, Default::default()),
                ParamData::new(ParamId::Sampler4, Default::default()),
                ParamData::new(ParamId::Sampler6, Default::default()),
                ParamData::new(ParamId::Sampler7, Default::default()),
            ],
            textures: vec![
                default_texture_param(ParamId::Texture0),
                default_texture_param(ParamId::Texture4),
                default_texture_param(ParamId::Texture6),
                default_texture_param(ParamId::Texture7),
            ],
        },
        MatlEntryData {
            material_label: "PRM Alpha Blend".into(),
            shader_label: "SFX_PBS_0100000008018269_sort".into(),
            blend_states: vec![ParamData::new(
                ParamId::BlendState0,
                BlendStateData {
                    source_color: BlendFactor::One,
                    destination_color: BlendFactor::OneMinusSourceAlpha,
                    alpha_sample_to_coverage: false,
                },
            )],
            floats: vec![ParamData::new(ParamId::CustomFloat8, 0.4)],
            booleans: vec![
                ParamData::new(ParamId::CustomBoolean1, true),
                ParamData::new(ParamId::CustomBoolean2, false),
                ParamData::new(ParamId::CustomBoolean3, true),
                ParamData::new(ParamId::CustomBoolean4, true),
            ],
            vectors: vec![
                ParamData::new(ParamId::CustomVector0, Vector4::new(0.0, 0.0, 0.0, 0.0)),
                ParamData::new(ParamId::CustomVector13, Vector4::new(1.0, 1.0, 1.0, 1.0)),
                ParamData::new(ParamId::CustomVector14, Vector4::new(1.0, 1.0, 1.0, 1.0)),
                ParamData::new(ParamId::CustomVector8, Vector4::new(1.0, 1.0, 1.0, 1.0)),
            ],
            rasterizer_states: vec![ParamData::new(
                ParamId::RasterizerState0,
                Default::default(),
            )],
            samplers: vec![
                ParamData::new(ParamId::Sampler0, Default::default()),
                ParamData::new(ParamId::Sampler4, Default::default()),
                ParamData::new(ParamId::Sampler6, Default::default()),
                ParamData::new(ParamId::Sampler7, Default::default()),
            ],
            textures: vec![
                default_texture_param(ParamId::Texture0),
                default_texture_param(ParamId::Texture4),
                default_texture_param(ParamId::Texture6),
                default_texture_param(ParamId::Texture7),
            ],
        },
        MatlEntryData {
            material_label: "PRM Alpha Test".into(),
            shader_label: "SFX_PBS_01000000080c8269_opaque".into(),
            blend_states: vec![ParamData::new(ParamId::BlendState0, Default::default())],
            floats: vec![ParamData::new(ParamId::CustomFloat8, 0.4)],
            booleans: vec![
                ParamData::new(ParamId::CustomBoolean1, true),
                ParamData::new(ParamId::CustomBoolean3, true),
                ParamData::new(ParamId::CustomBoolean4, true),
            ],
            vectors: vec![
                ParamData::new(ParamId::CustomVector0, Vector4::new(0.0, 0.0, 0.0, 0.0)),
                ParamData::new(ParamId::CustomVector13, Vector4::new(1.0, 1.0, 1.0, 1.0)),
                ParamData::new(ParamId::CustomVector14, Vector4::new(1.0, 1.0, 1.0, 1.0)),
                ParamData::new(ParamId::CustomVector8, Vector4::new(1.0, 1.0, 1.0, 1.0)),
            ],
            rasterizer_states: vec![ParamData::new(
                ParamId::RasterizerState0,
                Default::default(),
            )],
            samplers: vec![
                ParamData::new(ParamId::Sampler0, Default::default()),
                ParamData::new(ParamId::Sampler4, Default::default()),
                ParamData::new(ParamId::Sampler6, Default::default()),
                ParamData::new(ParamId::Sampler7, Default::default()),
            ],
            textures: vec![
                default_texture_param(ParamId::Texture0),
                default_texture_param(ParamId::Texture4),
                default_texture_param(ParamId::Texture6),
                default_texture_param(ParamId::Texture7),
            ],
        },
        MatlEntryData {
            material_label: "PRM Anisotropic Alpha Test".into(),
            shader_label: "SFX_PBS_0100080008048269_opaque".into(),
            blend_states: vec![ParamData::new(
                ParamId::BlendState0,
                BlendStateData {
                    source_color: BlendFactor::One,
                    destination_color: BlendFactor::Zero,
                    alpha_sample_to_coverage: true,
                },
            )],
            floats: vec![
                ParamData::new(ParamId::CustomFloat10, 0.6),
                ParamData::new(ParamId::CustomFloat8, 0.7),
            ],
            booleans: vec![
                ParamData::new(ParamId::CustomBoolean1, true),
                ParamData::new(ParamId::CustomBoolean3, true),
                ParamData::new(ParamId::CustomBoolean4, true),
            ],
            vectors: vec![
                ParamData::new(ParamId::CustomVector0, Vector4::new(0.0, 0.0, 0.0, 0.0)),
                ParamData::new(ParamId::CustomVector13, Vector4::new(1.0, 1.0, 1.0, 1.0)),
                ParamData::new(ParamId::CustomVector14, Vector4::new(0.75, 0.75, 0.75, 1.0)),
                ParamData::new(ParamId::CustomVector8, Vector4::new(1.0, 1.0, 1.0, 1.0)),
            ],
            rasterizer_states: vec![ParamData::new(
                ParamId::RasterizerState0,
                Default::default(),
            )],
            samplers: vec![
                ParamData::new(ParamId::Sampler0, Default::default()),
                ParamData::new(ParamId::Sampler4, Default::default()),
                ParamData::new(ParamId::Sampler6, Default::default()),
                ParamData::new(ParamId::Sampler7, Default::default()),
            ],
            textures: vec![
                default_texture_param(ParamId::Texture0),
                default_texture_param(ParamId::Texture4),
                default_texture_param(ParamId::Texture6),
                default_texture_param(ParamId::Texture7),
            ],
        },
        MatlEntryData {
            material_label: "PRM Emi Opaque".into(),
            shader_label: "SFX_PBS_010000080a008269_opaque".into(),
            blend_states: vec![ParamData::new(ParamId::BlendState0, Default::default())],
            floats: vec![ParamData::new(ParamId::CustomFloat8, 0.4)],
            booleans: vec![
                ParamData::new(ParamId::CustomBoolean1, true),
                ParamData::new(ParamId::CustomBoolean3, true),
                ParamData::new(ParamId::CustomBoolean4, true),
                ParamData::new(ParamId::CustomBoolean5, true),
            ],
            vectors: vec![
                ParamData::new(ParamId::CustomVector0, Vector4::new(1.0, 0.0, 0.0, 0.0)),
                ParamData::new(ParamId::CustomVector13, Vector4::new(1.0, 1.0, 1.0, 1.0)),
                ParamData::new(ParamId::CustomVector14, Vector4::new(0.75, 0.75, 0.75, 1.0)),
                ParamData::new(ParamId::CustomVector29, Vector4::new(0.0, 0.0, 0.0, 0.0)),
                ParamData::new(ParamId::CustomVector3, Vector4::new(1.0, 1.0, 1.0, 1.0)),
                ParamData::new(ParamId::CustomVector6, Vector4::new(1.0, 1.0, 0.0, 0.0)),
                ParamData::new(ParamId::CustomVector8, Vector4::new(1.0, 1.0, 1.0, 1.0)),
            ],
            rasterizer_states: vec![ParamData::new(
                ParamId::RasterizerState0,
                Default::default(),
            )],
            samplers: vec![
                ParamData::new(ParamId::Sampler0, Default::default()),
                ParamData::new(ParamId::Sampler4, Default::default()),
                ParamData::new(ParamId::Sampler5, Default::default()),
                ParamData::new(ParamId::Sampler6, Default::default()),
                ParamData::new(ParamId::Sampler7, Default::default()),
            ],
            textures: vec![
                default_texture_param(ParamId::Texture0),
                default_texture_param(ParamId::Texture4),
                default_texture_param(ParamId::Texture5),
                default_texture_param(ParamId::Texture6),
                default_texture_param(ParamId::Texture7),
            ],
        },
        MatlEntryData {
            material_label: "Glass Angle Fade".into(),
            shader_label: "SFX_PBS_0100000008018279_sort".into(),
            blend_states: vec![ParamData::new(
                ParamId::BlendState0,
                BlendStateData {
                    source_color: BlendFactor::One,
                    destination_color: BlendFactor::OneMinusSourceAlpha,
                    alpha_sample_to_coverage: false,
                },
            )],
            floats: vec![
                ParamData::new(ParamId::CustomFloat19, 1.2),
                ParamData::new(ParamId::CustomFloat8, 1.5),
            ],
            booleans: vec![
                ParamData::new(ParamId::CustomBoolean1, true),
                ParamData::new(ParamId::CustomBoolean2, true),
                ParamData::new(ParamId::CustomBoolean3, true),
                ParamData::new(ParamId::CustomBoolean4, true),
            ],
            vectors: vec![
                ParamData::new(ParamId::CustomVector0, Vector4::new(0.0, 0.0, 0.0, 0.0)),
                ParamData::new(ParamId::CustomVector13, Vector4::new(1.0, 1.0, 1.0, 1.0)),
                ParamData::new(
                    ParamId::CustomVector14,
                    Vector4::new(4.618421, 4.618421, 4.618421, 1.0),
                ),
                ParamData::new(ParamId::CustomVector8, Vector4::new(1.0, 1.0, 1.0, 1.0)),
            ],
            rasterizer_states: vec![ParamData::new(
                ParamId::RasterizerState0,
                Default::default(),
            )],
            samplers: vec![
                ParamData::new(ParamId::Sampler0, Default::default()),
                ParamData::new(ParamId::Sampler4, Default::default()),
                ParamData::new(ParamId::Sampler6, Default::default()),
                ParamData::new(ParamId::Sampler7, Default::default()),
            ],
            textures: vec![
                default_texture_param(ParamId::Texture0),
                default_texture_param(ParamId::Texture4),
                default_texture_param(ParamId::Texture6),
                default_texture_param(ParamId::Texture7),
            ],
        },
        MatlEntryData {
            material_label: "Emi Nor Shadeless".into(),
            shader_label: "SFX_PBS_2f00000002014248_opaque".into(),
            blend_states: vec![ParamData::new(
                ParamId::BlendState0,
                BlendStateData {
                    source_color: BlendFactor::One,
                    destination_color: BlendFactor::Zero,
                    alpha_sample_to_coverage: false,
                },
            )],
            floats: vec![],
            booleans: vec![
                ParamData::new(ParamId::CustomBoolean1, true),
                ParamData::new(ParamId::CustomBoolean2, true),
                ParamData::new(ParamId::CustomBoolean3, true),
                ParamData::new(ParamId::CustomBoolean4, true),
            ],
            vectors: vec![
                ParamData::new(ParamId::CustomVector0, Vector4::new(0.0, 0.0, 0.0, 0.0)),
                ParamData::new(ParamId::CustomVector13, Vector4::new(1.0, 1.0, 1.0, 1.0)),
                ParamData::new(ParamId::CustomVector3, Vector4::new(1.0, 1.0, 1.0, 1.0)),
                ParamData::new(ParamId::CustomVector8, Vector4::new(1.0, 1.0, 1.0, 1.0)),
            ],
            rasterizer_states: vec![ParamData::new(
                ParamId::RasterizerState0,
                Default::default(),
            )],
            samplers: vec![
                ParamData::new(ParamId::Sampler4, Default::default()),
                ParamData::new(ParamId::Sampler5, Default::default()),
                ParamData::new(ParamId::Sampler7, Default::default()),
            ],
            textures: vec![
                default_texture_param(ParamId::Texture4),
                default_texture_param(ParamId::Texture5),
                default_texture_param(ParamId::Texture7),
            ],
        },
        MatlEntryData {
            material_label: "Emi Shadeless".into(),
            shader_label: "SFX_PBS_0000000000080100_opaque".into(),
            blend_states: vec![ParamData::new(ParamId::BlendState0, Default::default())],
            floats: vec![],
            booleans: vec![],
            vectors: vec![
                ParamData::new(ParamId::CustomVector3, Vector4::new(1.0, 1.0, 1.0, 1.0)),
                ParamData::new(ParamId::CustomVector8, Vector4::new(1.0, 1.0, 1.0, 1.0)),
            ],
            rasterizer_states: vec![ParamData::new(
                ParamId::RasterizerState0,
                Default::default(),
            )],
            samplers: vec![ParamData::new(ParamId::Sampler5, Default::default())],
            textures: vec![ParamData::new(
                ParamId::Texture5,
                "/common/shader/sfxpbs/default_black".into(),
            )],
        },
        MatlEntryData {
            material_label: "CustomVector47 PRM Params".into(),
            shader_label: "SFX_PBS_010000000808ba68_opaque".into(),
            blend_states: vec![ParamData::new(ParamId::BlendState0, Default::default())],
            floats: vec![],
            booleans: vec![
                ParamData::new(ParamId::CustomBoolean1, true),
                ParamData::new(ParamId::CustomBoolean3, true),
                ParamData::new(ParamId::CustomBoolean4, true),
            ],
            vectors: vec![
                ParamData::new(ParamId::CustomVector0, Vector4::new(0.0, 0.0, 0.0, 0.0)),
                ParamData::new(ParamId::CustomVector13, Vector4::new(1.0, 1.0, 1.0, 1.0)),
                ParamData::new(ParamId::CustomVector14, Vector4::new(1.0, 1.0, 1.0, 1.0)),
                ParamData::new(ParamId::CustomVector47, Vector4::new(0.0, 0.5, 1.0, 0.16)),
                ParamData::new(ParamId::CustomVector8, Vector4::new(1.0, 1.0, 1.0, 1.0)),
            ],
            rasterizer_states: vec![ParamData::new(
                ParamId::RasterizerState0,
                Default::default(),
            )],
            samplers: vec![
                ParamData::new(ParamId::Sampler0, Default::default()),
                ParamData::new(ParamId::Sampler4, Default::default()),
                ParamData::new(ParamId::Sampler7, Default::default()),
            ],
            textures: vec![
                default_texture_param(ParamId::Texture0),
                default_texture_param(ParamId::Texture4),
                default_texture_param(ParamId::Texture7),
            ],
        },
        MatlEntryData {
            material_label: "Diffuse Cube Map".into(),
            shader_label: "SFX_PBS_0d00000000000000_opaque".into(),
            blend_states: vec![ParamData::new(ParamId::BlendState0, Default::default())],
            floats: vec![],
            booleans: vec![],
            vectors: vec![ParamData::new(
                ParamId::CustomVector8,
                Vector4::new(1.0, 1.0, 1.0, 1.0),
            )],
            rasterizer_states: vec![ParamData::new(
                ParamId::RasterizerState0,
                Default::default(),
            )],
            samplers: vec![ParamData::new(ParamId::Sampler8, Default::default())],
            textures: vec![default_texture_param(ParamId::Texture8)],
        },
    ]
}

pub fn load_json_presets(json: &[u8]) -> anyhow::Result<Vec<MatlEntryData>> {
    serde_json::from_slice(json)
        .map(|matl: MatlData| matl.entries)
        .map_err(Into::into)
}

fn first_child(node: &Element) -> anyhow::Result<&Element> {
    node.children
        .get(0)
        .and_then(XMLNode::as_element)
        .ok_or_else(|| anyhow!("XML node {} has no children.", node.name))
}

fn attribute(node: &Element, name: &str) -> anyhow::Result<String> {
    node.attributes
        .get(name)
        .ok_or_else(|| anyhow!("Node {} has no attribute {:?}.", node.name, name))
        .cloned()
}

pub fn load_xml_presets(xml_text: &[u8]) -> anyhow::Result<Vec<MatlEntryData>> {
    let element = Element::parse(xml_text)?;
    if element.name != "MaterialLibrary" {
        return Err(anyhow!(
            "Unexpected first element. Expected \"MaterialLibrary\" but found {:?}",
            element.name
        ));
    }

    element
        .children
        .iter()
        .filter_map(XMLNode::as_element)
        .map(|node| {
            let mut blend_states = Vec::new();
            let mut floats = Vec::new();
            let mut booleans = Vec::new();
            let mut vectors = Vec::new();
            let mut rasterizer_states = Vec::new();
            let mut samplers = Vec::new();
            let mut textures = Vec::new();

            for param_node in node.children.iter().filter_map(XMLNode::as_element) {
                let param_id = ParamId::from_str(&attribute(param_node, "name")?)?;

                if is_blend(param_id) {
                    let child_node = first_child(param_node)?;
                    let data = BlendStateData {
                        source_color: parse_xml_text(child_node, 0)?,
                        destination_color: parse_xml_text(child_node, 2)?,
                        alpha_sample_to_coverage: parse_xml_text::<usize>(child_node, 6)? != 0,
                    };
                    blend_states.push(ParamData::new(param_id, data))
                } else if is_bool(param_id) {
                    let data = parse_xml_text(param_node, 0)?;
                    booleans.push(ParamData::new(param_id, data));
                } else if is_float(param_id) {
                    let data = parse_xml_text(param_node, 0)?;
                    floats.push(ParamData::new(param_id, data));
                } else if is_vector(param_id) {
                    let child_node = first_child(param_node)?;
                    let x = parse_xml_text(child_node, 0)?;
                    let y = parse_xml_text(child_node, 1)?;
                    let z = parse_xml_text(child_node, 2)?;
                    let w = parse_xml_text(child_node, 3)?;
                    let data = Vector4::new(x, y, z, w);
                    vectors.push(ParamData::new(param_id, data));
                } else if is_rasterizer(param_id) {
                    let child_node = first_child(param_node)?;
                    let data = RasterizerStateData {
                        fill_mode: parse_xml_text(child_node, 0)?,
                        cull_mode: parse_xml_text(child_node, 1)?,
                        depth_bias: parse_xml_text(child_node, 2)?,
                    };
                    rasterizer_states.push(ParamData::new(param_id, data));
                } else if is_sampler(param_id) {
                    samplers.push(ParamData::new(param_id, Default::default()));
                } else if is_texture(param_id) {
                    textures.push(default_texture_param(param_id));
                }
            }

            Ok(MatlEntryData {
                material_label: attribute(node, "materialLabel")?,
                shader_label: attribute(node, "shaderLabel")?,
                blend_states,
                floats,
                booleans,
                vectors,
                rasterizer_states,
                samplers,
                textures,
            })
        })
        .collect()
}

fn parse_xml_text<T: FromStr>(node: &Element, index: usize) -> anyhow::Result<T>
where
    <T as FromStr>::Err: std::error::Error + Send + Sync + 'static,
{
    node.children
        .get(index)
        .and_then(XMLNode::as_element)
        .ok_or_else(|| anyhow!("Node {} is missing child at index {}", node.name, index))?
        .get_text()
        .ok_or_else(|| anyhow!("Node {} has no inner text.", node.name))?
        .parse()
        .map_err(Into::into)
}
