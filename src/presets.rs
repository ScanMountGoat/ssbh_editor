use crate::material::default_texture;
use ssbh_data::matl_data::*;

fn default_texture_param(param: ParamId) -> TextureParam {
    TextureParam::new(param, default_texture(param).to_string())
}

pub fn default_presets() -> Vec<MatlEntryData> {
    vec![
        MatlEntryData {
            material_label: "PRM Standard (Mario)".into(),
            shader_label: "SFX_PBS_0100000008008269_opaque".into(),
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
            material_label: "PRM Emi Standard (Samus)".into(),
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
                ParamData::new(ParamId::CustomVector0, Vector4::new(0.0, 0.0, 0.0, 0.0)),
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
            material_label: "Glass (Olimar Helmet)".into(),
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
            material_label: "Skin Standard (Mario)".into(),
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
            material_label: "Emission Shadeless (Mario Past USA)".into(),
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
            material_label: "CustomVector47 No PRM (Dedede)".into(),
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
            material_label: "Diffuse Cube Map (Rosalina)".into(),
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
        MatlEntryData {
            material_label: "Hair Anisotropic (Corrin)".into(),
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
            material_label: "Flat Shading (Mr. Game and Watch)".into(),
            shader_label: "SFX_PBS_2f00000002014248_opaque".into(),
            blend_states: vec![ParamData::new(
                ParamId::BlendState0,
                BlendStateData {
                    source_color: BlendFactor::One,
                    destination_color: BlendFactor::OneMinusSourceAlpha,
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
    ]
}
